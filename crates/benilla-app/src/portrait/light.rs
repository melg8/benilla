//! The round portraits' light rig: a fixed light buffer, its material-twin cache, and the values
//! behind it. The character panes light through the UI model tiles' pool instead
//! (`crate::ui_models::PaneLights`). A booth draws with the world's own materials, cloned with only
//! the light storage swapped ([`material_variant`]), so it never inherits the time of day.

use benilla_world::lighting::LightBlob;
use std::collections::HashMap;

use bevy::prelude::*;

use benilla_assets::materials::WowModelMaterial;

/// One booth light: its own copy of the shared-light storage buffer, written once at startup so a
/// booth reads the same at noon, midnight or in fog. `variants` caches each world material's twin,
/// an exact clone with only `light_buf` swapped.
#[derive(Default)]
pub(super) struct BoothRig {
    pub(super) buffer: Option<bevy::render::render_resource::Buffer>,
    variants: HashMap<AssetId<WowModelMaterial>, Handle<WowModelMaterial>>,
    /// The round portraits' twins, on [`VariantLane::RigUnfogged`]: a twin's lane is fixed, so
    /// a source a booth also uses on the world lane needs a twin of each.
    rig_variants: HashMap<AssetId<WowModelMaterial>, Handle<WowModelMaterial>>,
    /// Set by [`Self::variant`] or [`Self::variant_rig`] when the source material was not
    /// resident yet.
    unready: bool,
}

impl BoothRig {
    /// The booth twin of a world-built material, cached per source. When the source is not
    /// resident this returns the world material itself, on the world light buffer: a caller about
    /// to commit a bake must check [`Self::take_unready`] first, since a sleeping booth would keep
    /// that wrongly lit pane until something forces a re-bake.
    pub(super) fn variant(
        &mut self,
        world: &Handle<WowModelMaterial>,
        materials: &mut Assets<WowModelMaterial>,
    ) -> Handle<WowModelMaterial> {
        let Some(buffer) = self.buffer.clone() else {
            // No booth buffer (headless tests): nothing to wait for, so a plain fallback.
            if super::booth_log() {
                eprintln!("[booth] variant NO-BUFFER -> world lane (unlit in a booth)");
            }
            return world.clone();
        };
        match material_variant(
            &mut self.variants,
            &buffer,
            world,
            materials,
            VariantLane::World,
        ) {
            Some(twin) => twin,
            None => {
                self.unready = true;
                world.clone()
            }
        }
    }

    /// Take the "a twin could not be built" flag since the last call. When set, the bake site
    /// abandons this frame's bake without touching `Booth::baked`, so it retries once the
    /// material lands.
    pub(super) fn take_unready(&mut self) -> bool {
        std::mem::take(&mut self.unready)
    }

    /// The round portrait twin of a world material, on the authored-rig lane: its lights are the
    /// booth's [`studio_light`] probe (the reference's `PortraitLighting`, `0x525b10`), not the
    /// world's entity lobe with its 2.5 intensity. Fog off, as the bake scene has none.
    pub(super) fn variant_rig(
        &mut self,
        world: &Handle<WowModelMaterial>,
        materials: &mut Assets<WowModelMaterial>,
    ) -> Handle<WowModelMaterial> {
        let Some(buffer) = self.buffer.clone() else {
            if super::booth_log() {
                eprintln!("[booth] variant NO-BUFFER -> world lane (unlit in a booth)");
            }
            return world.clone();
        };
        match material_variant(
            &mut self.rig_variants,
            &buffer,
            world,
            materials,
            VariantLane::RigUnfogged,
        ) {
            Some(twin) => twin,
            None => {
                self.unready = true;
                world.clone()
            }
        }
    }
}

/// The booths' light, with its own variant cache since a twin points at one buffer.
/// [`Self::studio`] lights the round unit-frame portraits with the reference's own fixed rig
/// (`0x525b10`): a 0.45 grey ambient and a white directional, no fog, no world time of day.
#[derive(Resource, Default)]
pub(super) struct BoothLight {
    pub(super) studio: BoothRig,
}

/// Reap the booth twins whose world source died (`AssetEvent::Removed`, e.g. a map teardown); a
/// twin is pinned only by this cache, so it would otherwise outlive the teardown.
pub(super) fn reap_dead_variants(
    mut events: MessageReader<AssetEvent<WowModelMaterial>>,
    mut booths: ResMut<BoothLight>,
) {
    for ev in events.read() {
        if let AssetEvent::Removed { id } = ev {
            booths.studio.variants.remove(id);
            booths.studio.rig_variants.remove(id);
        }
    }
}

/// Which shade lane and fog policy a [`material_variant`] twin draws with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum VariantLane {
    /// The world's shade lane and fog policy, under a frozen light: the portrait booths.
    World,
    /// The M2 light rig ([`benilla_world::model_render::ShadeSel::Rig`]) with fog forced off: the
    /// glue character model takes no fog (its fill callback `0x470ce0` stages none), nor does a
    /// `<Model>` pane that never armed any.
    RigUnfogged,
    /// The rig lane with the authored fog policy: a `<Model>` pane whose Lua armed fog, and the
    /// glue background scene. Unfogged materials (bit `0x02`) still draw unfogged (`0x70bb24`).
    RigFogged,
}

/// The twin of a world-built material against `buffer`, cached in `variants`: only the light
/// storage swapped, plus what `lane` asks for. `None` means the source is not resident yet; the
/// world material is no substitute (wrong light buffer), so callers retry.
pub(crate) fn material_variant(
    variants: &mut HashMap<AssetId<WowModelMaterial>, Handle<WowModelMaterial>>,
    buffer: &bevy::render::render_resource::Buffer,
    world: &Handle<WowModelMaterial>,
    materials: &mut Assets<WowModelMaterial>,
    lane: VariantLane,
) -> Option<Handle<WowModelMaterial>> {
    if let Some(twin) = variants.get(&world.id()) {
        return Some(twin.clone());
    }
    // A blend/zfill twin can be cloned before the world binds it: realize the parked value first.
    benilla_world::model_render::lazy::realize(materials, world.id());
    let mat = materials.get(world)?;
    let mut twin = mat.clone();
    twin.extension.light_buf = buffer.clone();
    if lane != VariantLane::World {
        twin.extension.sun_scale.x = benilla_world::model_render::ShadeSel::Rig.selector();
    }
    if lane == VariantLane::RigUnfogged {
        // Fog off, keeping every pipeline marker `specialize` keys on (bits 0-3 and the multiply
        // markers, bits 7-8); the mask belongs to `model_render`, next to the packer.
        twin.extension.clutter_fade.z = benilla_world::model_render::replace_fog_policy(
            twin.extension.clutter_fade.z,
            benilla_formats::FogPolicy::Off,
        );
    }
    let handle = materials.add(twin);
    variants.insert(world.id(), handle.clone());
    Some(handle)
}

/// The round portraits' light, the reference's `PortraitLighting` (`0x525b10`): a 0.45 grey
/// ambient and a white directional whose WoW-space travel is `(-1, 0, -1)`, on the `Model2.bls`
/// SH lane the booth's rig materials read. Point lights and fog off; the bake scene has none.
pub(super) fn studio_light() -> LightBlob {
    const AMBIENT: [f32; 3] = [0.45; 3];
    const DIFFUSE: [f32; 3] = [1.0; 3];
    // `0x525b1f` stores 0.45 three times for the ambient; `0x525b43`/`0x525b51`/`0x525b58` store
    // the direction `(-1, 0, -1)` and the colour `(1, 1, 1)` in WoW axes. The travel direction maps
    // through `wow_to_bevy` (X north, Y west, Z up) to the booth's space.
    let sun_dir = benilla_assets::coords::wow_to_bevy([-1.0, 0.0, -1.0]);
    LightBlob::model(AMBIENT, DIFFUSE, sun_dir)
        // The rig lane reads probe slot 0: the same two lights, ambient in the DC term and the
        // directional as one toward-light lobe.
        .probe(AMBIENT, &[(-sun_dir, DIFFUSE)])
        // Fog and the SIDN night lane stay at the builder's off-world defaults; no shader reads
        // 19.w (the booths' exterior dial is 0.4).
        .dial(0.4)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The studio rig is the reference's (`0x525b10`): 0.45 grey ambient and a white directional
    /// travelling `(-1, 0, -1)` in WoW axes, so its toward-light is up and from the camera side.
    #[test]
    fn studio_light_is_the_portrait_rig() {
        let blob = studio_light();
        let rows = blob.header_rows();
        assert_eq!(rows[0][..3], [0.45; 3], "ambient");
        assert_eq!(rows[1][..3], [1.0; 3], "diffuse");
        // The header stores the travel direction as given; the shader normalizes it.
        let to_light = (-Vec3::from_slice(&rows[2][..3])).normalize();
        let want = Vec3::new(0.0, 1.0, -1.0).normalize();
        assert!(
            (to_light - want).length() < 1e-5,
            "the key is up and toward the camera: {to_light:?}"
        );
        // The rig also folds the same two lights into probe slot 0, which the rig lane reads: the
        // DC is the 0.45 ambient plus the lobe's constant `(4/17)·(0.375 + 0.9375·(uₓ² + u_y²))`,
        // `u = (0, 1/√2, −1/√2)`.
        let want_dc = 0.45 + (4.0 / 17.0) * (0.375 + 0.9375 * 0.5);
        let dc = blob.probe_dc();
        assert!(
            dc.iter().all(|c| (c - want_dc).abs() < 1e-5),
            "the rig lane's probe DC: {dc:?}"
        );
    }
}

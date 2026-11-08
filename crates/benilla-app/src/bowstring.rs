//! The engine-drawn bowstring: bow M2s have no string geometry. The reference's per-frame callback
//! (`0x611ff0`) draws a two-segment line between the `$WTT`/`$WTB` limb-tip markers, its middle
//! vertex at the HandArrow attach while the nock latch (`[+0xd58] & 0x4000`) holds, else the tip
//! midpoint. Drawn here as gizmo lines one device pixel wide, the width of the reference's `Gx`
//! line primitive (`GxPrimRender` topology 1). The color is the model's ambient sum (`+0x54` of its
//! lighting record, `0x612044` to `0x612116`): each channel clamped to [0, 1] and packed at 255,
//! alpha 255. Outdoors that is the scene ambient; indoors the owner's `NodeAmbient` word.
//!
//! The tips are posed: `$BWP` plays BowPull (160) on the prop and `$BWR` returns it to Stand (0),
//! so the markers ride limb bones that bend through the draw. An event's position is the model
//! point at bind (its tip bone's pivot on every shipped bow), so the posed path takes the offset
//! from that pivot — [`benilla_world::rig_anim::RigPose::posed_point`] applies the bone's full
//! matrix — and a prop without a pose draws the bind point itself.

use bevy::prelude::*;

use benilla_assets::StringAnchor;

use crate::creature_anim::NockLatch;
use crate::entities::BoneAttach;

/// The HandArrow attach id (35, `0x6121b8`): the string's middle point while nocked.
const HAND_ARROW: u16 = 0x23;

/// Marks a bow prop root whose model authors the `$WTT`/`$WTB` anchors; despawned with the prop.
#[derive(Component)]
pub(crate) struct Bowstring {
    /// The unit wearing the bow, which carries the HandArrow attach and the nock latch.
    pub(crate) owner: Entity,
    /// The `$WTT` top and `$WTB` bottom anchors, each on its limb-tip bone.
    pub(crate) top: StringAnchor,
    pub(crate) bottom: StringAnchor,
}

/// Draws every visible bow's string, tip to middle to tip, from this frame's joint frames.
fn draw_bowstrings(
    bows: Query<(
        &Bowstring,
        &GlobalTransform,
        &InheritedVisibility,
        Option<&benilla_world::rig_anim::RigPose>,
    )>,
    owners: Query<(
        &BoneAttach,
        &benilla_world::rig_anim::RigPose,
        Has<NockLatch>,
    )>,
    joints: Query<&GlobalTransform>,
    lighting: Option<Res<benilla_world::lighting::WowLighting>>,
    // The owner's committed ambient word, present only while its light node is indoors.
    indoors: Query<&benilla_world::interior::NodeAmbient>,
    mut gizmos: Gizmos,
) {
    // The exterior ambient, a unit's own outdoors (`0x69e4ad`); before the first lighting
    // resolve, white keeps the string's authored colour.
    let scene = lighting.as_deref().map_or([1.0; 3], |l| l.ambient);
    for (bs, prop, vis, flex) in &bows {
        if !vis.get() {
            continue;
        }
        // The prop's own pose when it flexes (`joints_root` is this entity); its rigid frame
        // otherwise, where the bind point is the anchor itself.
        let tip = |a: StringAnchor| {
            flex.and_then(|p| p.posed_point(prop, a.bone, a.offset))
                .unwrap_or_else(|| prop.transform_point(a.position))
        };
        let top = tip(bs.top);
        let bottom = tip(bs.bottom);
        let middle = owners
            .get(bs.owner)
            .ok()
            .filter(|(_, _, latched)| *latched)
            .and_then(|(bones, pose, _)| {
                let &(bone, offset) = bones.points.get(&HAND_ARROW)?;
                pose.posed_point(joints.get(pose.joints_root).ok()?, bone, offset)
            })
            .unwrap_or_else(|| (top + bottom) / 2.0);
        // `0x612044`–`0x612116`: the lighting record's ambient sum (`+0x54`), each channel
        // clamped to [0, 1] and packed at 255 with alpha 255. Indoors it is the node's own ramped
        // ambient; the bow inherits its wearer's record (`[item+0x3b8] = [wearer+0x3b8]`).
        let ambient = indoors
            .get(bs.owner)
            .map_or(scene, |a| a.0)
            .map(|c| c.clamp(0.0, 1.0));
        let color = Color::srgb(ambient[0], ambient[1], ambient[2]);
        gizmos.line(top, middle, color);
        gizmos.line(middle, bottom, color);
    }
}

/// Registers the string drawer after the palette pass, beside the other joint-frame readers.
pub(crate) struct BowstringPlugin;

impl Plugin for BowstringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            draw_bowstrings.in_set(benilla_world::billboard::BillboardPlace),
        );
    }
}

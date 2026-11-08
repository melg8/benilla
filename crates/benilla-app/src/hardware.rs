//! The hardware step of start-up: `DetectHardware` (`0x641260`) matches the running adapter against
//! `VideoHardware.dbc`, and when detection applies the client writes the matched row's quality
//! block into sixteen video CVars (`0x639a60`) and sets `hwDetect` to "0". A saved `hwDetect "0"`
//! means the CVars keep what the callbacks stored, so it pins the install to its own values; that
//! is the reference's steady state and what its own `Config.wtf` holds.
//!
//! Deviation: benilla runs the quality half only, because it has no exclusive mode-set to restart
//! (its `gx*` rows describe a window it manages itself, and `gxResolution` is already the windowed
//! size). The other half of the step (`0x639c40`) writes the row's default device-format record into
//! twelve `gx*` CVars and restarts the device, so those rows keep their registered defaults.
//!
//! Deviation: a nonzero `hwDetect` is the only trigger, because benilla has no `-hwdetect` switch
//! and no registry record of the last run; the `Hardware changed.  Reload default settings?` box
//! (`0x641670`) is not modelled, so detection always runs on a fresh install.

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;

use benilla_assets::{AssetSet, LockRecover, WorldAssets};
use benilla_formats::QualityBlock;

use crate::cvars::{CvarHardware, Cvars};

pub(crate) struct HardwarePlugin;

impl Plugin for HardwarePlugin {
    fn build(&self, app: &mut App) {
        // `CvarHardware` already follows `CvarLoad`, whose file read decides `hwDetect`; the chain
        // must also be open, which the table read needs.
        app.add_systems(
            Startup,
            detect_hardware.in_set(CvarHardware).after(AssetSet::Open),
        );
    }
}

/// The adapter's class, as `0x58bc90` infers it from the device caps: 2 with hardware transform and
/// lighting, more than two simultaneous textures and a pixel shader of 1.1 or more; 1 with the
/// transform and two textures; else 0. Every wgpu device but a software rasterizer meets the top
/// tier; llvmpipe and its kind meet none of it.
fn adapter_class(device_type: wgpu::DeviceType) -> u32 {
    match device_type {
        wgpu::DeviceType::Cpu => 0,
        wgpu::DeviceType::IntegratedGpu
        | wgpu::DeviceType::DiscreteGpu
        | wgpu::DeviceType::VirtualGpu
        | wgpu::DeviceType::Other => 2,
    }
}

/// `0x6414c0`: processor class 1 when the frequency is above 1500 MHz. `OsGetCpuFrequency`
/// (`0x42c060`) reads the power call, which fails on a host with more logical CPUs than its four
/// records, and then the calibrated counter rate, above 1500 MHz on any current part — the fact
/// `SoundBufferSize` ships "50" on (`cvars::table`).
const FAST_CPU: bool = true;

/// `0x639a60`: the block's values into the sixteen quality CVars. Integers go through the
/// reference's integer setter (`0x639bc0`) and floats through its float setter (`0x639b80`), whose
/// `%f` text the reference's own `Config.wtf` shows ("1.000000", "300.000000", "0.070000").
/// A row a `$WOW_…` lever owns for the session is left at the lever's value, as `load_config`
/// leaves it; detection is not a reason to override an A/B.
fn apply_quality(cvars: &mut Cvars, block: &QualityBlock) {
    let flags = [
        ("farclip", block.farclip),
        ("lodDist", block.lod_dist),
        ("shadowLevel", block.shadow_level),
        ("frillDensity", block.frill_density),
        ("fullAlpha", block.full_alpha),
        ("doodadAnim", block.doodad_anim),
        ("trilinear", block.trilinear),
        ("MaxLights", block.max_lights),
        // One column feeds both rows, and each takes whether it is nonzero.
        ("pixelShaders", u32::from(block.pixel_shaders != 0)),
        ("specular", u32::from(block.pixel_shaders != 0)),
        ("waterLOD", block.water_lod),
        ("DistCull", block.dist_cull),
        ("baseMip", block.base_mip),
    ];
    for (name, value) in flags {
        if !cvars.is_session_owned(name) {
            cvars.set(name, &value.to_string());
        }
    }
    for (name, value) in [
        ("particleDensity", block.particle_density),
        ("unitDrawDist", block.unit_draw_dist),
        ("SmallCull", block.small_cull),
    ] {
        if !cvars.is_session_owned(name) {
            cvars.set(name, &format!("{value:.6}"));
        }
    }
}

/// Startup: the detection step. It reads `hwDetect` after the config file, matches the adapter,
/// writes the quality block and sets `hwDetect` to "0", then runs the change callbacks the writes
/// owe, as `load_config` does.
fn detect_hardware(world: &mut World) {
    let Some(adapter) = world.get_resource::<RenderAdapterInfo>() else {
        // The reference matches only when the adapter answers its ids (`0x6414ee`); with no
        // adapter there is nothing to match and the CVars keep their registered defaults.
        return;
    };
    let (vendor, device, device_type) = (adapter.vendor, adapter.device, adapter.device_type);
    let Some(detect) = world.resource::<Cvars>().flag("hwDetect") else {
        return;
    };
    if !detect {
        return;
    }
    let class = adapter_class(device_type);
    let Some(chain) = world
        .get_resource::<WorldAssets>()
        .map(|assets| assets.chain.clone())
    else {
        warn!("hardware: no patch chain open — hwDetect left at 1");
        return;
    };
    let loaded = {
        let mut chain = chain.lock_recover();
        benilla_formats::load_video_hardware_catalog(&mut chain)
    };
    let catalog = match loaded {
        Ok(catalog) => catalog,
        Err(e) => {
            warn!("hardware: VideoHardware.dbc failed — hwDetect left at 1: {e:#}");
            return;
        }
    };
    let Some(row) = catalog.matched(vendor, device, class) else {
        warn!("hardware: no VideoHardware.dbc row matches {vendor:#x}:{device:#x} (class {class})");
        return;
    };
    let block = row.quality(FAST_CPU);
    info!(
        "hardware: DetectHardware(): videoID {} vendor {vendor:#x} device {device:#x} class \
         {class} cpu {} — pixelShaders {}",
        row.id,
        u8::from(FAST_CPU),
        block.pixel_shaders
    );
    let events = {
        let mut cvars = world.resource_mut::<Cvars>();
        apply_quality(&mut cvars, &block);
        // `0x63a2a5`: detection applies, and the record is saved with it off.
        cvars.set("hwDetect", "0");
        cvars.take_events()
    };
    for event in events {
        world.trigger(event);
    }
}

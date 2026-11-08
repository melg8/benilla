//! `VideoHardware.dbc`, the table of known video hardware the start-up detection matches against.
//! 193 rows of 22 columns (88 B) in 5875: `ID`, the vendor and device ids the match compares, the
//! quality columns, the default device-format index, and the two override strings. Row 0 is never
//! matched; the fallback rows carry vendor `0xffff` and an adapter class (2, 1 or 0) in the device
//! column, so a GPU the table does not list still lands on one of them.

use std::collections::HashMap;

use anyhow::{Context, Result};
use benilla_dbc::{FieldType, Schema, SchemaField};

use crate::dbc::{parse, u32_at};
use crate::Chain;

const VIDEO_HARDWARE: &str = "DBFilesClient\\VideoHardware.dbc";

/// The farclip column's table, `0x8656c4` (`0x641830`).
const FARCLIP: [u32; 8] = [200, 250, 300, 350, 350, 400, 450, 500];
/// The lodDist column's table, `0x865610` (`0x641846`).
const LOD_DIST: [u32; 4] = [80, 80, 100, 100];
/// The frillDensity column's table, `0x865620` (`0x64185f`).
const FRILL: [u32; 4] = [8, 12, 16, 24];
/// The doodadAnim column's table, `0x865630` (`0x641877`).
const DOODAD: [u32; 4] = [0, 0, 0, 1];
/// The waterLOD column's table, `0x865634` (`0x6418b6`).
const WATER: [u32; 4] = [0, 0, 0, 1];
/// The particleDensity column's table, `0x865644` (`0x6418cf`).
const PARTICLE: [f32; 8] = [0.3, 0.4, 0.5, 0.6, 0.9, 1.0, 1.0, 1.0];
/// The unitDrawDist column's table, `0x865664` (`0x6418e8`).
const UNIT_DRAW: [u32; 8] = [35, 50, 50, 75, 75, 300, 300, 300];
/// The SmallCull column's table, `0x865684` (`0x641901`).
const SMALL_CULL: [f32; 8] = [0.08, 0.08, 0.07, 0.07, 0.07, 0.07, 0.04, 0.04];
/// The DistCull column's table, `0x8656a4` (`0x64191a`).
const DIST_CULL: [u32; 8] = [350, 350, 400, 400, 450, 450, 500, 500];

/// One `VideoHardware.dbc` row, its quality columns raw: the tables above resolve them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoHardware {
    pub id: u32,
    /// The match's first word; `0xffff` on the fallback rows.
    pub vendor: u32,
    /// The match's second word: a device id, or the adapter class on a fallback row.
    pub device: u32,
    /// `+0x0c`, through [`FARCLIP`].
    pub farclip: u32,
    /// `+0x10`, through [`LOD_DIST`].
    pub lod_dist: u32,
    /// `+0x14`, the value itself.
    pub shadow_level: u32,
    /// `+0x18`, through [`FRILL`].
    pub frill_density: u32,
    /// `+0x1c`, the value itself.
    pub full_alpha: u32,
    /// `+0x20`, through [`DOODAD`].
    pub doodad_anim: u32,
    /// `+0x24`, nonzero is trilinear filtering on.
    pub trilinear: u32,
    /// `+0x28`, the value itself.
    pub max_lights: u32,
    /// `+0x2c`, nonzero is `pixelShaders` and `specular` on.
    pub pixel_shaders: u32,
    /// `+0x30`, through [`WATER`].
    pub water_lod: u32,
    /// `+0x34`, through [`PARTICLE`].
    pub particle_density: u32,
    /// `+0x38`, through [`UNIT_DRAW`].
    pub unit_draw_dist: u32,
    /// `+0x3c`, through [`SMALL_CULL`] and [`DIST_CULL`].
    pub small_cull: u32,
    /// `+0x40`, the index into the seven start-up device formats.
    pub format_index: u32,
    /// `+0x44`, the value itself.
    pub base_mip: u32,
    /// `+0x50`, the default of `gxFixLag`.
    pub fix_lag: u32,
    /// `+0x54`, the default of `gxMultisample`.
    pub multisample: u32,
}

/// The sixteen quality CVars `0x639a60` applies, resolved off one row and the processor-class bias
/// row (`0x641820` fills the block at `0xc4e678`; `0x639a60` writes the CVars).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityBlock {
    pub farclip: u32,
    pub lod_dist: u32,
    pub shadow_level: u32,
    pub frill_density: u32,
    pub full_alpha: u32,
    pub doodad_anim: u32,
    pub trilinear: u32,
    pub max_lights: u32,
    pub pixel_shaders: u32,
    pub water_lod: u32,
    pub particle_density: f32,
    pub unit_draw_dist: f32,
    pub small_cull: f32,
    pub dist_cull: u32,
    pub base_mip: u32,
}

impl VideoHardware {
    /// The quality block this row gives, on a fast (`bias = 1`) or slow (`bias = 0`) processor.
    /// The five two-column tables index `2 · column + bias`; the rest read the column itself.
    pub fn quality(&self, fast_cpu: bool) -> QualityBlock {
        let bias = usize::from(fast_cpu);
        let at = |table: &[u32], column: u32| {
            table.get(2 * column as usize + bias).copied().unwrap_or(0)
        };
        let atf = |table: &[f32], column: u32| {
            table
                .get(2 * column as usize + bias)
                .copied()
                .unwrap_or(0.0)
        };
        QualityBlock {
            farclip: at(&FARCLIP, self.farclip),
            lod_dist: LOD_DIST.get(self.lod_dist as usize).copied().unwrap_or(0),
            shadow_level: self.shadow_level,
            frill_density: FRILL.get(self.frill_density as usize).copied().unwrap_or(0),
            full_alpha: self.full_alpha,
            doodad_anim: at(&DOODAD, self.doodad_anim),
            trilinear: self.trilinear,
            max_lights: self.max_lights,
            pixel_shaders: self.pixel_shaders,
            water_lod: at(&WATER, self.water_lod),
            particle_density: atf(&PARTICLE, self.particle_density),
            unit_draw_dist: at(&UNIT_DRAW, self.unit_draw_dist) as f32,
            small_cull: atf(&SMALL_CULL, self.small_cull),
            dist_cull: at(&DIST_CULL, self.small_cull),
            base_mip: self.base_mip,
        }
    }
}

/// Every `VideoHardware.dbc` row in file order, plus an id index.
pub struct VideoHardwareCatalog {
    rows: Vec<(u32, VideoHardware)>,
    by_id: HashMap<u32, usize>,
}

impl VideoHardwareCatalog {
    /// A catalog over rows given directly, in record order, for tests that need a table of known
    /// shape.
    pub fn from_rows(rows: Vec<(u32, VideoHardware)>) -> Self {
        let by_id = rows
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        Self { rows, by_id }
    }

    pub fn get(&self, id: u32) -> Option<&VideoHardware> {
        self.by_id.get(&id).map(|&i| &self.rows[i].1)
    }

    /// Every row in file order.
    pub fn rows(&self) -> impl Iterator<Item = (u32, &VideoHardware)> {
        self.rows.iter().map(|(id, row)| (*id, row))
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// `0x641610`: the first row past index 0 whose vendor and device words match, else the first
    /// fallback row whose vendor is `0xffff` and whose device is the adapter `class` (2, 1 or 0).
    pub fn matched(&self, vendor: u32, device: u32, class: u32) -> Option<&VideoHardware> {
        let scan = |vendor: u32, device: u32| {
            self.rows
                .iter()
                .skip(1)
                .find(|(_, row)| row.vendor == vendor && row.device == device)
                .map(|(_, row)| row)
        };
        scan(vendor, device).or_else(|| scan(0xffff, class))
    }
}

fn schema() -> Schema {
    let mut s = Schema::new("VideoHardware");
    for name in [
        "ID",
        "VendorID",
        "DeviceID",
        "Farclip",
        "LodDist",
        "ShadowLevel",
        "FrillDensity",
        "FullAlpha",
        "DoodadAnim",
        "Trilinear",
        "MaxLights",
        "PixelShaders",
        "WaterLOD",
        "ParticleDensity",
        "UnitDrawDist",
        "SmallCull",
        "Format",
        "BaseMip",
    ] {
        s.add_field(SchemaField::new(name, FieldType::UInt32));
    }
    // The two override texts: OpenGL `+0x48`, Direct3D `+0x4c`.
    s.add_field(SchemaField::new("OpenGLOverride", FieldType::String));
    s.add_field(SchemaField::new("Direct3DOverride", FieldType::String));
    for name in ["FixLag", "Multisample"] {
        s.add_field(SchemaField::new(name, FieldType::UInt32));
    }
    s
}

/// Read `VideoHardware.dbc` off the patch chain into a [`VideoHardwareCatalog`].
pub fn load_video_hardware_catalog(chain: &mut Chain) -> Result<VideoHardwareCatalog> {
    let bytes = chain
        .read_file(VIDEO_HARDWARE)
        .context("reading VideoHardware.dbc")?;
    let rs = parse(&bytes, schema(), "VideoHardware")?;
    let mut rows = Vec::with_capacity(rs.records().len());
    let mut by_id = HashMap::with_capacity(rs.records().len());
    for r in rs.records() {
        let Some(id) = u32_at(r, 0) else { continue };
        let word = |i| u32_at(r, i).unwrap_or(0);
        let row = VideoHardware {
            id,
            vendor: word(1),
            device: word(2),
            farclip: word(3),
            lod_dist: word(4),
            shadow_level: word(5),
            frill_density: word(6),
            full_alpha: word(7),
            doodad_anim: word(8),
            trilinear: word(9),
            max_lights: word(10),
            pixel_shaders: word(11),
            water_lod: word(12),
            particle_density: word(13),
            unit_draw_dist: word(14),
            small_cull: word(15),
            format_index: word(16),
            base_mip: word(17),
            fix_lag: word(20),
            multisample: word(21),
        };
        by_id.insert(id, rows.len());
        rows.push((id, row));
    }
    Ok(VideoHardwareCatalog { rows, by_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 5875 layout and the fallback row a modern GPU lands on.
    #[test]
    fn real_video_hardware_layout_sanity() {
        let data = crate::wow_data_or_skip!();
        let mut chain = crate::open_chain(&data).expect("open chain");
        let cat = load_video_hardware_catalog(&mut chain).expect("load VideoHardware");
        assert_eq!(cat.len(), 193, "all 193 rows load");

        // An unlisted adapter takes the class-2 fallback row, `videoID 170` in the reference's
        // own `Logs/gx.log`.
        let row = cat.matched(0x10de, 0x1234, 2).expect("class-2 fallback");
        assert_eq!(row.id, 170);
        assert_eq!((row.vendor, row.device), (0xffff, 2));

        // A fast processor's block on row 170: the values the reference's own install runs at.
        let fast = row.quality(true);
        assert_eq!(fast.farclip, 500);
        assert_eq!(fast.lod_dist, 100);
        assert_eq!(fast.shadow_level, 1);
        assert_eq!(fast.frill_density, 24);
        assert_eq!(fast.full_alpha, 1);
        assert_eq!(fast.doodad_anim, 1);
        assert_eq!(fast.trilinear, 1);
        assert_eq!(fast.max_lights, 4);
        assert_eq!(fast.pixel_shaders, 1);
        assert_eq!(fast.water_lod, 1);
        assert_eq!(fast.particle_density, 1.0);
        assert_eq!(fast.unit_draw_dist, 300.0);
        assert_eq!(fast.small_cull, 0.04);
        assert_eq!(fast.dist_cull, 500);
        assert_eq!(fast.base_mip, 0);
        // A slow processor reads the even entries: a shorter far clip and no doodad animation.
        let slow = row.quality(false);
        assert_eq!(slow.farclip, 450);
        assert_eq!(slow.doodad_anim, 0);
        assert_eq!(slow.water_lod, 0);
        assert_eq!(slow.small_cull, 0.04);
        assert_eq!(slow.unit_draw_dist, 300.0);
    }

    /// `0x641610` skips row 0, takes the first exact hit, and falls back to `0xffff` + class.
    #[test]
    fn the_match_skips_row_zero_and_falls_back_to_the_class() {
        let row = |id, vendor, device| VideoHardware {
            id,
            vendor,
            device,
            farclip: 0,
            lod_dist: 0,
            shadow_level: 0,
            frill_density: 0,
            full_alpha: 0,
            doodad_anim: 0,
            trilinear: 0,
            max_lights: 0,
            pixel_shaders: 0,
            water_lod: 0,
            particle_density: 0,
            unit_draw_dist: 0,
            small_cull: 0,
            format_index: 0,
            base_mip: 0,
            fix_lag: 0,
            multisample: 0,
        };
        let cat = VideoHardwareCatalog::from_rows(vec![
            (1, row(1, 0x1002, 0x1111)),
            (2, row(2, 0xffff, 2)),
            (3, row(3, 0x10de, 0x2222)),
            (4, row(4, 0x10de, 0x2222)),
        ]);
        assert_eq!(
            cat.matched(0x1002, 0x1111, 2).map(|r| r.id),
            Some(2),
            "row 0 is never examined: the exact pair falls through to the class fallback"
        );
        assert_eq!(cat.matched(0x10de, 0x2222, 2).map(|r| r.id), Some(3));
        assert_eq!(cat.matched(0x10de, 0x9999, 2).map(|r| r.id), Some(2));
        assert_eq!(cat.matched(0x10de, 0x9999, 1).map(|r| r.id), None);
    }
}

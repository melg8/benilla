//! The questgiver `!` marker `Interface\Buttons\TalkToMe.m2` is two batches: an unlit opaque
//! `YellowOrange64` face and a lit additive, depth-write-off `White8x8` halo carrying the authored
//! 0.4 transparency weight and the `M2Color` gold tint. The reference's additive blend
//! (`SRC_ALPHA/ONE`, blend mode 4) is the shader's `ONE/ONE` with the weight folded into the
//! colour; anything else double-dims the halo or lifts it off its owner's shade.
//!
//! Measured against the reference's full-strength add: a lit cell bakes
//! `(0.992, 0.863, 0) × 0.4 × 2.5 × 255 ≈ (101, 88, 0)` per channel. Keep those factors intact.

use benilla_formats::{load_m2_mesh, open_chain, ModelBlend};

/// The `!` marker's two batches, and the material facts the halo's brightness depends on.
#[test]
fn the_questgiver_bang_marker_is_an_unlit_face_and_an_additive_gold_halo() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open vanilla patch chain");
    let subs = load_m2_mesh(&mut chain, "Interface\\Buttons\\TalkToMe.m2").expect("load TalkToMe");

    assert_eq!(
        subs.len(),
        2,
        "the marker is a face batch and a halo batch, not a coplanar stack"
    );
    let tex = |s: &benilla_formats::RenderSubmesh| {
        s.texture
            .as_deref()
            .unwrap_or_default()
            .to_ascii_uppercase()
    };

    // Batch 0: the gold `!` itself, material flag UNLIT (0x01) and blend 0 Opaque.
    let face = &subs[0];
    assert_eq!(face.positions.len(), 34, "the face's vertex count");
    assert!(
        tex(face).ends_with("YELLOWORANGE64.BLP"),
        "the face reads the gold `!` texture, got {}",
        tex(face)
    );
    assert!(
        face.emissive,
        "material 0x01 (UNLIT): the face shows its texture colour, unlit"
    );
    assert_eq!(face.blend, ModelBlend::Opaque, "material blend 0");
    assert!(!face.additive, "not an additive batch");
    assert!(!face.no_depth_write, "the face writes depth");
    assert!(
        face.vertex_colors.is_empty(),
        "the face's colorIndex is 65535 — no M2Color tint"
    );
    assert!(
        face.alpha_anim.is_none(),
        "the face's color alpha and weight are constant 1"
    );

    // Batch 1: the halo, material flag 0x10 NO_DEPTH_WRITE and blend 4 AddAlpha.
    let halo = &subs[1];
    assert_eq!(halo.positions.len(), 80, "the halo's vertex count");
    assert!(
        tex(halo).ends_with("WHITE8X8.BLP"),
        "the halo is a white multiplicative card, got {}",
        tex(halo)
    );
    assert!(
        !halo.emissive,
        "the halo is lit: it takes its owner's sun × 2.5 / shadow × 0.5"
    );
    assert_eq!(halo.blend, ModelBlend::Blend, "material blend 4");
    assert!(
        halo.additive,
        "blend mode 4 is the additive lane (reference `SRC_ALPHA/ONE`)"
    );
    assert!(
        halo.no_depth_write,
        "render flag 0x10: the halo never occludes the face behind it"
    );

    // The authored 0.4 transparency weight, which the reference multiplies into the instance
    // alpha (`0x707680`). Constant, so it bakes as a one-key loop.
    let alpha = halo
        .alpha_anim
        .as_ref()
        .expect("the halo's transparency weight is not 1, so it must ride the alpha anim");
    for t in [0.0f32, 0.7, 12.5] {
        let a = alpha.sample(None, t, 0.0);
        assert!(
            (a - 0.4).abs() < 1e-3,
            "the halo holds its authored 0.4 weight at {t}s, got {a}"
        );
    }

    // The M2Color tint baked into the vertex colours: index 0 of the color table, gold.
    let tint = halo
        .vertex_colors
        .first()
        .copied()
        .expect("the halo carries its M2Color tint as a vertex colour");
    assert!(
        halo.vertex_colors.iter().all(|c| *c == tint),
        "one constant tint across the halo"
    );
    for ((got, want), ch) in tint.iter().zip([0.992, 0.863, 0.0]).zip("rgb".chars()) {
        assert!(
            (got - want).abs() < 1e-3,
            "halo tint {ch} is the reference gold {want}, got {got}"
        );
    }
    assert_eq!(tint[3], 1.0, "M2Color authors no alpha; 0.4 is the weight");
}

use freecandy7::{Job, filter, manifest_json, required_overlap};
use serde_json::{Value, json};
fn run(effect: &str, extra: Value) -> Vec<f32> {
    let mut px = vec![0.0; 192 * 160 * 4];
    for y in 40..120 {
        for x in 48..144 {
            if !(75..117).contains(&x) || !(65..95).contains(&y) {
                px[(y * 192 + x) * 4..][..4].copy_from_slice(&[0.3, 0.5, 0.7, 1.0]);
            }
        }
    }
    let mut p = json!({"effect":effect});
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    let bytes = serde_json::to_vec(&p).unwrap();
    let mut j = Job {
        px: &mut px,
        width: 192,
        height: 160,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: &bytes,
    };
    assert_eq!(filter(&mut j), 0);
    assert!(px.iter().all(|v| v.is_finite()));
    px
}
#[test]
fn registered_and_cardboard_is_a_paper_cut_preset() {
    let m: Value = serde_json::from_str(&manifest_json()).unwrap();
    for e in ["holographic_foil", "cracked_paint", "paper_cut"] {
        assert!(
            m["params"]["effect"]["options"]
                .as_array()
                .unwrap()
                .contains(&json!(e))
        );
        assert!(
            m["selectionShape"]["values"]
                .as_array()
                .unwrap()
                .contains(&json!(e))
        );
    }
    assert_eq!(m["presets"]["Layered Cardboard"]["effect"], "paper_cut");
    assert_eq!(
        m["presets"]["Layered Cardboard"]["paper_cut_material"],
        "cardboard"
    );
}
#[test]
fn material_controls_change_pixels_and_preserve_holes() {
    for (e, p) in [
        ("holographic_foil", json!({"holographic_foil_shift":170})),
        ("cracked_paint", json!({"cracked_paint_chipping":85})),
        ("paper_cut", json!({"paper_cut_material":"cardboard"})),
    ] {
        let a = run(e, json!({}));
        let b = run(e, p.clone());
        assert_ne!(a, b, "{e}");
        assert_eq!(b, run(e, p));
        if e != "paper_cut" {
            assert_eq!(
                a[(80 * 192 + 96) * 4 + 3],
                0.0,
                "hole must remain open: {e}"
            );
        }
    }
}
#[test]
fn paper_depth_changes_geometry_and_context() {
    let a = run("paper_cut", json!({"paper_cut_depth":1}));
    let b = run("paper_cut", json!({"paper_cut_depth":30}));
    let count = |p: &[f32]| p.as_chunks::<4>().0.iter().filter(|p| p[3] > 0.5).count();
    assert!(count(&b) > count(&a));
    assert!(required_overlap(br#"{"effect":"paper_cut","paper_cut_depth":500}"#) >= 504);
}
#[test]
fn independently_rendered_bands_join_without_seams() {
    for effect in ["holographic_foil", "cracked_paint", "paper_cut"] {
        let expected = run(effect, json!({}));
        let mut original = vec![0.0; 192 * 160 * 4];
        for y in 40..120 {
            for x in 48..144 {
                if !(75..117).contains(&x) || !(65..95).contains(&y) {
                    original[(y * 192 + x) * 4..][..4].copy_from_slice(&[0.3, 0.5, 0.7, 1.0]);
                }
            }
        }
        for left in [0, 96] {
            let (mut band, w, h) = (vec![0.0; 224 * 288 * 4], 224, 288);
            for y in 0..h {
                for x in 0..w {
                    let sx = (left + x as i32 - 64).clamp(0, 191) as usize;
                    let sy = (y as i32 - 64).clamp(0, 159) as usize;
                    band[(y * w + x) * 4..][..4]
                        .copy_from_slice(&original[(sy * 192 + sx) * 4..][..4]);
                }
            }
            let p = json!({"effect":effect,"_image":{"x":left-64,"y":-64,"canvasWidth":192,"canvasHeight":160,"overlap":64,"outputX":64,"outputY":64,"outputWidth":96,"outputHeight":160}});
            let bytes = serde_json::to_vec(&p).unwrap();
            let mut j = Job {
                px: &mut band,
                width: w,
                height: h,
                channels: 4,
                format: 8 | 256 | (3 << 16),
                json: &bytes,
            };
            assert_eq!(filter(&mut j), 0);
            for y in 0..160 {
                for x in 0..96 {
                    for c in 0..4 {
                        assert!(
                            (band[((y + 64) * w + x + 64) * 4 + c]
                                - expected[(y * 192 + left as usize + x) * 4 + c])
                                .abs()
                                < 0.00001,
                            "{effect}: seam at {x},{y}/{c}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn grayscale_and_empty_masks_are_supported() {
    for effect in ["holographic_foil", "cracked_paint", "paper_cut"] {
        for empty in [false, true] {
            let mut px = vec![0.0; 96 * 96 * 2];
            if !empty {
                for y in 20..70 {
                    for x in 20..70 {
                        px[(y * 96 + x) * 2] = 0.4;
                        px[(y * 96 + x) * 2 + 1] = 1.0;
                    }
                }
            }
            let bytes = serde_json::to_vec(&json!({"effect":effect})).unwrap();
            let mut job = Job {
                px: &mut px,
                width: 96,
                height: 96,
                channels: 2,
                format: 8 | 256 | (1 << 16),
                json: &bytes,
            };
            assert_eq!(filter(&mut job), 0);
            assert!(px.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
            if empty {
                assert!(px.iter().all(|v| *v == 0.0));
            }
        }
    }
}
#[test]
fn maximum_paper_depth_reaches_five_hundred_pixels() {
    let mut px = vec![0.0; 720 * 120 * 4];
    for y in 30..70 {
        for x in 25..65 {
            px[(y * 720 + x) * 4..][..4].copy_from_slice(&[1.0, 1.0, 1.0, 1.0]);
        }
    }
    let params =
        br#"{"effect":"paper_cut","paper_cut_layers":2,"paper_cut_depth":500,"paper_cut_angle":0}"#;
    let mut job = Job {
        px: &mut px,
        width: 720,
        height: 120,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: params,
    };
    assert_eq!(filter(&mut job), 0);
    assert_eq!(
        px[(50 * 720 + 545) * 4 + 3],
        1.0,
        "back sheet moves the full requested 500 pixels"
    );
    assert_eq!(
        px[(50 * 720 + 600) * 4 + 3],
        0.0,
        "render does not extend without limit"
    );
}

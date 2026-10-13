use freecandy7::{Job, filter, manifest_json, required_overlap};
use serde_json::{Value, json};

fn source() -> Vec<f32> {
    let mut px = vec![0.0; 256 * 192 * 4];
    for y in 40..152 {
        for x in 48..208 {
            if !(85..170).contains(&x) || !(75..117).contains(&y) {
                px[(y * 256 + x) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.7, 1.0]);
            }
        }
    }
    px
}

fn render(extra: Value) -> Vec<f32> {
    let mut px = source();
    let mut params = json!({"effect":"neon_tubes", "neon_tubes_glow":8});
    for (k, v) in extra.as_object().unwrap() {
        params[k] = v.clone();
    }
    let bytes = serde_json::to_vec(&params).unwrap();
    let mut job = Job {
        px: &mut px,
        width: 256,
        height: 192,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: &bytes,
    };
    assert_eq!(filter(&mut job), 0);
    assert!(px.iter().all(|x| x.is_finite()));
    px
}

#[test]
fn neon_traces_both_boundaries_and_leaves_the_center_open() {
    let px = render(json!({"neon_tubes_width":8}));
    let alpha = |x: usize, y: usize| px[(y * 256 + x) * 4 + 3];
    assert!(alpha(48, 96) > 0.95, "outside edge has a luminous tube");
    assert!(alpha(85, 96) > 0.95, "hole edge has a luminous tube");
    assert_eq!(alpha(128, 96), 0.0, "hole center stays open");
    assert_eq!(alpha(65, 96), 0.0, "source fill becomes an outline");
    assert_eq!(alpha(20, 20), 0.0, "remote pixels remain transparent");
}

#[test]
fn width_glow_and_breaks_change_actual_geometry() {
    let small = render(json!({"neon_tubes_width":4,"neon_tubes_glow":0}));
    let wide = render(json!({"neon_tubes_width":20,"neon_tubes_glow":0}));
    let count = |p: &[f32]| p.as_chunks::<4>().0.iter().filter(|p| p[3] > 0.8).count();
    assert!(count(&wide) > count(&small) * 2);
    let glow = render(json!({"neon_tubes_width":4,"neon_tubes_glow":30}));
    assert!(glow[(96 * 256 + 32) * 4 + 3] > 0.0);
    assert_eq!(small[(96 * 256 + 32) * 4 + 3], 0.0);
    let broken = render(json!({"neon_tubes_width":20,"neon_tubes_glow":0,"neon_tubes_breaks":70}));
    assert!(count(&broken) < count(&wide) / 2);
    assert_eq!(
        broken,
        render(json!({"neon_tubes_width":20,"neon_tubes_glow":0,"neon_tubes_breaks":70}))
    );
}

#[test]
fn neon_is_registered_scoped_and_requests_its_full_reach() {
    let m: Value = serde_json::from_str(&manifest_json()).unwrap();
    assert!(
        m["params"]["effect"]["options"]
            .as_array()
            .unwrap()
            .contains(&json!("neon_tubes"))
    );
    assert!(
        m["selectionShape"]["values"]
            .as_array()
            .unwrap()
            .contains(&json!("neon_tubes"))
    );
    assert!(
        m["presets"]
            .as_object()
            .unwrap()
            .values()
            .filter(|p| p["effect"] == "neon_tubes")
            .count()
            >= 3
    );
    assert_eq!(
        required_overlap(
            br#"{"effect":"neon_tubes","neon_tubes_width":500,"neon_tubes_glow":1250}"#
        ),
        1504
    );
}

#[test]
fn independent_bands_match_the_whole_image() {
    let original = source();
    let params = json!({"neon_tubes_width":12,"neon_tubes_glow":18,
        "neon_tubes_breaks":25,"neon_tubes_variation":65});
    let expected = render(params.clone());
    for left in [0, 128] {
        let mut band = vec![0.0; 256 * 320 * 4];
        for y in 0..320 {
            for x in 0..256 {
                let sx = (left + x as i32 - 64).clamp(0, 255) as usize;
                let sy = (y as i32 - 64).clamp(0, 191) as usize;
                band[(y * 256 + x) * 4..][..4]
                    .copy_from_slice(&original[(sy * 256 + sx) * 4..][..4]);
            }
        }
        let mut settings = params.clone();
        settings["effect"] = json!("neon_tubes");
        settings["_image"] = json!({"x":left-64,"y":-64,"canvasWidth":256,"canvasHeight":192,
            "overlap":64,"outputX":64,"outputY":64,"outputWidth":128,"outputHeight":192});
        let bytes = serde_json::to_vec(&settings).unwrap();
        let mut job = Job {
            px: &mut band,
            width: 256,
            height: 320,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: &bytes,
        };
        assert_eq!(filter(&mut job), 0);
        for y in 0..192 {
            for x in 0..128 {
                let a = &band[((y + 64) * 256 + x + 64) * 4..][..4];
                let b = &expected[(y * 256 + left as usize + x) * 4..][..4];
                for c in 0..4 {
                    assert!(
                        (a[c] - b[c]).abs() < 0.00001,
                        "band mismatch at {x},{y}/{c}"
                    );
                }
            }
        }
    }
}

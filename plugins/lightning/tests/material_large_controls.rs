use freecandy7::{Job, filter};
use serde_json::{Value, json};

fn apply(width: usize, height: usize, pixels: &mut [f32], mut params: Value) {
    params["_image"] = json!({"x":0,"y":0,"canvasWidth":width,"canvasHeight":height});
    let bytes = serde_json::to_vec(&params).unwrap();
    let mut job = Job {
        px: pixels,
        width,
        height,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: &bytes,
    };
    assert_eq!(filter(&mut job), 0);
    assert!(
        pixels
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    );
}

fn rectangle(
    width: usize,
    height: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> Vec<f32> {
    let mut pixels = vec![0.0; width * height * 4];
    for y in top..bottom {
        for x in left..right {
            pixels[(y * width + x) * 4..][..4].copy_from_slice(&[
                0.2 + 0.5 * x as f32 / width as f32,
                0.4,
                0.7,
                1.0,
            ]);
        }
    }
    pixels
}

#[test]
fn hanging_materials_render_beyond_one_thousand_pixels() {
    let (width, height) = (512, 1600);
    for effect in ["drip", "icicles"] {
        let mut pixels = rectangle(width, height, 0, 20, width, 40);
        apply(
            width,
            height,
            &mut pixels,
            json!({
                "effect": effect,
                format!("{effect}_length"):150,
                format!("{effect}_size"):1000,
                format!("{effect}_size_jitter"):0,
                "drip_amount":100,"icicles_density":100
            }),
        );
        assert!(
            (0..width).any(|x| pixels[(1200 * width + x) * 4 + 3] > 0.05),
            "{effect} must create geometry more than 1000px below its source"
        );
        assert!(
            (1540..height).all(|y| (0..width).all(|x| pixels[(y * width + x) * 4 + 3] == 0.0)),
            "{effect} must remain within its requested 1500px reach"
        );
    }
}

#[test]
fn large_snow_forms_a_cap_hundreds_of_pixels_above_the_subject() {
    let (width, height) = (512, 700);
    let mut pixels = rectangle(width, height, 8, 600, width - 8, 640);
    apply(
        width,
        height,
        &mut pixels,
        json!({"effect":"snow","snow_depth":500,"snow_amount":100}),
    );
    assert!((16..width - 16).any(|x| pixels[(400 * width + x) * 4 + 3] > 0.9));
}

#[test]
fn shadow_distance_moves_the_source_fourteen_hundred_pixels() {
    let (width, height) = (1600, 64);
    let mut pixels = rectangle(width, height, 16, 20, 40, 44);
    apply(
        width,
        height,
        &mut pixels,
        json!({"effect":"shadow","shadow_distance":1400,"shadow_angle":0,"shadow_softness":0,"shadow_opacity":100}),
    );
    assert!(pixels[(32 * width + 1428) * 4 + 3] > 0.99);
    assert_eq!(pixels[(32 * width + 100) * 4 + 3], 0.0);
}

#[test]
fn broad_bevel_changes_the_face_well_beyond_the_old_edge_width() {
    let (width, height) = (704, 704);
    let mut pixels = rectangle(width, height, 16, 16, 688, 688);
    let index = (352 * width + 160) * 4;
    let before = pixels[index..index + 3].to_vec();
    apply(
        width,
        height,
        &mut pixels,
        json!({"effect":"bevel","bevel_size":350,"bevel_depth":100}),
    );
    let difference: f32 = pixels[index..index + 3]
        .iter()
        .zip(before)
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(
        difference > 0.1,
        "350px bevel left the face 144px from its edge unchanged: {difference}"
    );
}

#[test]
fn large_glass_refraction_changes_the_broad_interior() {
    let (width, height) = (704, 704);
    let source = rectangle(width, height, 16, 16, 688, 688);
    let mut small = source.clone();
    let mut large = source;
    apply(
        width,
        height,
        &mut small,
        json!({"effect":"glass","glass_refraction":24}),
    );
    apply(
        width,
        height,
        &mut large,
        json!({"effect":"glass","glass_refraction":500}),
    );
    let difference: f32 = (96..240)
        .flat_map(|x| (300..400).map(move |y| (y * width + x) * 4))
        .map(|i| {
            (small[i] - large[i]).abs()
                + (small[i + 1] - large[i + 1]).abs()
                + (small[i + 2] - large[i + 2]).abs()
        })
        .sum();
    assert!(
        difference > 100.0,
        "large glass refraction remained capped: {difference}"
    );
}

#[test]
fn large_water_drops_have_different_geometry_than_the_old_maximum() {
    let (width, height) = (704, 704);
    let source = rectangle(width, height, 0, 0, width, height);
    let mut small = source.clone();
    let mut large = source;
    apply(
        width,
        height,
        &mut small,
        json!({"effect":"water_drops","water_drops_scale":64,"water_drops_amount":100}),
    );
    apply(
        width,
        height,
        &mut large,
        json!({"effect":"water_drops","water_drops_scale":1500,"water_drops_amount":100}),
    );
    let difference: f32 = small.iter().zip(large).map(|(a, b)| (a - b).abs()).sum();
    assert!(
        difference > 1000.0,
        "1500px water drops remained capped: {difference}"
    );
}

#[test]
fn long_extrusion_keeps_a_single_faint_pixel_connected() {
    let (width, height) = (1600, 64);
    let mut pixels = vec![0.0; width * height * 4];
    pixels[(32 * width + 16) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.8, 0.2]);
    apply(
        width,
        height,
        &mut pixels,
        json!({"effect":"extrude","extrude_length":1500,"extrude_angle":0}),
    );
    assert!(
        pixels[(32 * width + 1400) * 4 + 3] > 0.1,
        "long extrusion skipped its one-pixel translucent source"
    );
}

#[test]
fn long_motion_trail_retains_a_single_faint_pixel() {
    let (width, height) = (1600, 64);
    let mut pixels = vec![0.0; width * height * 4];
    pixels[(32 * width + 1400) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.8, 0.2]);
    apply(
        width,
        height,
        &mut pixels,
        json!({"effect":"motion_trail","motion_trail_length":1500,"motion_trail_angle":0,"motion_trail_opacity":100}),
    );
    assert!(
        pixels[(32 * width + 100) * 4 + 3] > 0.000001,
        "long motion trail skipped its one-pixel translucent source"
    );
}

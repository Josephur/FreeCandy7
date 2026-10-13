use freecandy7::{Job, filter};
use serde_json::json;

fn render(extra: serde_json::Value) -> Vec<f32> {
    let mut params =
        json!({"effect":"animal_fur", "animal_fur_pattern":"leopard", "animal_fur_scale":70});
    for (key, value) in extra.as_object().unwrap() {
        params[key] = value.clone();
    }
    let bytes = serde_json::to_vec(&params).unwrap();
    let mut pixels = vec![1.0; 128 * 96 * 4];
    assert_eq!(
        filter(&mut Job {
            px: &mut pixels,
            width: 128,
            height: 96,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: &bytes
        }),
        0
    );
    pixels
}

#[test]
fn coat_has_independent_local_code_controls() {
    let base = render(json!({}));
    for change in [
        json!({"animal_fur_length":45}),
        json!({"animal_fur_density":25}),
        json!({"animal_fur_flow":90}),
        json!({"animal_fur_lighting":150}),
        json!({"animal_fur_relief":90}),
    ] {
        let other = render(change.clone());
        let delta: f32 = base
            .iter()
            .zip(&other)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / base.len() as f32;
        assert!(delta > 0.001, "dead fur control {change}: {delta}");
        assert!(
            other
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        );
        assert_eq!(other, render(change), "nondeterministic coat");
    }
}

#[test]
fn natural_zebra_retains_neutral_black_and_white_hairs() {
    let p = render(json!({"animal_fur_pattern":"zebra"}));
    let chroma = p
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| (p[0] - p[2]).abs())
        .sum::<f32>()
        / (128 * 96) as f32;
    assert!(chroma < 0.12, "zebra was recolored golden: {chroma}");
}

#[test]
fn adaptive_texture_scale_changes_with_canvas_size() {
    // The same physical crop has smaller features in an adaptive small canvas;
    // fixed mode keeps the existing pixel scale.
    let fixed = render(json!({"animal_fur_scaling":"fixed"}));
    let adaptive = render(json!({"animal_fur_scaling":"adaptive"}));
    let difference = fixed
        .iter()
        .zip(&adaptive)
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / fixed.len() as f32;
    assert!(
        difference > 0.025,
        "adaptive mode does not affect output: {difference}"
    );
}

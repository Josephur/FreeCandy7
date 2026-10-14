use freecandy7::{Job, filter, manifest_json};
use serde_json::{Value, json};

const WIDTH: usize = 384;
const HEIGHT: usize = 144;
const EDGE: usize = 40;

fn render(effect: &str, size: f32, jitter: f32, length: f32) -> Vec<f32> {
    let mut pixels = vec![0.0; WIDTH * HEIGHT * 4];
    for y in 20..=EDGE {
        for x in 16..WIDTH - 16 {
            pixels[(y * WIDTH + x) * 4..][..4].copy_from_slice(&[0.25, 0.5, 0.75, 1.0]);
        }
    }
    let params = json!({
        "effect": effect,
        format!("{effect}_size"): size,
        format!("{effect}_size_jitter"): jitter,
        format!("{effect}_length"): length,
        "drip_amount": 100,
        "icicles_density": 100,
        "_image": {"x":0,"y":0,"canvasWidth":WIDTH,"canvasHeight":HEIGHT}
    });
    let bytes = serde_json::to_vec(&params).unwrap();
    let mut job = Job {
        px: &mut pixels,
        width: WIDTH,
        height: HEIGHT,
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
    pixels
}

fn extent(pixels: &[f32]) -> usize {
    (EDGE + 1..HEIGHT)
        .rev()
        .find(|&y| (0..WIDTH).any(|x| pixels[(y * WIDTH + x) * 4 + 3] > 0.1))
        .unwrap_or(EDGE)
        - EDGE
}

// Measure individual silhouettes below a horizontal source edge. At these sizes,
// clear columns separate the particles, without relying on their procedural keys.
fn particle_lengths(pixels: &[f32]) -> Vec<usize> {
    let mut lengths = Vec::new();
    let mut current = 0;
    for x in 20..WIDTH - 20 {
        let height = (EDGE + 1..HEIGHT)
            .rev()
            // Include the antialiased tips of the translucent ice. A high alpha
            // cutoff measures lighting/subpixel placement instead of its length.
            .find(|&y| pixels[(y * WIDTH + x) * 4 + 3] > 0.01)
            .map_or(0, |y| y - EDGE);
        if height > 0 {
            current = current.max(height);
        } else if current > 0 {
            lengths.push(current);
            current = 0;
        }
    }
    // Exclude the cut-off particles at the measurement boundaries.
    if !lengths.is_empty() {
        lengths.remove(0);
    }
    lengths
}

#[test]
fn size_controls_enlarge_the_rendered_particles() {
    for effect in ["drip", "icicles"] {
        let small = render(effect, 100.0, 0.0, 16.0);
        let large = render(effect, 300.0, 0.0, 16.0);
        assert!(
            extent(&large) > extent(&small) * 2,
            "{effect}: Size must increase length"
        );
        let mean_width = |pixels: &[f32]| {
            let row: Vec<bool> = (20..WIDTH - 20)
                .map(|x| pixels[((EDGE + 3) * WIDTH + x) * 4 + 3] > 0.1)
                .collect();
            let occupied = row.iter().filter(|&&p| p).count();
            let runs = row.windows(2).filter(|p| !p[0] && p[1]).count().max(1);
            occupied as f32 / runs as f32
        };
        assert!(
            mean_width(&large) > mean_width(&small) * 1.7,
            "{effect}: Size must increase width"
        );
    }
}

#[test]
fn zero_jitter_is_uniform_and_full_jitter_varies_individual_lengths() {
    for effect in ["drip", "icicles"] {
        let uniform = particle_lengths(&render(effect, 100.0, 0.0, 40.0));
        let varied = particle_lengths(&render(effect, 100.0, 100.0, 40.0));
        assert!(
            uniform.len() >= 8 && varied.len() >= 8,
            "{effect}: insufficient isolated particles"
        );
        let spread = |values: &[usize]| values.iter().max().unwrap() - values.iter().min().unwrap();
        assert!(
            spread(&uniform) <= 2,
            "{effect}: zero jitter still varies lengths: {uniform:?}"
        );
        assert!(
            spread(&varied) >= 10,
            "{effect}: jitter must vary lengths: {varied:?}"
        );
    }
}

#[test]
fn sixty_pixel_particles_remain_deterministic_and_within_the_legacy_halo() {
    for effect in ["drip", "icicles"] {
        for jitter in [0.0, 50.0, 100.0] {
            let output = render(effect, 100.0, jitter, 60.0);
            assert_eq!(output, render(effect, 100.0, jitter, 60.0));
            assert!(extent(&output) <= 60, "{effect}: exceeded supported reach");
            assert!(
                extent(&output) >= 45,
                "{effect}: maximum size should reach beyond the old length limit"
            );
        }
    }
}

#[test]
fn manifest_exposes_effect_scoped_size_and_jitter_controls() {
    let manifest: Value = serde_json::from_str(&manifest_json()).unwrap();
    for effect in ["drip", "icicles"] {
        for suffix in ["size", "size_jitter"] {
            let key = format!("{effect}_{suffix}");
            let spec = &manifest["params"][&key];
            assert_eq!(spec["type"], "number", "missing {key}");
            assert_eq!(spec["visibleWhen"]["effect"], effect);
            assert!(spec["min"].as_f64().unwrap() < spec["max"].as_f64().unwrap());
        }
    }
}

// A rounded liquid tip retains a broad cross-section one pixel above its
// bottom. The old second taper collapsed that cross-section into a point.
#[test]
fn drip_bulbs_have_rounded_bottoms_instead_of_double_tapered_points() {
    let pixels = render("drip", 500.0, 0.0, 18.0);
    let coverage = |y: usize| -> f32 {
        (25..WIDTH - 25)
            .map(|x| pixels[(y * WIDTH + x) * 4 + 3])
            .sum()
    };
    let bulb_width = (EDGE + 70..EDGE + 89).map(coverage).fold(0.0_f32, f32::max);
    let near_bottom = coverage(EDGE + 89);
    assert!(bulb_width > 10.0, "fixture needs visible bulbs");
    assert!(
        near_bottom / bulb_width > 0.32,
        "tip collapsed into a point: near-bottom coverage {near_bottom}, bulb coverage {bulb_width}"
    );
}

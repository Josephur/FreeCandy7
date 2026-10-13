//! Public context negotiation, ABI rejection, and the rendered source-context contract.
use freecandy7::{Job, filter, manifest_json, pc_filter, pc_overlap, required_overlap};
use serde_json::{Value, json};

fn overlap(params: &Value) -> u32 {
    required_overlap(&serde_json::to_vec(params).unwrap())
}

#[test]
fn default_and_inactive_effect_settings_keep_the_established_context() {
    let manifest: Value = serde_json::from_str(&manifest_json()).unwrap();
    let specs = manifest["params"].as_object().unwrap();
    for effect in specs["effect"]["options"].as_array().unwrap() {
        let mut params = json!({"effect":effect});
        for (key, spec) in specs {
            if key != "effect" {
                params[key] = if spec["visibleWhen"]["effect"] == *effect {
                    spec["default"].clone()
                } else {
                    spec.get("max").unwrap_or(&spec["default"]).clone()
                };
            }
        }
        assert_eq!(
            overlap(&params),
            64,
            "inactive controls enlarged {effect}'s source context"
        );
    }
    assert_eq!(required_overlap(b"{}"), 64);
}

#[test]
fn selected_geometry_requests_sufficient_context_without_exceeding_the_desktop_limit() {
    // These are independent support bounds for the maximum accepted geometry, including
    // sampling/normal margins. A larger accepted control must not retain the old 64px halo.
    for (params, minimum) in [
        (
            json!({"effect":"drip","drip_length":150,"drip_size":1000}),
            1502,
        ),
        (
            json!({"effect":"icicles","icicles_length":150,"icicles_size":1000}),
            1502,
        ),
        (json!({"effect":"fire","fire_height":1500}), 1504),
        (json!({"effect":"smoke","smoke_height":1500}), 1504),
        (json!({"effect":"glow","glow_radius":1500}), 1504),
        (json!({"effect":"backlight","backlight_radius":1500}), 1504),
        (json!({"effect":"corona","corona_radius":1500}), 1504),
        (json!({"effect":"electrify","electrify_reach":1500}), 1506),
        (
            json!({"effect":"shadow","shadow_distance":1500,"shadow_softness":12}),
            1520,
        ),
        (json!({"effect":"extrude","extrude_length":1500}), 1534),
        (
            json!({"effect":"motion_trail","motion_trail_length":1500}),
            1534,
        ),
        (json!({"effect":"bevel","bevel_size":500}), 1004),
        (json!({"effect":"glass","glass_refraction":500}), 467),
        (json!({"effect":"snow","snow_depth":500}), 667),
        (
            json!({"effect":"water_drops","water_drops_scale":1500}),
            157,
        ),
    ] {
        let actual = overlap(&params);
        assert!(
            (minimum..=1536).contains(&actual),
            "inadequate or excessive context for {params}: {actual}"
        );
        let bytes = serde_json::to_vec(&params).unwrap();
        // SAFETY: the parameter allocation is live and its actual length is supplied.
        assert_eq!(
            unsafe { pc_overlap(bytes.as_ptr(), bytes.len() as u32) },
            actual as i32
        );
    }
}

#[test]
fn every_manifest_endpoint_stays_within_the_native_context_budget() {
    let manifest: Value = serde_json::from_str(&manifest_json()).unwrap();
    let specs = manifest["params"].as_object().unwrap();
    for effect in specs["effect"]["options"].as_array().unwrap() {
        for endpoint in ["min", "max"] {
            let mut params = json!({"effect":effect});
            for (key, spec) in specs {
                if key != "effect" {
                    params[key] = spec.get(endpoint).unwrap_or(&spec["default"]).clone();
                }
            }
            let actual = overlap(&params);
            assert!(
                (64..=1536).contains(&actual),
                "{effect}/{endpoint} requested {actual}px"
            );
        }
    }
}

#[test]
fn manifest_keeps_full_context_for_materials_with_long_distance_field_probes() {
    let manifest: Value = serde_json::from_str(&manifest_json()).unwrap();
    assert_eq!(manifest["clampContext"], true);
    let condition = &manifest["clampContextWhen"];
    assert_eq!(condition["parameter"], "effect");
    let accepted = condition["values"]
        .as_array()
        .expect("conditional clipping effects");
    let specs = manifest["params"].as_object().unwrap();
    for effect in manifest["params"]["effect"]["options"].as_array().unwrap() {
        let name = effect.as_str().unwrap();
        let mut maximum = json!({"effect":effect});
        for (key, spec) in specs {
            if key != "effect" && spec["visibleWhen"]["effect"] == *effect {
                maximum[key] = spec.get("max").unwrap_or(&spec["default"]).clone();
            }
        }
        if overlap(&maximum) > 256 {
            assert_eq!(
                accepted.contains(effect),
                !matches!(name, "bevel" | "glass" | "motion_trail"),
                "{name} must use the large-context layout its distance probes support"
            );
        }
    }
    let registered = manifest["params"]["effect"]["options"].as_array().unwrap();
    assert!(
        accepted.iter().all(|effect| registered.contains(effect)),
        "clipping condition names an unregistered effect"
    );
}

fn invoke_abi(pixels: &mut [f32], width: usize, height: usize, params: &Value) -> i32 {
    let bytes = serde_json::to_vec(params).unwrap();
    // SAFETY: both allocations are live and correctly aligned, and all supplied lengths
    // describe their actual storage. This is the same entry point called by the WASM host.
    unsafe {
        pc_filter(
            pixels.as_mut_ptr(),
            std::mem::size_of_val(pixels) as u32,
            width as u32,
            height as u32,
            4,
            8 | 256 | (3 << 16),
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    }
}

#[test]
fn abi_rejects_missing_or_insufficient_context_without_touching_source_pixels() {
    for supplied in [None, Some(64), Some(1503)] {
        let mut params = json!({"effect":"fire","fire_height":1500});
        if let Some(pad) = supplied {
            params["_image"] = json!({"overlap":pad});
        }
        let mut pixels = vec![0.23, 0.41, 0.67, 0.89];
        let before = pixels.clone();
        assert_eq!(invoke_abi(&mut pixels, 1, 1, &params), 5, "{params}");
        assert_eq!(pixels, before, "rejected work modified its source");
    }
}

#[test]
fn abi_accepts_normal_settings_with_the_existing_64_pixel_halo() {
    let width = 132;
    let mut pixels = vec![0.0; width * width * 4];
    pixels[(66 * width + 66) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.8, 1.0]);
    assert_eq!(
        invoke_abi(
            &mut pixels,
            width,
            width,
            &json!({
                "effect":"glow",
                "_image":{"x":-64,"y":-64,"canvasWidth":4,"canvasHeight":4,"overlap":64}
            })
        ),
        0
    );
    assert!(pixels[(66 * width + 65) * 4 + 3] > 0.0);
}

fn apply(pixels: &mut [f32], width: usize, height: usize, params: &Value) {
    let bytes = serde_json::to_vec(params).unwrap();
    let mut job = Job {
        px: pixels,
        width,
        height,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: &bytes,
    };
    assert_eq!(filter(&mut job), 0);
}

#[test]
fn negotiated_large_halos_match_a_full_frame_at_the_kept_band_boundary() {
    const FULL: usize = 864;
    const CORE: usize = 96;
    for (mut params, source_y) in [
        (
            json!({"effect":"drip","drip_length":100,"drip_size":300,"drip_amount":100,"drip_size_jitter":0}),
            228,
        ),
        (
            json!({"effect":"icicles","icicles_length":100,"icicles_size":300,"icicles_density":100,"icicles_size_jitter":0}),
            228,
        ),
        (json!({"effect":"fire","fire_height":300}), 612),
        (json!({"effect":"smoke","smoke_height":300}), 612),
        (json!({"effect":"glow","glow_radius":300}), 612),
    ] {
        let pad = overlap(&params) as usize;
        let side = CORE + pad * 2;
        let origin = (FULL - side) / 2;
        let mut source = vec![0.0; FULL * FULL * 4];
        for y in source_y..source_y + 4 {
            for x in 64..FULL - 64 {
                source[(y * FULL + x) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.8, 1.0]);
            }
        }
        let mut band = Vec::with_capacity(side * side * 4);
        for y in origin..origin + side {
            band.extend_from_slice(
                &source[(y * FULL + origin) * 4..(y * FULL + origin + side) * 4],
            );
        }
        let band_before = band.clone();
        params["_image"] = json!({"x":0,"y":0,"canvasWidth":FULL,"canvasHeight":FULL});
        apply(&mut source, FULL, FULL, &params);
        params["_image"] =
            json!({"x":origin,"y":origin,"canvasWidth":FULL,"canvasHeight":FULL,"overlap":pad});
        assert_eq!(invoke_abi(&mut band, side, side, &params), 0);
        let mut maximum_error = 0.0_f32;
        let mut visible = 0;
        for y in pad..pad + CORE {
            for x in pad..pad + CORE {
                for c in 0..4 {
                    maximum_error = maximum_error.max(
                        (band[(y * side + x) * 4 + c]
                            - source[((y + origin) * FULL + x + origin) * 4 + c])
                            .abs(),
                    );
                }
                visible += usize::from(band[(y * side + x) * 4 + 3] > 0.001);
            }
        }
        assert!(
            visible > 0,
            "seam fixture produced no effect: {}",
            params["effect"]
        );
        assert!(
            maximum_error < 0.0001,
            "{} changed across its processing boundary: {maximum_error}",
            params["effect"]
        );
        assert_eq!(
            &band[..side * 4],
            &band_before[..side * 4],
            "source-only halo was modified"
        );
    }
}

#[derive(Clone, Copy)]
struct Region {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

/// Model the host's two input layouts from one canvas: legacy symmetric edge replication,
/// and context intersected with the canvas plus a 36-pixel replicated support guard.
fn render_canvas_region(
    source: &[f32],
    canvas_width: usize,
    canvas_height: usize,
    settings: &Value,
    core: Region,
    clipped: bool,
) -> Vec<f32> {
    const GUARD: i32 = 36;
    let pad = overlap(settings) as i32;
    let mut left = core.x as i32 - pad;
    let mut top = core.y as i32 - pad;
    let mut right = (core.x + core.width) as i32 + pad;
    let mut bottom = (core.y + core.height) as i32 + pad;
    if clipped {
        left = left.max(-GUARD);
        top = top.max(-GUARD);
        right = right.min(canvas_width as i32 + GUARD);
        bottom = bottom.min(canvas_height as i32 + GUARD);
    }
    let width = (right - left) as usize;
    let height = (bottom - top) as usize;
    let output_x = (core.x as i32 - left) as usize;
    let output_y = (core.y as i32 - top) as usize;
    let mut input = Vec::with_capacity(width * height * 4);
    for y in top..bottom {
        let sy = y.clamp(0, canvas_height as i32 - 1) as usize;
        for x in left..right {
            let sx = x.clamp(0, canvas_width as i32 - 1) as usize;
            input.extend_from_slice(&source[(sy * canvas_width + sx) * 4..][..4]);
        }
    }
    let before = input.clone();
    let mut params = settings.clone();
    params["_image"] = json!({
        "x":left,"y":top,"canvasWidth":canvas_width,"canvasHeight":canvas_height,
        "overlap":pad
    });
    if clipped {
        params["_image"]["outputX"] = json!(output_x);
        params["_image"]["outputY"] = json!(output_y);
        params["_image"]["outputWidth"] = json!(core.width);
        params["_image"]["outputHeight"] = json!(core.height);
    }
    assert_eq!(
        invoke_abi(&mut input, width, height, &params),
        0,
        "{params}"
    );
    assert!(input.iter().all(|value| value.is_finite()), "{params}");
    let mut output = Vec::with_capacity(core.width * core.height * 4);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            if (output_y..output_y + core.height).contains(&y)
                && (output_x..output_x + core.width).contains(&x)
            {
                output.extend_from_slice(&input[i..i + 4]);
            } else {
                assert_eq!(
                    &input[i..i + 4],
                    &before[i..i + 4],
                    "{} modified source context outside its explicit output: ({x},{y}), clipped={clipped}",
                    settings["effect"]
                );
            }
        }
    }
    output
}

fn edge_fixture(width: usize, height: usize) -> Vec<f32> {
    let mut source = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let alpha = if ((1..138).contains(&x) && (70..190).contains(&y))
                || (x >= 250 && (105..185).contains(&y))
                || ((32..80).contains(&x) && y >= 250)
                || (y == 34 && x < 220)
            {
                1.0
            } else if (x == 225 && y < 79) || ((190..300).contains(&x) && y == 224) {
                0.2
            } else {
                0.0
            };
            source.extend_from_slice(&[
                0.15 + x as f32 / width as f32 * 0.7,
                0.2 + y as f32 / height as f32 * 0.6,
                if (x / 17 + y / 13) % 2 == 0 { 0.7 } else { 0.3 },
                alpha,
            ]);
        }
    }
    source
}

#[test]
fn clipped_canvas_context_preserves_edges_absolute_phase_and_adjacent_band_cores() {
    const WIDTH: usize = 384;
    const HEIGHT: usize = 256;
    let source = edge_fixture(WIDTH, HEIGHT);
    let whole = Region {
        x: 0,
        y: 0,
        width: WIDTH,
        height: HEIGHT,
    };
    let cases = [
        json!({"effect":"fire","fire_height":300}),
        json!({"effect":"smoke","smoke_height":300}),
        json!({"effect":"glow","glow_radius":300}),
        json!({"effect":"corona","corona_radius":300}),
        json!({"effect":"backlight","backlight_radius":300}),
        json!({"effect":"electrify","electrify_reach":300,"electrify_density":100}),
        json!({"effect":"drip","drip_length":100,"drip_size":300,"drip_amount":100,"drip_size_jitter":0}),
        json!({"effect":"icicles","icicles_length":100,"icicles_size":300,"icicles_density":100,"icicles_size_jitter":0}),
        json!({"effect":"snow","snow_depth":300,"snow_amount":100}),
        json!({"effect":"shadow","shadow_distance":300,"shadow_angle":0,"shadow_softness":12,"shadow_opacity":100}),
        json!({"effect":"extrude","extrude_length":300,"extrude_angle":0}),
    ];
    let mut failures = Vec::new();
    for params in cases {
        let expected = render_canvas_region(&source, WIDTH, HEIGHT, &params, whole, false);
        assert!(
            expected
                .iter()
                .zip(&source)
                .any(|(a, b)| (a - b).abs() > 0.001),
            "fixture failed to exercise {}",
            params["effect"]
        );
        let mut maximum_error = 0.0_f32;
        let mut worst = (0, 0, 0);
        // These four explicit cores cover all canvas borders and share both a horizontal
        // and a vertical processing seam. Their clipped source buffers have asymmetric pads.
        for core_y in [0, HEIGHT / 2] {
            for core_x in [0, WIDTH / 2] {
                let core = Region {
                    x: core_x,
                    y: core_y,
                    width: WIDTH / 2,
                    height: HEIGHT / 2,
                };
                let actual = render_canvas_region(&source, WIDTH, HEIGHT, &params, core, true);
                for y in 0..core.height {
                    for x in 0..core.width {
                        let a = (y * core.width + x) * 4;
                        let b = ((y + core.y) * WIDTH + x + core.x) * 4;
                        for channel in 0..4 {
                            let delta = if channel == 3 {
                                (actual[a + 3] - expected[b + 3]).abs()
                            } else {
                                (actual[a + channel] * actual[a + 3]
                                    - expected[b + channel] * expected[b + 3])
                                    .abs()
                            };
                            if delta > maximum_error {
                                maximum_error = delta;
                                worst = (x + core.x, y + core.y, channel);
                            }
                        }
                    }
                }
            }
        }
        if maximum_error >= 0.0001 {
            failures.push(format!(
                "{} changed after clipping repeated canvas-edge context: error {maximum_error} at {worst:?}",
                params["effect"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn clipped_context_keeps_a_single_faint_pixel_in_long_extrusions_and_trails() {
    const WIDTH: usize = 384;
    const HEIGHT: usize = 16;
    for (effect, source_x, probe_x, minimum_alpha) in [
        ("extrude", 24, 280, 0.1),
        ("motion_trail", 330, 60, 0.000001),
    ] {
        let mut source = vec![0.0; WIDTH * HEIGHT * 4];
        source[(8 * WIDTH + source_x) * 4..][..4].copy_from_slice(&[0.2, 0.4, 0.8, 0.2]);
        let params = json!({
            "effect":effect,"extrude_length":300,"extrude_angle":0,
            "motion_trail_length":300,"motion_trail_angle":0,"motion_trail_opacity":100
        });
        let core = Region {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        };
        let expected = render_canvas_region(&source, WIDTH, HEIGHT, &params, core, false);
        let actual = render_canvas_region(&source, WIDTH, HEIGHT, &params, core, true);
        assert!(
            actual[(8 * WIDTH + probe_x) * 4 + 3] > minimum_alpha,
            "{effect} lost its one-pixel translucent source"
        );
        let maximum_error = actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0.iter())
            .map(|(a, b)| {
                (a[3] - b[3]).abs().max(
                    (0..3)
                        .map(|c| (a[c] * a[3] - b[c] * b[3]).abs())
                        .fold(0.0_f32, f32::max),
                )
            })
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_error < 0.0001,
            "{effect} changed its translucent geometry after context clipping: {maximum_error}"
        );
    }
}

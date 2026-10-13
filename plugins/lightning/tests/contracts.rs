use freecandy7::{Job, filter, manifest_json};

#[test]
fn every_effect_control_has_a_readable_label_and_scoped_visibility() {
    let m: serde_json::Value = serde_json::from_str(&manifest_json()).unwrap();
    assert_eq!(m["presetSelector"], "effect");
    let params = m["params"].as_object().unwrap();
    let effects = params["effect"]["options"].as_array().unwrap();
    for (key, spec) in params {
        let label = spec["label"]
            .as_str()
            .unwrap_or_else(|| panic!("label missing for {key}"));
        assert!(!label.contains('_'), "{key}: {label}");
        if key != "effect" {
            assert!(
                effects.contains(&spec["visibleWhen"]["effect"]),
                "unscoped control {key}"
            );
        }
    }
    let visible = |effect: &str| -> Vec<&str> {
        params
            .iter()
            .filter(|(key, spec)| *key == "effect" || spec["visibleWhen"]["effect"] == effect)
            .map(|(key, _)| key.as_str())
            .collect()
    };
    assert_eq!(
        visible("fire"),
        [
            "effect",
            "fire_detail",
            "fire_height",
            "fire_intensity",
            "fire_seed"
        ]
    );
    assert!(
        visible("clouds")
            .iter()
            .all(|key| *key == "effect" || key.starts_with("clouds_"))
    );
    assert_eq!(params["start"]["visibleWhen"]["effect"], "lightning");
    assert_eq!(
        params["swirl_center"]["visibleWhen"]["swirl_style"],
        "distort"
    );
    assert_eq!(params["star_depth"]["visibleWhen"]["star_style"], "polygon");
}
#[test]
fn all_reference_effects_are_registered() {
    let manifest = manifest_json();
    for name in [
        "lightning",
        "animal_fur",
        "brick_wall",
        "brushed_metal",
        "clouds",
        "diamond_plate",
        "marble",
        "reptile_skin",
        "rust",
        "stone_wall",
        "texture_noise",
        "weave",
        "wood",
        "bevel",
        "chrome",
        "glass",
        "extrude",
        "shadow",
        "motion_trail",
        "water_drops",
        "drip",
        "snow",
        "icicles",
        "backlight",
        "corona",
        "electrify",
        "fire",
        "glow",
        "smoke",
        "star",
        "swirl",
        "ripples",
    ] {
        assert!(manifest.contains(&format!("\"{name}\"")), "missing {name}");
    }
    assert!(manifest.len() < 65536);
}
#[test]
fn malformed_buffer_is_rejected_before_rendering() {
    let mut px = [0.0; 4];
    let mut job = Job {
        px: &mut px,
        width: 200,
        height: 200,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: b"{}",
    };
    assert_ne!(filter(&mut job), 0);
}

#[test]
fn unknown_effect_is_rejected_without_modifying_pixels() {
    let mut px = [0.1, 0.2, 0.3, 0.4];
    let before = px;
    let mut job = Job {
        px: &mut px,
        width: 1,
        height: 1,
        channels: 4,
        format: 8 | 256 | (3 << 16),
        json: br#"{"effect":"missing"}"#,
    };
    assert_eq!(filter(&mut job), 4);
    assert_eq!(px, before);
}
#[test]
fn empty_dimensions_are_rejected() {
    let mut px = [];
    for (width, height) in [(0, 1), (1, 0), (0, 0)] {
        let mut job = Job {
            px: &mut px,
            width,
            height,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        assert_eq!(filter(&mut job), 1);
    }
}

#[test]
fn ripple_keeps_host_context_pixels_unchanged() {
    let mut pixels = vec![0.3; 24 * 24 * 4];
    for p in pixels.as_chunks_mut::<4>().0 {
        p[3] = 1.0;
    }
    let original = pixels.clone();
    let mut job=Job{px:&mut pixels,width:24,height:24,channels:4,format:8|256|(3<<16),json:br#"{"effect":"ripples","_image":{"x":-4,"y":-4,"canvasWidth":16,"canvasHeight":16,"overlap":4}}"#};
    assert_eq!(filter(&mut job), 0);
    assert_eq!(&pixels[..24 * 4], &original[..24 * 4]);
}

#[test]
fn built_in_presets_are_useful_variations_scoped_to_every_effect() {
    let m: serde_json::Value = serde_json::from_str(&manifest_json()).unwrap();
    let presets = m["presets"].as_object().expect("built-in preset library");
    let params = m["params"].as_object().unwrap();
    for effect in params["effect"]["options"].as_array().unwrap() {
        let effect = effect.as_str().unwrap();
        let variants: Vec<_> = presets
            .iter()
            .filter(|(_, p)| p["effect"] == effect)
            .collect();
        assert!(
            variants.len() >= 3,
            "at least three useful presets for {effect}"
        );
        let mut unique = std::collections::BTreeSet::new();
        for (name, values) in variants {
            assert_ne!(name.to_lowercase().replace(' ', "_"), effect);
            let mut resolved = serde_json::Map::new();
            for (key, spec) in params {
                if key != "effect"
                    && spec["visibleWhen"]["effect"] == effect
                    && !key.ends_with("seed")
                {
                    resolved.insert(
                        key.clone(),
                        values.get(key).unwrap_or(&spec["default"]).clone(),
                    );
                }
            }
            assert!(
                unique.insert(serde_json::to_string(&resolved).unwrap()),
                "duplicate appearance preset {name}"
            );
            for key in values.as_object().unwrap().keys() {
                assert!(params.contains_key(key), "unknown preset control {key}");
                assert!(
                    key == "effect" || params[key]["visibleWhen"]["effect"] == effect,
                    "unrelated control in {name}: {key}"
                );
            }
        }
    }
}

#[test]
fn fur_styles_leave_coat_choices_independent() {
    let m: serde_json::Value = serde_json::from_str(&manifest_json()).unwrap();
    let presets = m["presets"].as_object().unwrap();
    let material_keys = [
        "animal_fur_pattern",
        "animal_fur_scale",
        "animal_fur_scaling",
        "animal_fur_natural_colors",
        "animal_fur_light",
        "animal_fur_dark",
        "animal_fur_seed",
    ];
    let preserved = m["presetPreserve"].as_array().unwrap();
    for key in material_keys {
        assert!(
            preserved.iter().any(|entry| entry == key),
            "style presets must preserve {key}"
        );
    }
    for (name, preset) in presets.iter().filter(|(_, p)| p["effect"] == "animal_fur") {
        for key in material_keys {
            assert!(
                preset.get(key).is_none(),
                "{name} unexpectedly replaces the coat setting {key}"
            );
        }
    }
    for name in [
        "Short smooth fur",
        "Long shaggy fur",
        "Soft dense fur",
        "Wind-swept fur",
        "Furry silhouette",
    ] {
        assert_eq!(presets[name]["effect"], "animal_fur");
    }
}

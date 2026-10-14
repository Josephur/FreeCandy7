#[test]
fn shape_effects_request_the_actual_selection_as_their_input_shape() {
    let m: serde_json::Value = serde_json::from_str(&freecandy7::manifest_json()).unwrap();
    assert_eq!(
        m["selectionShape"]["parameter"], "effect",
        "selection must reach the shape renderer"
    );
    let effects = m["selectionShape"]["values"].as_array().unwrap();
    let expected = [
        "bevel",
        "chrome",
        "glass",
        "extrude",
        "shadow",
        "motion_trail",
        "drip",
        "snow",
        "icicles",
        "backlight",
        "corona",
        "electrify",
        "fire",
        "glow",
        "smoke",
        "neon_tubes",
        "holographic_foil",
        "cracked_paint",
        "paper_cut",
    ];
    assert_eq!(effects.len(), expected.len());
    for name in expected {
        assert!(
            effects.iter().any(|v| v == name),
            "missing selection input for {name}"
        );
    }
    assert!(
        !effects.iter().any(|v| v == "animal_fur"),
        "texture fill keeps ordinary selection clipping"
    );
}

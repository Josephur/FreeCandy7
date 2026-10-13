//! Clean-room procedural material textures.

use super::Effect;
use super::common::{canvas, supported, write};
use crate::Job;
use libm::{cosf, sinf};

pub const EFFECTS: &[Effect] = &[
    Effect {
        name: "animal_fur",
        params: ANIMAL_FUR_PARAMS,
        render: animal_fur,
    },
    Effect {
        name: "brick_wall",
        params: BRICK_WALL_PARAMS,
        render: brick_wall,
    },
    Effect {
        name: "brushed_metal",
        params: BRUSHED_METAL_PARAMS,
        render: brushed_metal,
    },
    Effect {
        name: "clouds",
        params: CLOUDS_PARAMS,
        render: clouds,
    },
    Effect {
        name: "diamond_plate",
        params: DIAMOND_PLATE_PARAMS,
        render: diamond_plate,
    },
    Effect {
        name: "marble",
        params: MARBLE_PARAMS,
        render: marble,
    },
    Effect {
        name: "reptile_skin",
        params: REPTILE_SKIN_PARAMS,
        render: reptile_skin,
    },
    Effect {
        name: "rust",
        params: RUST_PARAMS,
        render: rust,
    },
    Effect {
        name: "stone_wall",
        params: STONE_WALL_PARAMS,
        render: stone_wall,
    },
    Effect {
        name: "texture_noise",
        params: TEXTURE_NOISE_PARAMS,
        render: texture_noise,
    },
    Effect {
        name: "weave",
        params: WEAVE_PARAMS,
        render: weave,
    },
    Effect {
        name: "wood",
        params: WOOD_PARAMS,
        render: wood,
    },
];

const ANIMAL_FUR_PARAMS: &str = r##"    "animal_fur_pattern": {"type":"choice","options":["fur","giraffe","leopard","zebra","tiger","cheetah"],"default":"fur","group":"Animal Fur","label":"Coat type","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Animal Fur","label":"Scaling","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_scale": {"type":"number","min":4,"max":160,"default":38,"group":"Animal Fur","label":"Scale","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_detail": {"type":"number","min":0,"max":100,"default":58,"group":"Animal Fur","label":"Detail","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_direction": {"type":"angle","default":90,"group":"Animal Fur","label":"Direction","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_light": {"type":"color","default":"#d6ad42","group":"Animal Fur","label":"Highlight colour","visibleWhen":{"effect":"animal_fur","animal_fur_natural_colors":false}},
    "animal_fur_dark": {"type":"color","default":"#24180c","group":"Animal Fur","label":"Shadow colour","visibleWhen":{"effect":"animal_fur","animal_fur_natural_colors":false}},
    "animal_fur_seed": {"type":"int","min":0,"max":9999,"default":7,"group":"Animal Fur","label":"Random seed","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_length": {"type":"number","min":1,"max":60,"default":12,"group":"Animal Fur","label":"Hair length","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_density": {"type":"number","min":10,"max":100,"default":65,"group":"Animal Fur","label":"Hair density","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_flow": {"type":"number","min":0,"max":100,"default":25,"group":"Animal Fur","label":"Flow variation","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_lighting": {"type":"angle","default":315,"group":"Animal Fur","label":"Light direction","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_relief": {"type":"number","min":0,"max":100,"default":35,"group":"Animal Fur","label":"Hair relief","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_edge": {"type":"number","min":0,"max":30,"default":0,"group":"Animal Fur","label":"Edge hairs","visibleWhen":{"effect":"animal_fur","animal_fur_preserve_alpha":true}},
    "animal_fur_natural_colors": {"type":"bool","default":true,"group":"Animal Fur","label":"Natural coat colours","visibleWhen":{"effect":"animal_fur"}},
    "animal_fur_preserve_alpha": {"type":"bool","default":false,"group":"Animal Fur","label":"Preserve transparency","visibleWhen":{"effect":"animal_fur"}}"##;
const BRICK_WALL_PARAMS: &str = r##"    "brick_wall_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Brick Wall","label":"Scaling","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_scale": {"type":"number","min":12,"max":180,"default":52,"group":"Brick Wall","label":"Scale","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_detail": {"type":"number","min":0,"max":100,"default":50,"group":"Brick Wall","label":"Detail","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_mortar": {"type":"color","default":"#c8c4b8","group":"Brick Wall","label":"Mortar","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_brick": {"type":"color","default":"#9c4932","group":"Brick Wall","label":"Brick","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_seed": {"type":"int","min":0,"max":9999,"default":11,"group":"Brick Wall","label":"Random seed","visibleWhen":{"effect":"brick_wall"}},
    "brick_wall_preserve_alpha": {"type":"bool","default":false,"group":"Brick Wall","label":"Preserve transparency","visibleWhen":{"effect":"brick_wall"}}"##;
const BRUSHED_METAL_PARAMS: &str = r##"    "brushed_metal_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Brushed Metal","label":"Scaling","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_scale": {"type":"number","min":1,"max":80,"default":13,"group":"Brushed Metal","label":"Scale","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_detail": {"type":"number","min":0,"max":100,"default":45,"group":"Brushed Metal","label":"Detail","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_direction": {"type":"angle","default":0,"group":"Brushed Metal","label":"Direction","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_colour": {"type":"color","default":"#87969c","group":"Brushed Metal","label":"Colour","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_seed": {"type":"int","min":0,"max":9999,"default":13,"group":"Brushed Metal","label":"Random seed","visibleWhen":{"effect":"brushed_metal"}},
    "brushed_metal_preserve_alpha": {"type":"bool","default":false,"group":"Brushed Metal","label":"Preserve transparency","visibleWhen":{"effect":"brushed_metal"}}"##;
const CLOUDS_PARAMS: &str = r##"    "clouds_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Clouds","label":"Scaling","visibleWhen":{"effect":"clouds"}},
    "clouds_scale": {"type":"number","min":12,"max":220,"default":90,"group":"Clouds","label":"Scale","visibleWhen":{"effect":"clouds"}},
    "clouds_detail": {"type":"number","min":0,"max":100,"default":65,"group":"Clouds","label":"Detail","visibleWhen":{"effect":"clouds"}},
    "clouds_light": {"type":"color","default":"#eef5ff","group":"Clouds","label":"Highlight colour","visibleWhen":{"effect":"clouds"}},
    "clouds_dark": {"type":"color","default":"#214778","group":"Clouds","label":"Shadow colour","visibleWhen":{"effect":"clouds"}},
    "clouds_seed": {"type":"int","min":0,"max":9999,"default":17,"group":"Clouds","label":"Random seed","visibleWhen":{"effect":"clouds"}},
    "clouds_preserve_alpha": {"type":"bool","default":false,"group":"Clouds","label":"Preserve transparency","visibleWhen":{"effect":"clouds"}}"##;
const DIAMOND_PLATE_PARAMS: &str = r##"    "diamond_plate_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Diamond Plate","label":"Scaling","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_scale": {"type":"number","min":8,"max":96,"default":34,"group":"Diamond Plate","label":"Scale","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_detail": {"type":"number","min":0,"max":100,"default":55,"group":"Diamond Plate","label":"Detail","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_direction": {"type":"angle","default":0,"group":"Diamond Plate","label":"Direction","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_colour": {"type":"color","default":"#9ca6a9","group":"Diamond Plate","label":"Colour","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_seed": {"type":"int","min":0,"max":9999,"default":19,"group":"Diamond Plate","label":"Random seed","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_relief": {"type":"number","min":0,"max":100,"default":48,"group":"Diamond Plate","label":"Tread height","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_roughness": {"type":"number","min":0,"max":100,"default":24,"group":"Diamond Plate","label":"Roughness","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_contrast": {"type":"number","min":0,"max":100,"default":85,"group":"Diamond Plate","label":"Reflection contrast","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_lighting": {"type":"angle","default":315,"group":"Diamond Plate","label":"Light direction","visibleWhen":{"effect":"diamond_plate"}},
    "diamond_plate_preserve_alpha": {"type":"bool","default":false,"group":"Diamond Plate","label":"Preserve transparency","visibleWhen":{"effect":"diamond_plate"}}"##;
const MARBLE_PARAMS: &str = r##"    "marble_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Marble","label":"Scaling","visibleWhen":{"effect":"marble"}},
    "marble_scale": {"type":"number","min":12,"max":220,"default":88,"group":"Marble","label":"Scale","visibleWhen":{"effect":"marble"}},
    "marble_detail": {"type":"number","min":0,"max":100,"default":60,"group":"Marble","label":"Detail","visibleWhen":{"effect":"marble"}},
    "marble_light": {"type":"color","default":"#e7dcc9","group":"Marble","label":"Highlight colour","visibleWhen":{"effect":"marble"}},
    "marble_dark": {"type":"color","default":"#785a56","group":"Marble","label":"Shadow colour","visibleWhen":{"effect":"marble"}},
    "marble_seed": {"type":"int","min":0,"max":9999,"default":23,"group":"Marble","label":"Random seed","visibleWhen":{"effect":"marble"}},
    "marble_preserve_alpha": {"type":"bool","default":false,"group":"Marble","label":"Preserve transparency","visibleWhen":{"effect":"marble"}}"##;
const REPTILE_SKIN_PARAMS: &str = r##"    "reptile_skin_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Reptile Skin","label":"Scaling","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_scale": {"type":"number","min":8,"max":120,"default":37,"group":"Reptile Skin","label":"Scale","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_detail": {"type":"number","min":0,"max":100,"default":60,"group":"Reptile Skin","label":"Detail","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_light": {"type":"color","default":"#bd803b","group":"Reptile Skin","label":"Highlight colour","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_dark": {"type":"color","default":"#331f12","group":"Reptile Skin","label":"Shadow colour","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_seed": {"type":"int","min":0,"max":9999,"default":29,"group":"Reptile Skin","label":"Random seed","visibleWhen":{"effect":"reptile_skin"}},
    "reptile_skin_preserve_alpha": {"type":"bool","default":false,"group":"Reptile Skin","label":"Preserve transparency","visibleWhen":{"effect":"reptile_skin"}}"##;
const RUST_PARAMS: &str = r##"    "rust_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Rust","label":"Scaling","visibleWhen":{"effect":"rust"}},
    "rust_scale": {"type":"number","min":8,"max":160,"default":55,"group":"Rust","label":"Scale","visibleWhen":{"effect":"rust"}},
    "rust_detail": {"type":"number","min":0,"max":100,"default":62,"group":"Rust","label":"Detail","visibleWhen":{"effect":"rust"}},
    "rust_light": {"type":"color","default":"#c97b37","group":"Rust","label":"Highlight colour","visibleWhen":{"effect":"rust"}},
    "rust_dark": {"type":"color","default":"#394448","group":"Rust","label":"Shadow colour","visibleWhen":{"effect":"rust"}},
    "rust_seed": {"type":"int","min":0,"max":9999,"default":31,"group":"Rust","label":"Random seed","visibleWhen":{"effect":"rust"}},
    "rust_preserve_alpha": {"type":"bool","default":false,"group":"Rust","label":"Preserve transparency","visibleWhen":{"effect":"rust"}}"##;
const STONE_WALL_PARAMS: &str = r##"    "stone_wall_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Stone Wall","label":"Scaling","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_scale": {"type":"number","min":20,"max":180,"default":67,"group":"Stone Wall","label":"Scale","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_detail": {"type":"number","min":0,"max":100,"default":56,"group":"Stone Wall","label":"Detail","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_mortar": {"type":"color","default":"#55534c","group":"Stone Wall","label":"Mortar","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_stone": {"type":"color","default":"#aaa89d","group":"Stone Wall","label":"Stone","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_seed": {"type":"int","min":0,"max":9999,"default":37,"group":"Stone Wall","label":"Random seed","visibleWhen":{"effect":"stone_wall"}},
    "stone_wall_preserve_alpha": {"type":"bool","default":false,"group":"Stone Wall","label":"Preserve transparency","visibleWhen":{"effect":"stone_wall"}}"##;
const TEXTURE_NOISE_PARAMS: &str = r##"    "texture_noise_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Texture Noise","label":"Scaling","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_scale": {"type":"number","min":2,"max":100,"default":23,"group":"Texture Noise","label":"Scale","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_detail": {"type":"number","min":0,"max":100,"default":55,"group":"Texture Noise","label":"Detail","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_variant": {"type":"choice","options":["ridged","cellular"],"default":"cellular","group":"Texture Noise","label":"Variant","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_light": {"type":"color","default":"#ead7bc","group":"Texture Noise","label":"Highlight colour","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_dark": {"type":"color","default":"#382641","group":"Texture Noise","label":"Shadow colour","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_seed": {"type":"int","min":0,"max":9999,"default":41,"group":"Texture Noise","label":"Random seed","visibleWhen":{"effect":"texture_noise"}},
    "texture_noise_preserve_alpha": {"type":"bool","default":false,"group":"Texture Noise","label":"Preserve transparency","visibleWhen":{"effect":"texture_noise"}}"##;
const WEAVE_PARAMS: &str = r##"    "weave_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Weave","label":"Scaling","visibleWhen":{"effect":"weave"}},
    "weave_scale": {"type":"number","min":4,"max":80,"default":18,"group":"Weave","label":"Scale","visibleWhen":{"effect":"weave"}},
    "weave_detail": {"type":"number","min":0,"max":100,"default":48,"group":"Weave","label":"Detail","visibleWhen":{"effect":"weave"}},
    "weave_direction": {"type":"angle","default":0,"group":"Weave","label":"Direction","visibleWhen":{"effect":"weave"}},
    "weave_light": {"type":"color","default":"#376487","group":"Weave","label":"Highlight colour","visibleWhen":{"effect":"weave"}},
    "weave_dark": {"type":"color","default":"#142b45","group":"Weave","label":"Shadow colour","visibleWhen":{"effect":"weave"}},
    "weave_seed": {"type":"int","min":0,"max":9999,"default":43,"group":"Weave","label":"Random seed","visibleWhen":{"effect":"weave"}},
    "weave_preserve_alpha": {"type":"bool","default":false,"group":"Weave","label":"Preserve transparency","visibleWhen":{"effect":"weave"}}"##;
const WOOD_PARAMS: &str = r##"    "wood_scaling": {"type":"choice","options":["fixed","adaptive"],"default":"fixed","group":"Wood","label":"Scaling","visibleWhen":{"effect":"wood"}},
    "wood_scale": {"type":"number","min":10,"max":180,"default":76,"group":"Wood","label":"Scale","visibleWhen":{"effect":"wood"}},
    "wood_detail": {"type":"number","min":0,"max":100,"default":56,"group":"Wood","label":"Detail","visibleWhen":{"effect":"wood"}},
    "wood_light": {"type":"color","default":"#b5642d","group":"Wood","label":"Highlight colour","visibleWhen":{"effect":"wood"}},
    "wood_dark": {"type":"color","default":"#35180e","group":"Wood","label":"Shadow colour","visibleWhen":{"effect":"wood"}},
    "wood_seed": {"type":"int","min":0,"max":9999,"default":47,"group":"Wood","label":"Random seed","visibleWhen":{"effect":"wood"}},
    "wood_preserve_alpha": {"type":"bool","default":false,"group":"Wood","label":"Preserve transparency","visibleWhen":{"effect":"wood"}}"##;

// The renders and manifest entries stay together, so the host can build its selector from this list.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_all_twelve_distinct_textures() {
        let names: [&str; 12] = [
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
        ];
        assert_eq!(EFFECTS.len(), names.len());
        for (effect, name) in EFFECTS.iter().zip(names) {
            assert_eq!(effect.name, name);
            assert!(!effect.params.is_empty());
        }
    }

    #[test]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn each_renderer_fills_rgb_and_respects_alpha_toggle() {
        for effect in EFFECTS {
            let mut pixels = [0.0_f32; 4 * 4 * 4];
            for px in pixels.chunks_exact_mut(4) {
                px[3] = 0.25;
            }
            let mut json =
                std::string::String::from("{\"x\":0,\"y\":0,\"canvasWidth\":4,\"canvasHeight\":4,");
            json.push('"');
            json.push_str(effect.name);
            json.push_str("_preserve_alpha\":true}");
            let mut job = Job {
                px: &mut pixels,
                width: 4,
                height: 4,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json: json.as_bytes(),
            };
            assert_eq!((effect.render)(&mut job), 0, "{}", effect.name);
            assert!(
                job.px.chunks_exact(4).all(|p| p[0].is_finite()
                    && p[1].is_finite()
                    && p[2].is_finite()
                    && (p[3] - 0.25).abs() < 0.001),
                "{}",
                effect.name
            );

            let mut opaque = [0.0_f32; 4 * 4 * 4];
            for i in (3..opaque.len()).step_by(4) {
                opaque[i] = 0.25;
            }
            let full_canvas =
                std::string::String::from("{\"x\":0,\"y\":0,\"canvasWidth\":4,\"canvasHeight\":4}");
            let mut job = Job {
                px: &mut opaque,
                width: 4,
                height: 4,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json: full_canvas.as_bytes(),
            };
            assert_eq!((effect.render)(&mut job), 0, "{}", effect.name);
            assert!(
                job.px.chunks_exact(4).all(|p| (p[3] - 1.0).abs() < 0.001),
                "{}",
                effect.name
            );
        }
    }

    #[test]
    fn renderers_convert_grayscale_and_reject_unsupported_modes() {
        let mut gray = [0.0_f32; 16];
        let mut job = Job {
            px: &mut gray,
            width: 4,
            height: 4,
            channels: 1,
            format: 8 | (1 << 16),
            json: b"{\"canvasWidth\":4,\"canvasHeight\":4}",
        };
        for effect in EFFECTS {
            job.px.fill(0.0);
            assert_eq!((effect.render)(&mut job), 0, "{}", effect.name);
            assert!(
                job.px
                    .iter()
                    .all(|p| p.is_finite() && (0.0..=1.0).contains(p))
            );
        }

        let mut cmyk = [0.0_f32; 4 * 4 * 4];
        let mut unsupported = Job {
            px: &mut cmyk,
            width: 4,
            height: 4,
            channels: 4,
            format: 8 | (4 << 16),
            json: b"{}",
        };
        for effect in EFFECTS {
            assert_eq!((effect.render)(&mut unsupported), 2, "{}", effect.name);
        }
    }

    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn coat_render(pattern: &str, controls: &str, ox: usize, width: usize) -> std::vec::Vec<f32> {
        let json = std::format!(
            "{{\"canvasWidth\":96,\"canvasHeight\":64,\"x\":{ox},\"y\":0,\"animal_fur_pattern\":\"{pattern}\",\"animal_fur_scale\":20,\"animal_fur_natural_colors\":false{controls}}}"
        );
        let mut pixels = std::vec![0.0_f32; width * 64 * 4];
        for px in pixels.chunks_exact_mut(4) {
            px[3] = 0.37;
        }
        let mut job = Job {
            px: &mut pixels,
            width,
            height: 64,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: json.as_bytes(),
        };
        assert_eq!(animal_fur(&mut job), 0);
        pixels
    }

    #[test]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn animal_patterns_have_distinct_geometry_with_identical_colours() {
        let controls = ",\"animal_fur_natural_colors\":false,\"animal_fur_light\":\"#ffffff\",\"animal_fur_dark\":\"#000000\",\"animal_fur_detail\":0";
        let patterns = ["fur", "giraffe", "leopard", "zebra", "tiger", "cheetah"];
        let renders: std::vec::Vec<_> = patterns
            .iter()
            .map(|p| coat_render(p, controls, 0, 96))
            .collect();
        for (i, first) in renders.iter().enumerate() {
            assert_eq!(*first, coat_render(patterns[i], controls, 0, 96));
            for second in renders.iter().skip(i + 1) {
                let difference: f32 = first
                    .chunks_exact(4)
                    .zip(second.chunks_exact(4))
                    .map(|(a, b)| (a[0] - b[0]).abs())
                    .sum::<f32>()
                    / (96 * 64) as f32;
                assert!(
                    difference > 0.09,
                    "{} geometry differs by only {difference}",
                    patterns[i]
                );
            }
        }
    }

    #[test]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn every_animal_pattern_obeys_coat_controls_and_global_tile_coordinates() {
        for pattern in ["fur", "giraffe", "leopard", "zebra", "tiger", "cheetah"] {
            let baseline = coat_render(pattern, "", 0, 96);
            for control in [
                ",\"animal_fur_direction\":15",
                ",\"animal_fur_detail\":0",
                ",\"animal_fur_seed\":111",
                ",\"animal_fur_scale\":35",
                ",\"animal_fur_light\":\"#ff0011\"",
                ",\"animal_fur_dark\":\"#00ff11\"",
            ] {
                // Use a scale override without duplicate keys, since the parser takes the first key.
                let changed = if control.contains("scale") {
                    let json = std::format!(
                        "{{\"canvasWidth\":96,\"canvasHeight\":64,\"animal_fur_pattern\":\"{pattern}\",\"animal_fur_scale\":35,\"animal_fur_natural_colors\":false}}"
                    );
                    let mut pixels = std::vec![0.0; 96 * 64 * 4];
                    let mut job = Job {
                        px: &mut pixels,
                        width: 96,
                        height: 64,
                        channels: 4,
                        format: 8 | crate::HAS_ALPHA | (3 << 16),
                        json: json.as_bytes(),
                    };
                    assert_eq!(animal_fur(&mut job), 0);
                    pixels
                } else {
                    coat_render(pattern, control, 0, 96)
                };
                assert_ne!(baseline, changed, "{pattern}: {control}");
            }
            let preserved = coat_render(pattern, ",\"animal_fur_preserve_alpha\":true", 0, 96);
            assert!(preserved.chunks_exact(4).all(|p| p[3] == 0.37));
            assert!(baseline.chunks_exact(4).all(|p| p[3] == 1.0));
            let tile = coat_render(pattern, "", 31, 27);
            for row in 0..64 {
                assert_eq!(
                    &tile[row * 27 * 4..(row + 1) * 27 * 4],
                    &baseline[(row * 96 + 31) * 4..(row * 96 + 58) * 4],
                    "{pattern} row {row}"
                );
            }
        }
    }

    #[test]
    fn texture_noise_variants_render_different_fields() {
        let mut ridged = [0.0_f32; 32 * 32 * 3];
        let mut cellular = [0.0_f32; 32 * 32 * 3];
        let mut first = Job {
            px: &mut ridged,
            width: 32,
            height: 32,
            channels: 3,
            format: 8 | (3 << 16),
            json: b"{\"canvasWidth\":32,\"canvasHeight\":32,\"texture_noise_variant\":\"ridged\"}",
        };
        let mut second = Job {
            px: &mut cellular,
            width: 32,
            height: 32,
            channels: 3,
            format: 8 | (3 << 16),
            json:
                b"{\"canvasWidth\":32,\"canvasHeight\":32,\"texture_noise_variant\":\"cellular\"}",
        };
        assert_eq!((EFFECTS[9].render)(&mut first), 0);
        assert_eq!((EFFECTS[9].render)(&mut second), 0);
        assert_ne!(first.px, second.px);
    }
}

#[derive(Clone, Copy)]
struct Config {
    scale: f32,
    detail: f32,
    seed: u32,
    direction: f32,
    a: [f32; 3],
    b: [f32; 3],
    preserve: bool,
    variant: u8,
}

/// Concatenate a fixed prefix and suffix into a stack buffer for the shared JSON readers.
fn key<'a>(buf: &'a mut [u8; 64], prefix: &str, suffix: &str) -> &'a [u8] {
    let mut n = 0;
    for part in [prefix.as_bytes(), b"_", suffix.as_bytes()] {
        let end = n + part.len();
        buf[n..end].copy_from_slice(part);
        n = end;
    }
    &buf[..n]
}

pub(super) fn read_num(job: &Job, prefix: &str, suffix: &str, default: f32) -> f32 {
    let mut k = [0; 64];
    crate::find_value(job.json, key(&mut k, prefix, suffix)).map_or(default, crate::parse_num)
}

pub(super) fn read_bool(job: &Job, prefix: &str, suffix: &str, default: bool) -> bool {
    let mut k = [0; 64];
    crate::find_value(job.json, key(&mut k, prefix, suffix))
        .map_or(default, |v| v.starts_with(b"true"))
}

pub(super) fn read_colour(job: &Job, prefix: &str, suffix: &str, default: [f32; 3]) -> [f32; 3] {
    let mut k = [0; 64];
    let Some(v) = crate::find_value(job.json, key(&mut k, prefix, suffix)) else {
        return default;
    };
    let s = crate::parse_str(v);
    if s.len() != 7 || s[0] != b'#' {
        return default;
    }
    let mut rgb = [0.0; 3];
    for i in 0..3 {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        };
        let (Some(hi), Some(lo)) = (digit(s[1 + i * 2]), digit(s[2 + i * 2])) else {
            return default;
        };
        rgb[i] = (u32::from(hi) * 16 + u32::from(lo)) as f32 / 255.0;
    }
    rgb
}

#[allow(clippy::too_many_arguments)]
fn config(
    job: &Job,
    prefix: &str,
    a_key: &str,
    b_key: &str,
    def_scale: f32,
    def_detail: f32,
    seed: u32,
    default_a: [f32; 3],
    default_b: [f32; 3],
    direction: f32,
) -> Config {
    Config {
        scale: read_num(job, prefix, "scale", def_scale).max(1.0),
        detail: (read_num(job, prefix, "detail", def_detail) / 100.0).clamp(0.0, 1.0),
        seed: read_num(job, prefix, "seed", seed as f32).max(0.0) as u32,
        direction: read_num(job, prefix, "direction", direction),
        a: read_colour(job, prefix, a_key, default_a),
        b: read_colour(job, prefix, b_key, default_b),
        preserve: read_bool(job, prefix, "preserve_alpha", false),
        variant: if prefix == "animal_fur" {
            match crate::find_value(job.json, b"animal_fur_pattern").map(crate::parse_str) {
                Some(b"giraffe") => 1,
                Some(b"leopard") => 2,
                Some(b"zebra") => 3,
                Some(b"tiger") => 4,
                Some(b"cheetah") => 5,
                _ => 0,
            }
        } else if prefix == "texture_noise"
            && crate::find_value(job.json, b"texture_noise_variant")
                .is_none_or(|v| crate::parse_str(v) == b"cellular")
        {
            1
        } else {
            0
        },
    }
}

fn animal_fur(job: &mut Job) -> i32 {
    super::fur::render(job)
}
fn brick_wall(job: &mut Job) -> i32 {
    run(job, 1)
}
fn brushed_metal(job: &mut Job) -> i32 {
    run(job, 2)
}
fn clouds(job: &mut Job) -> i32 {
    run(job, 3)
}
fn diamond_plate(job: &mut Job) -> i32 {
    super::treadplate::render(job)
}
fn marble(job: &mut Job) -> i32 {
    run(job, 5)
}
fn reptile_skin(job: &mut Job) -> i32 {
    run(job, 6)
}
fn rust(job: &mut Job) -> i32 {
    run(job, 7)
}
fn stone_wall(job: &mut Job) -> i32 {
    run(job, 8)
}
fn texture_noise(job: &mut Job) -> i32 {
    run(job, 9)
}
fn weave(job: &mut Job) -> i32 {
    run(job, 10)
}
fn wood(job: &mut Job) -> i32 {
    run(job, 11)
}

fn run(job: &mut Job, kind: u8) -> i32 {
    if !supported(job) {
        return 2;
    }
    let (prefix, ak, bk, scale, detail, seed, a, b, direction) = match kind {
        0 => (
            "animal_fur",
            "light",
            "dark",
            38.0,
            58.0,
            7,
            [0.84, 0.68, 0.26],
            [0.14, 0.09, 0.04],
            90.0,
        ),
        1 => (
            "brick_wall",
            "brick",
            "mortar",
            52.0,
            50.0,
            11,
            [0.61, 0.28, 0.19],
            [0.78, 0.77, 0.72],
            0.0,
        ),
        2 => (
            "brushed_metal",
            "colour",
            "colour",
            13.0,
            45.0,
            13,
            [0.53, 0.59, 0.61],
            [0.53, 0.59, 0.61],
            0.0,
        ),
        3 => (
            "clouds",
            "light",
            "dark",
            90.0,
            65.0,
            17,
            [0.93, 0.96, 1.0],
            [0.13, 0.28, 0.47],
            0.0,
        ),
        4 => (
            "diamond_plate",
            "colour",
            "colour",
            34.0,
            55.0,
            19,
            [0.61, 0.65, 0.66],
            [0.61, 0.65, 0.66],
            0.0,
        ),
        5 => (
            "marble",
            "light",
            "dark",
            88.0,
            60.0,
            23,
            [0.91, 0.86, 0.79],
            [0.47, 0.35, 0.33],
            0.0,
        ),
        6 => (
            "reptile_skin",
            "light",
            "dark",
            37.0,
            60.0,
            29,
            [0.74, 0.5, 0.23],
            [0.2, 0.12, 0.07],
            0.0,
        ),
        7 => (
            "rust",
            "light",
            "dark",
            55.0,
            62.0,
            31,
            [0.79, 0.48, 0.22],
            [0.22, 0.27, 0.28],
            0.0,
        ),
        8 => (
            "stone_wall",
            "stone",
            "mortar",
            67.0,
            56.0,
            37,
            [0.66, 0.65, 0.6],
            [0.32, 0.31, 0.28],
            0.0,
        ),
        9 => (
            "texture_noise",
            "light",
            "dark",
            23.0,
            55.0,
            41,
            [0.91, 0.84, 0.74],
            [0.22, 0.15, 0.25],
            0.0,
        ),
        10 => (
            "weave",
            "light",
            "dark",
            18.0,
            48.0,
            43,
            [0.22, 0.39, 0.53],
            [0.08, 0.17, 0.27],
            0.0,
        ),
        _ => (
            "wood",
            "light",
            "dark",
            76.0,
            56.0,
            47,
            [0.71, 0.39, 0.18],
            [0.21, 0.09, 0.05],
            0.0,
        ),
    };
    let c = config(job, prefix, ak, bk, scale, detail, seed, a, b, direction);
    let (ox, oy, cw, ch) = canvas(job);
    let factor = adaptive_scale(job, prefix);
    let ang = c.direction * core::f32::consts::PI / 180.0;
    let (ca, sa) = (cosf(ang), sinf(ang));
    let w = job.width;
    let channels = job.channels;
    let has_alpha = job.format & crate::HAS_ALPHA != 0 && channels > 1;
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let px = ox + x as f32 + 0.5;
            let py = oy + y as f32 + 0.5;
            let (u, v) = (
                (px - cw * 0.5) * ca + (py - ch * 0.5) * sa,
                -(px - cw * 0.5) * sa + (py - ch * 0.5) * ca,
            );
            let (r, g, b) = sample(kind, u / factor, v / factor, px / factor, py / factor, &c);
            let i = (y * w + x) * channels;
            let alpha = if c.preserve && has_alpha {
                job.px[i + channels - 1]
            } else {
                1.0
            };
            write(job, y * w + x, [r, g, b, alpha]);
        }
    }
    0
}

/// Species fur is dispatched separately; other materials share the original controls.
fn sample(kind: u8, u: f32, v: f32, x: f32, y: f32, c: &Config) -> (f32, f32, f32) {
    let rgb = super::texture_materials::sample(
        kind, u, v, x, y, c.scale, c.detail, c.seed, c.a, c.b, c.variant,
    )
    .unwrap_or(c.a);
    (rgb[0], rgb[1], rgb[2])
}

/// Fixed units are pixels. Adaptive texture units are relative to a 512px
/// short canvas edge, using full-canvas metadata so every processing band agrees.
pub(super) fn adaptive_scale(job: &Job, prefix: &str) -> f32 {
    let mut k = [0; 64];
    if crate::find_value(job.json, key(&mut k, prefix, "scaling")).map(crate::parse_str)
        == Some(b"adaptive")
    {
        let (_, _, w, h) = canvas(job);
        (w.min(h) / 512.0).clamp(1.0 / 512.0, 128.0)
    } else {
        1.0
    }
}

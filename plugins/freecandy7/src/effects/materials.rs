//! Clean-room object and surface treatments. Each effect uses only the current input layer and
//! absolute canvas coordinates, so output remains deterministic across PhotoCraft bands.

use super::Effect;
use super::common::{
    Distance, Surface, VerticalEdges, canvas, fbm, hash, mix, over, smoothstep, supported, write,
};
use crate::{Job, colour, num};
use libm::{ceilf, cosf, floorf, sinf, sqrtf};

pub const EFFECTS: &[Effect] = &[
    Effect {
        name: "bevel",
        params: BEVEL_PARAMS,
        render: bevel,
    },
    Effect {
        name: "chrome",
        params: CHROME_PARAMS,
        render: chrome,
    },
    Effect {
        name: "glass",
        params: GLASS_PARAMS,
        render: glass,
    },
    Effect {
        name: "extrude",
        params: EXTRUDE_PARAMS,
        render: extrude,
    },
    Effect {
        name: "shadow",
        params: SHADOW_PARAMS,
        render: shadow,
    },
    Effect {
        name: "motion_trail",
        params: MOTION_TRAIL_PARAMS,
        render: motion_trail,
    },
    Effect {
        name: "water_drops",
        params: WATER_DROPS_PARAMS,
        render: water_drops,
    },
    Effect {
        name: "drip",
        params: DRIP_PARAMS,
        render: drip,
    },
    Effect {
        name: "snow",
        params: SNOW_PARAMS,
        render: snow,
    },
    Effect {
        name: "icicles",
        params: ICICLES_PARAMS,
        render: icicles,
    },
];

const BEVEL_PARAMS: &str = r##"    "bevel_size": {"type":"number","min":1,"max":500,"default":8,"group":"Bevel","label":"Size","visibleWhen":{"effect":"bevel"}},
    "bevel_depth": {"type":"number","min":0,"max":100,"default":58,"group":"Bevel","label":"Depth","visibleWhen":{"effect":"bevel"}},
    "bevel_angle": {"type":"angle","default":315,"group":"Bevel","label":"Angle","visibleWhen":{"effect":"bevel"}},
    "bevel_highlight": {"type":"color","default":"#fff5d6","group":"Bevel","label":"Highlight","visibleWhen":{"effect":"bevel"}},
    "bevel_shadow": {"type":"color","default":"#263246","group":"Bevel","label":"Shadow","visibleWhen":{"effect":"bevel"}}"##;
const CHROME_PARAMS: &str = r##"    "chrome_scale": {"type":"number","min":12,"max":80,"default":34,"group":"Chrome","label":"Scale","visibleWhen":{"effect":"chrome"}},
    "chrome_strength": {"type":"number","min":0,"max":100,"default":78,"group":"Chrome","label":"Strength","visibleWhen":{"effect":"chrome"}},
    "chrome_tint": {"type":"color","default":"#bad2dc","group":"Chrome","label":"Tint","visibleWhen":{"effect":"chrome"}},
    "chrome_angle": {"type":"angle","default":35,"group":"Chrome","label":"Angle","visibleWhen":{"effect":"chrome"}}"##;
const GLASS_PARAMS: &str = r##"    "glass_refraction": {"type":"number","min":0,"max":500,"default":8,"group":"Glass","label":"Refraction","visibleWhen":{"effect":"glass"}},
    "glass_opacity": {"type":"number","min":0,"max":100,"default":68,"group":"Glass","label":"Opacity","visibleWhen":{"effect":"glass"}},
    "glass_tint": {"type":"color","default":"#8ddfff","group":"Glass","label":"Tint","visibleWhen":{"effect":"glass"}},
    "glass_highlight": {"type":"number","min":0,"max":100,"default":68,"group":"Glass","label":"Highlight","visibleWhen":{"effect":"glass"}}"##;
const EXTRUDE_PARAMS: &str = r##"    "extrude_length": {"type":"number","min":1,"max":1500,"default":18,"group":"Extrude","label":"Length","visibleWhen":{"effect":"extrude"}},
    "extrude_angle": {"type":"angle","default":45,"group":"Extrude","label":"Angle","visibleWhen":{"effect":"extrude"}},
    "extrude_depth": {"type":"int","min":2,"max":16,"default":9,"group":"Extrude","label":"Depth","visibleWhen":{"effect":"extrude"}},
    "extrude_colour": {"type":"color","default":"#35485e","group":"Extrude","label":"Colour","visibleWhen":{"effect":"extrude"}}"##;
const SHADOW_PARAMS: &str = r##"    "shadow_distance": {"type":"number","min":0,"max":1500,"default":15,"group":"Shadow","label":"Distance","visibleWhen":{"effect":"shadow"}},
    "shadow_angle": {"type":"angle","default":135,"group":"Shadow","label":"Angle","visibleWhen":{"effect":"shadow"}},
    "shadow_softness": {"type":"number","min":0,"max":12,"default":5,"group":"Shadow","label":"Softness","visibleWhen":{"effect":"shadow"}},
    "shadow_opacity": {"type":"number","min":0,"max":100,"default":58,"group":"Shadow","label":"Opacity","visibleWhen":{"effect":"shadow"}},
    "shadow_colour": {"type":"color","default":"#111722","group":"Shadow","label":"Colour","visibleWhen":{"effect":"shadow"}}"##;
const MOTION_TRAIL_PARAMS: &str = r##"    "motion_trail_length": {"type":"number","min":2,"max":1500,"default":22,"group":"Motion Trail","label":"Length","visibleWhen":{"effect":"motion_trail"}},
    "motion_trail_angle": {"type":"angle","default":210,"group":"Motion Trail","label":"Angle","visibleWhen":{"effect":"motion_trail"}},
    "motion_trail_steps": {"type":"int","min":2,"max":16,"default":9,"group":"Motion Trail","label":"Steps","visibleWhen":{"effect":"motion_trail"}},
    "motion_trail_colour": {"type":"color","default":"#39c8ff","group":"Motion Trail","label":"Colour","visibleWhen":{"effect":"motion_trail"}},
    "motion_trail_opacity": {"type":"number","min":0,"max":100,"default":55,"group":"Motion Trail","label":"Opacity","visibleWhen":{"effect":"motion_trail"}}"##;
const WATER_DROPS_PARAMS: &str = r##"    "water_drops_scale": {"type":"number","min":14,"max":1500,"default":32,"group":"Water Drops","label":"Scale","visibleWhen":{"effect":"water_drops"}},
    "water_drops_amount": {"type":"number","min":5,"max":100,"default":62,"group":"Water Drops","label":"Amount","visibleWhen":{"effect":"water_drops"}},
    "water_drops_seed": {"type":"int","min":0,"max":9999,"default":53,"group":"Water Drops","label":"Random seed","visibleWhen":{"effect":"water_drops"}},
    "water_drops_highlight": {"type":"color","default":"#f5ffff","group":"Water Drops","label":"Highlight","visibleWhen":{"effect":"water_drops"}}"##;
const DRIP_PARAMS: &str = r##"    "drip_length": {"type":"number","min":2,"max":150,"default":18,"group":"Drip","label":"Length","visibleWhen":{"effect":"drip"}},
    "drip_size": {"type":"number","min":25,"max":1000,"default":100,"group":"Drip","label":"Size (%)","visibleWhen":{"effect":"drip"}},
    "drip_size_jitter": {"type":"number","min":0,"max":100,"default":100,"group":"Drip","label":"Size jitter (%)","visibleWhen":{"effect":"drip"}},
    "drip_amount": {"type":"number","min":0,"max":100,"default":55,"group":"Drip","label":"Amount","visibleWhen":{"effect":"drip"}},
    "drip_colour": {"type":"color","default":"#55ca7a","group":"Drip","label":"Colour","visibleWhen":{"effect":"drip"}},
    "drip_seed": {"type":"int","min":0,"max":9999,"default":59,"group":"Drip","label":"Random seed","visibleWhen":{"effect":"drip"}}"##;
const SNOW_PARAMS: &str = r##"    "snow_depth": {"type":"number","min":1,"max":500,"default":14,"group":"Snow","label":"Depth","visibleWhen":{"effect":"snow"}},
    "snow_amount": {"type":"number","min":0,"max":100,"default":72,"group":"Snow","label":"Amount","visibleWhen":{"effect":"snow"}},
    "snow_cool": {"type":"color","default":"#d9f2ff","group":"Snow","label":"Snow colour","visibleWhen":{"effect":"snow"}},
    "snow_seed": {"type":"int","min":0,"max":9999,"default":61,"group":"Snow","label":"Random seed","visibleWhen":{"effect":"snow"}}"##;
const ICICLES_PARAMS: &str = r##"    "icicles_length": {"type":"number","min":2,"max":150,"default":32,"group":"Icicles","label":"Length","visibleWhen":{"effect":"icicles"}},
    "icicles_size": {"type":"number","min":25,"max":1000,"default":100,"group":"Icicles","label":"Size (%)","visibleWhen":{"effect":"icicles"}},
    "icicles_size_jitter": {"type":"number","min":0,"max":100,"default":100,"group":"Icicles","label":"Size jitter (%)","visibleWhen":{"effect":"icicles"}},
    "icicles_density": {"type":"number","min":5,"max":100,"default":66,"group":"Icicles","label":"Density","visibleWhen":{"effect":"icicles"}},
    "icicles_tint": {"type":"color","default":"#bceeff","group":"Icicles","label":"Tint","visibleWhen":{"effect":"icicles"}},
    "icicles_seed": {"type":"int","min":0,"max":9999,"default":67,"group":"Icicles","label":"Random seed","visibleWhen":{"effect":"icicles"}}"##;

fn setup(job: &Job) -> Option<(Surface, f32, f32, f32, f32)> {
    if !supported(job) {
        return None;
    }
    let (ox, oy, cw, ch) = canvas(job);
    Some((Surface::new(job)?, ox, oy, cw, ch))
}

fn tint(json: &[u8], name: &str, fallback: [f32; 3]) -> [f32; 3] {
    colour(json, name, fallback)
}
fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a.clamp(0.0, 1.0)]
}
fn put(job: &mut Job, i: usize, p: [f32; 4]) {
    write(job, i, p);
}
fn deg(a: f32) -> f32 {
    a * (core::f32::consts::PI / 180.0)
}

/// Rounded cross-section, viewed orthographically. The silhouette normal tilts toward the
/// camera as the surface approaches its flat face; this makes light follow the object.
fn rounded_normal(d: &Distance, x: i32, y: i32, radius: f32) -> [f32; 3] {
    // A wide Sobel derivative filters the pixel staircase before reflection magnifies it.
    // Keep its magnitude: at a thin stroke's medial ridge the two slopes cancel smoothly.
    let dx = (2.0 * (d.at(x - 3, y) - d.at(x + 3, y)) + d.at(x - 3, y - 2) - d.at(x + 3, y - 2)
        + d.at(x - 3, y + 2)
        - d.at(x + 3, y + 2))
        / 24.0;
    let dy = (2.0 * (d.at(x, y - 3) - d.at(x, y + 3)) + d.at(x - 2, y - 3) - d.at(x - 2, y + 3)
        + d.at(x + 2, y - 3)
        - d.at(x + 2, y + 3))
        / 24.0;
    let magnitude = sqrtf(dx * dx + dy * dy);
    let dist = d.at(x, y).max(0.0);
    let radius = radius.clamp(1.0, 500.0);
    let mut local_radius = dist;
    // Adapt the crown to a thin stroke's thickness. A fixed large radius leaves two
    // opposing planes at its center; finding the nearby medial crest makes a smooth dome.
    // Preserve the original 2px probes for small radii. Larger crowns use growing
    // intervals, keeping work bounded while still resolving thin strokes first.
    if dist < radius && magnitude > 0.01 {
        let mut travel = 2.0;
        for _ in 0..32 {
            if travel > radius {
                break;
            }
            let sx = libm::roundf(x as f32 - dx / magnitude * travel) as i32;
            let sy = libm::roundf(y as f32 - dy / magnitude * travel) as i32;
            let probe = d.at(sx, sy);
            if probe + 0.5 < local_radius {
                break;
            }
            local_radius = local_radius.max(probe).min(radius);
            if local_radius >= radius {
                break;
            }
            travel += if radius <= 28.0 {
                2.0
            } else {
                (travel * 0.2).max(2.0)
            };
        }
    }
    let side = (1.0 - dist / local_radius.max(1.0)).clamp(0.0, 1.0);
    let gain = side / magnitude.max(1.0);
    let nx = dx * gain;
    let ny = dy * gain;
    [nx, ny, sqrtf((1.0 - nx * nx - ny * ny).max(0.0))]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn power8(x: f32) -> f32 {
    let x = x.max(0.0);
    let q = x * x;
    let q = q * q;
    q * q
}
fn fresnel(nz: f32, base: f32) -> f32 {
    let g = (1.0 - nz).clamp(0.0, 1.0);
    base + (1.0 - base) * g * g * g * g * g
}
/// A bounded, analytic studio environment: sky, warm floor, a dark horizon and softboxes.
/// Reflections are driven by the surface normal rather than repeated sine-wave stripes.
fn studio(n: [f32; 3], u: f32, v: f32) -> [f32; 3] {
    let rx = 2.0 * n[0] * n[2];
    let ry = 2.0 * n[1] * n[2];
    let h = ry + v * 0.055;
    let sky = 1.0 - smoothstep(-0.22, 0.06, h);
    let floor = smoothstep(0.07, 0.48, h);
    let box_light = (1.0 - smoothstep(0.12, 0.23, (rx + 0.3 + u * 0.025).abs()))
        * (1.0 - smoothstep(0.44, 0.64, (ry + 0.38).abs()));
    let strip = (1.0 - smoothstep(0.015, 0.085, (ry + 0.55 + rx * 0.16).abs())) * 0.85;
    let mut out = [0.0; 3];
    let sky_c = [0.81, 0.9, 1.0];
    let floor_c = [0.79, 0.74, 0.67];
    let sky_value = 0.43 + 0.57 * (-h).clamp(0.0, 1.0);
    let floor_value = 0.24 + 0.65 * h.clamp(0.0, 1.0);
    for c in 0..3 {
        out[c] =
            (0.045 + sky * sky_c[c] * sky_value + floor * floor_c[c] * floor_value).clamp(0.0, 1.0);
        out[c] = mix(out[c], 1.0, box_light.max(strip).clamp(0.0, 1.0));
    }
    out
}

fn bevel(job: &mut Job) -> i32 {
    let Some((src, _, _, _, _)) = setup(job) else {
        return 1;
    };
    let size = num(job.json, "bevel_size", 8.0).clamp(1.0, 500.0);
    let depth = num(job.json, "bevel_depth", 58.0).clamp(0.0, 100.0) / 100.0;
    if depth == 0.0 {
        return 0;
    }
    let a = deg(num(job.json, "bevel_angle", 315.0));
    let light = [cosf(a) * 0.7, sinf(a) * 0.7, 0.714];
    let half = [light[0] * 0.54, light[1] * 0.54, 0.925];
    let hi = tint(job.json, "bevel_highlight", [1.0, 0.96, 0.84]);
    let lo = tint(job.json, "bevel_shadow", [0.15, 0.2, 0.28]);
    let Some(d) = Distance::with_limit(&src, size.max(64.0)) else {
        return 1;
    };
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let mut p = src.sample(x as f32, y as f32);
            let dist = d.at(x as i32, y as i32);
            if p[3] > 0.0 && dist >= 0.0 && dist < size {
                let n = rounded_normal(&d, x as i32, y as i32, size);
                let edge = 1.0 - smoothstep(size * 0.8, size, dist);
                let diffuse = dot(n, light).max(0.0);
                let spec = power8(dot(n, half)) * power8(dot(n, half));
                let shade = ((0.714 - diffuse) * 1.15).max(0.0) * depth * edge;
                let highlight = ((diffuse - 0.714).max(0.0) * 0.7 + spec * 0.7) * depth * edge;
                for c in 0..3 {
                    p[c] = mix(p[c], p[c] * (0.23 + lo[c] * 0.5), shade.clamp(0.0, 0.88));
                    p[c] = mix(p[c], hi[c], highlight.clamp(0.0, 0.95));
                }
            }
            put(job, y * job.width + x, p);
        }
    }
    0
}

/// A high-contrast studio reflected by polished metal. Large luminous panels and dark
/// flags give chrome its characteristic sharp value transitions, including on its face.
fn chrome_studio(n: [f32; 3]) -> [f32; 3] {
    let rx = 2.0 * n[0] * n[2];
    let ry = 2.0 * n[1] * n[2];
    let sky = 1.0 - smoothstep(-0.08, 0.04, ry);
    let floor = smoothstep(0.15, 0.24, ry);
    let sky_value = 0.52 + 0.34 * (-ry).clamp(0.0, 1.0);
    let floor_value = 0.80 - 0.45 * ry.clamp(0.0, 1.0);
    let strip = 1.0 - smoothstep(0.045, 0.115, (ry + 0.30 + rx * 0.14).abs());
    let panel = (1.0 - smoothstep(0.09, 0.20, (rx - 0.48).abs()))
        * (1.0 - smoothstep(0.42, 0.68, (ry - 0.20).abs()));
    let flag = (1.0 - smoothstep(0.035, 0.10, (rx + 0.48 + ry * 0.18).abs()))
        * (1.0 - smoothstep(0.45, 0.80, ry.abs()));
    let mut out = [0.0; 3];
    for c in 0..3 {
        let sky_tint = [0.91, 0.96, 1.0][c];
        let floor_tint = [1.0, 0.97, 0.91][c];
        let base = 0.025 + sky * sky_value * sky_tint + floor * floor_value * floor_tint;
        out[c] = mix(mix(base, 0.035, flag), 1.0, strip.max(panel));
    }
    out
}

fn chrome(job: &mut Job) -> i32 {
    let Some((src, ox, oy, cw, ch)) = setup(job) else {
        return 1;
    };
    let scale = num(job.json, "chrome_scale", 34.0).clamp(12.0, 80.0);
    let shape_x = num(job.json, "shapeX", 0.0);
    let shape_y = num(job.json, "shapeY", 0.0);
    let shape_width = num(job.json, "shapeWidth", cw).max(1.0);
    let shape_height = num(job.json, "shapeHeight", ch).max(1.0);
    let strength = num(job.json, "chrome_strength", 78.0).clamp(0.0, 100.0) / 100.0;
    if strength == 0.0 {
        return 0;
    }
    let tint = tint(job.json, "chrome_tint", [0.73, 0.82, 0.86]);
    let a = deg(num(job.json, "chrome_angle", 35.0));
    let (ca, sa) = (cosf(a), sinf(a));
    let Some(d) = Distance::new(&src) else {
        return 1;
    };
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let mut p = src.sample(x as f32, y as f32);
            if p[3] > 0.0 && d.at(x as i32, y as i32) >= 0.0 {
                let mut n = rounded_normal(&d, x as i32, y as i32, scale * 0.34 + 0.8);
                let u = (ox + x as f32 + 0.5 - shape_x) / shape_width * 2.0 - 1.0;
                let v = (oy + y as f32 + 0.5 - shape_y) / shape_height * 2.0 - 1.0;
                let luma = |q: [f32; 4]| {
                    let q = if q[3] > 0.01 { q } else { p };
                    q[0] * 0.2126 + q[1] * 0.7152 + q[2] * 0.0722
                };
                let dx = luma(src.sample(x as f32 - 3.0, y as f32))
                    - luma(src.sample(x as f32 + 3.0, y as f32));
                let dy = luma(src.sample(x as f32, y as f32 - 3.0))
                    - luma(src.sample(x as f32, y as f32 + 3.0));
                // A shallow crown gives a selected photograph a reflective face. Source
                // luminance bends that reflection locally instead of surviving as a dark
                // colour overlay. The fixed 3px probes are safe within the host halo.
                n[0] += n[2] * (u * 0.18 + dx * 0.65);
                n[1] += n[2] * (v * (0.33 + scale * 0.006) + dy * 0.65);
                n[2] = n[2].max(0.08);
                let length = sqrtf(dot(n, n)).max(0.001);
                n = [n[0] / length, n[1] / length, n[2] / length];
                let (nx, ny) = (n[0] * ca - n[1] * sa, n[0] * sa + n[1] * ca);
                n[0] = nx;
                n[1] = ny;
                let env = chrome_studio(n);
                let reflectance = fresnel(n[2], 0.97);
                for c in 0..3 {
                    // Retain a tint in midtones while letting the studio lights stay
                    // almost white. Suppressing those highlights made the old shader matte.
                    let metal =
                        env[c] * (1.0 - (1.0 - tint[c]) * 0.35 * (1.0 - env[c])) * reflectance;
                    p[c] = mix(p[c], metal.clamp(0.0, 1.0), strength);
                }
            }
            put(job, y * job.width + x, p);
        }
    }
    0
}

fn glass(job: &mut Job) -> i32 {
    let Some((src, ox, oy, cw, ch)) = setup(job) else {
        return 1;
    };
    let bend = num(job.json, "glass_refraction", 8.0).clamp(0.0, 500.0);
    let opacity = num(job.json, "glass_opacity", 68.0).clamp(0.0, 100.0) / 100.0;
    let shine = num(job.json, "glass_highlight", 68.0).clamp(0.0, 100.0) / 100.0;
    let tint = tint(job.json, "glass_tint", [0.55, 0.87, 1.0]);
    if bend == 0.0 && opacity == 0.0 && shine == 0.0 {
        return 0;
    }
    let Some(d) = Distance::with_limit(&src, (7.0 + bend * 0.45).max(64.0)) else {
        return 1;
    };
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            let mut p = base;
            let dist = d.at(x as i32, y as i32);
            if base[3] > 0.0 && dist >= 0.0 {
                let n = rounded_normal(&d, x as i32, y as i32, 7.0 + bend * 0.45);
                // Sample inward: the plug-in only receives this layer, never a hidden background.
                let refr = src.sample(x as f32 - n[0] * bend * 0.7, y as f32 - n[1] * bend * 0.7);
                let env = studio(n, (ox + x as f32) / cw, (oy + y as f32) / ch);
                let f = fresnel(n[2], 0.045);
                let spec =
                    power8(dot(n, [-0.31, -0.43, 0.848])) * power8(dot(n, [-0.31, -0.43, 0.848]));
                let reflected_y = 2.0 * n[1] * n[2];
                let reflected_x = 2.0 * n[0] * n[2];
                let softbox = (1.0
                    - smoothstep(0.03, 0.13, (reflected_y + 0.66 + reflected_x * 0.16).abs()))
                    * (-n[1] * 0.8 - n[0] * 0.2).max(0.0);
                let spec = spec.max(softbox * 1.8).clamp(0.0, 1.0);
                let thickness = 0.13 + (1.0 - n[2]) * 0.32;
                let rim = (1.0 - smoothstep(0.3, 1.8, dist)) * 0.25;
                for c in 0..3 {
                    let transmitted = mix(base[c], refr[c], refr[3])
                        * (1.0 - opacity * thickness * (1.0 - tint[c]));
                    let reflected = mix(transmitted, env[c], f * 0.8 * shine);
                    let reflected = mix(reflected, 1.0, spec * 0.9 * shine);
                    p[c] = mix(
                        base[c],
                        (reflected + rim * shine * tint[c]).clamp(0.0, 1.0),
                        opacity.max(shine),
                    );
                }
            }
            put(job, y * job.width + x, p);
        }
    }
    0
}

fn extrude(job: &mut Job) -> i32 {
    let Some((src, _, _, _, _)) = setup(job) else {
        return 1;
    };
    let length = num(job.json, "extrude_length", 18.0).clamp(1.0, 1500.0);
    let quality = num(job.json, "extrude_depth", 9.0).clamp(2.0, 16.0) as usize;
    let steps = (libm::ceilf(length * 1.5) as usize).max(quality * 3);
    let a = deg(num(job.json, "extrude_angle", 45.0));
    let (dx, dy) = (cosf(a) * length, sinf(a) * length);
    let shade = tint(job.json, "extrude_colour", [0.21, 0.28, 0.37]);
    if length > 40.0 {
        return extrude_large(job, &src, length, [cosf(a), sinf(a)], shade);
    }
    let Some(d) = Distance::new(&src) else {
        return 1;
    };
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let mut body = [0.0_f32; 4];
            for k in 1..=steps {
                let t = k as f32 / steps as f32;
                let sx = x as f32 - dx * t;
                let sy = y as f32 - dy * t;
                let sample = src.sample(sx, sy);
                if sample[3] > body[3] {
                    let (nx, ny) = d.normal(sx as i32, sy as i32);
                    let light = (nx * -0.55 + ny * -0.72).max(0.0);
                    let illumination = (0.37 + 0.62 * light) * (1.0 - 0.2 * t);
                    for c in 0..3 {
                        body[c] = (mix(shade[c], sample[c], 0.16) * illumination + 0.035 * light)
                            .clamp(0.0, 1.0);
                    }
                    body[3] = sample[3];
                }
                if body[3] >= 0.999 {
                    break;
                }
            }
            put(job, y * job.width + x, over(base, body));
        }
    }
    0
}

/// Bounds of all nonzero alpha, expanded for the bilinear reconstruction footprint.
/// Long rays that cannot meet this rectangle require no distance-field traversal.
fn alpha_bounds(src: &Surface) -> Option<[f32; 4]> {
    let mut left = src.width;
    let mut top = src.height;
    let mut right = 0;
    let mut bottom = 0;
    for y in 0..src.height {
        for x in 0..src.width {
            if src.alpha(x as i32, y as i32) > 0.0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    (left < src.width).then_some([
        left as f32 - 1.0,
        top as f32 - 1.0,
        right as f32 + 1.0,
        bottom as f32 + 1.0,
    ])
}

fn ray_interval(
    x: f32,
    y: f32,
    direction: [f32; 2],
    bounds: [f32; 4],
    length: f32,
) -> Option<(f32, f32)> {
    let mut enter = 0.0_f32;
    let mut exit = length;
    for (origin, delta, low, high) in [
        (x, direction[0], bounds[0], bounds[2]),
        (y, direction[1], bounds[1], bounds[3]),
    ] {
        if delta.abs() < 0.000001 {
            if origin < low || origin > high {
                return None;
            }
        } else {
            let a = (low - origin) / delta;
            let b = (high - origin) / delta;
            enter = enter.max(a.min(b));
            exit = exit.min(a.max(b));
            if enter > exit {
                return None;
            }
        }
    }
    Some((enter, exit))
}

fn extrude_large(
    job: &mut Job,
    src: &Surface,
    length: f32,
    direction: [f32; 2],
    shade: [f32; 3],
) -> i32 {
    let Some(bounds) = alpha_bounds(src) else {
        return 0;
    };
    let (ox, oy, _, _) = canvas(job);
    let bounds = [
        bounds[0] + ox,
        bounds[1] + oy,
        bounds[2] + ox,
        bounds[3] + oy,
    ];
    // Include translucent hairlines in the support field. The 32px saturation
    // bounds context and empty-space jumps, independent of the extrusion length.
    let Some(d) = Distance::with_threshold(src, 32.0, f32::MIN_POSITIVE) else {
        return 1;
    };
    let reverse = [-direction[0], -direction[1]];
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let ax = ox + x as f32;
            let ay = oy + y as f32;
            let Some((enter, exit)) = ray_interval(ax, ay, reverse, bounds, length) else {
                continue;
            };
            let mut travel = (floorf(enter * 2.0) * 0.5).max(0.5);
            let mut body = [0.0_f32; 4];
            while travel <= exit {
                let sx = ax + reverse[0] * travel - ox;
                let sy = ay + reverse[1] * travel - oy;
                let distance = d.at(floorf(sx) as i32, floorf(sy) as i32);
                if distance < -3.5 {
                    // The margin covers the field's half-pixel edge offset,
                    // a diagonal pixel-center error, and the bilinear footprint. Snap jumps so band origins cannot
                    // alter the final half-pixel sampling phase at an edge.
                    travel += (floorf((-distance - 3.5) * 2.0) * 0.5).max(0.5);
                    continue;
                }
                let sample = src.sample(sx, sy);
                if sample[3] > body[3] {
                    let (nx, ny) = d.normal(sx as i32, sy as i32);
                    let light = (nx * -0.55 + ny * -0.72).max(0.0);
                    let illumination = (0.37 + 0.62 * light) * (1.0 - 0.2 * travel / length);
                    for c in 0..3 {
                        body[c] = (mix(shade[c], sample[c], 0.16) * illumination + 0.035 * light)
                            .clamp(0.0, 1.0);
                    }
                    body[3] = sample[3];
                    if body[3] >= 0.999 {
                        break;
                    }
                }
                travel += 0.5;
            }
            put(job, y * job.width + x, over(base, body));
        }
    }
    0
}

fn scalar_sample(buf: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, w as f32 - 1.0);
    let y = y.clamp(0.0, h as f32 - 1.0);
    let ix = x as usize;
    let iy = y as usize;
    let nx = (ix + 1).min(w - 1);
    let ny = (iy + 1).min(h - 1);
    mix(
        mix(buf[iy * w + ix], buf[iy * w + nx], x - ix as f32),
        mix(buf[ny * w + ix], buf[ny * w + nx], x - ix as f32),
        y - iy as f32,
    )
}
fn shadow(job: &mut Job) -> i32 {
    let Some((src, _, _, _, _)) = setup(job) else {
        return 1;
    };
    let distance = num(job.json, "shadow_distance", 15.0).clamp(0.0, 1500.0);
    let softness = num(job.json, "shadow_softness", 5.0).clamp(0.0, 12.0);
    let opacity = num(job.json, "shadow_opacity", 58.0).clamp(0.0, 100.0) / 100.0;
    if opacity == 0.0 {
        return 0;
    }
    let a = deg(num(job.json, "shadow_angle", 135.0));
    let (dx, dy) = (cosf(a) * distance, sinf(a) * distance);
    let colour = tint(job.json, "shadow_colour", [0.07, 0.09, 0.13]);
    let count = job.width * job.height;
    let Some(horizontal) = crate::alloc::<f32>(count) else {
        return 1;
    };
    let Some(blurred) = crate::alloc::<f32>(count) else {
        return 1;
    };
    let radius = libm::ceilf(softness * 1.5) as i32;
    let sigma = (softness * 0.57).max(0.1);
    let mut weights = [0.0; 37];
    let mut total = 0.0;
    for k in -radius..=radius {
        let w = libm::expf(-(k * k) as f32 / (2.0 * sigma * sigma));
        weights[(k + radius) as usize] = w;
        total += w;
    }
    for y in 0..job.height {
        for x in 0..job.width {
            let mut sum = 0.0;
            for k in -radius..=radius {
                sum += src.alpha(x as i32 + k, y as i32) * weights[(k + radius) as usize];
            }
            horizontal[y * job.width + x] = sum / total;
        }
    }
    for y in 0..job.height {
        for x in 0..job.width {
            let mut sum = 0.0;
            for k in -radius..=radius {
                let sy = (y as i32 + k).clamp(0, job.height as i32 - 1) as usize;
                sum += horizontal[sy * job.width + x] * weights[(k + radius) as usize];
            }
            blurred[y * job.width + x] = sum / total;
        }
    }
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let alpha = scalar_sample(blurred, job.width, job.height, x as f32 - dx, y as f32 - dy);
            put(
                job,
                y * job.width + x,
                over(base, rgba(colour, alpha * opacity)),
            );
        }
    }
    0
}

fn motion_trail(job: &mut Job) -> i32 {
    let Some((src, _, _, _, _)) = setup(job) else {
        return 1;
    };
    let length = num(job.json, "motion_trail_length", 22.0).clamp(2.0, 1500.0);
    let steps = num(job.json, "motion_trail_steps", 9.0).clamp(2.0, 16.0) as usize;
    let samples = (libm::ceilf(length * 1.5) as usize).max(steps * 4);
    let opacity = num(job.json, "motion_trail_opacity", 55.0).clamp(0.0, 100.0) / 100.0;
    if opacity == 0.0 {
        return 0;
    }
    let a = deg(num(job.json, "motion_trail_angle", 210.0));
    let (dx, dy) = (cosf(a) * length, sinf(a) * length);
    let colour = tint(job.json, "motion_trail_colour", [0.22, 0.78, 1.0]);
    if length > 40.0 {
        return motion_trail_large(
            job,
            &src,
            length,
            steps,
            [cosf(a), sinf(a)],
            colour,
            opacity,
        );
    }
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let mut exposure = [0.0; 4];
            let mut weight = 0.0;
            for k in 1..=samples {
                let t = (k as f32 - 0.5) / samples as f32;
                let w = (1.0 - t) * (1.0 - t);
                let sample = src.sample(x as f32 + dx * t, y as f32 + dy * t);
                weight += w;
                exposure[3] += sample[3] * w;
                for c in 0..3 {
                    exposure[c] += sample[c] * sample[3] * w;
                }
            }
            if exposure[3] > 0.0 {
                for c in 0..3 {
                    exposure[c] = mix(exposure[c] / exposure[3], colour[c], 0.16);
                }
                exposure[3] = (exposure[3] / weight * opacity * 1.5).clamp(0.0, 1.0);
                put(job, y * job.width + x, over(base, exposure));
            }
        }
    }
    0
}

fn motion_trail_large(
    job: &mut Job,
    src: &Surface,
    length: f32,
    steps: usize,
    direction: [f32; 2],
    colour: [f32; 3],
    opacity: f32,
) -> i32 {
    let Some(bounds) = alpha_bounds(src) else {
        return 0;
    };
    let (ox, oy, _, _) = canvas(job);
    let bounds = [
        bounds[0] + ox,
        bounds[1] + oy,
        bounds[2] + ox,
        bounds[3] + oy,
    ];
    let Some(d) = Distance::with_threshold(src, 32.0, f32::MIN_POSITIVE) else {
        return 1;
    };
    let quality_step = (length / (steps * 4) as f32).max(0.5);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let ax = ox + x as f32;
            let ay = oy + y as f32;
            let Some((enter, exit)) = ray_interval(ax, ay, direction, bounds, length) else {
                continue;
            };
            let mut travel = floorf(enter * 2.0) * 0.5;
            let mut exposure = [0.0_f32; 4];
            while travel < exit {
                let sx = ax + direction[0] * travel - ox;
                let sy = ay + direction[1] * travel - oy;
                let distance = d.at(floorf(sx) as i32, floorf(sy) as i32);
                if distance < -3.5 {
                    travel += (floorf((-distance - 3.5) * 2.0) * 0.5).max(0.5);
                    continue;
                }
                // Fine intervals at every silhouette preserve one-pixel sources.
                // In a filled region, grow only as far as both the local boundary
                // and the requested quality allow, avoiding fine steps across broad faces.
                let span = (floorf((distance - 3.5).max(0.5).min(quality_step) * 2.0) * 0.5)
                    .max(0.5)
                    .min(exit - travel);
                let midpoint = travel + span * 0.5;
                let sample = src.sample(
                    ax + direction[0] * midpoint - ox,
                    ay + direction[1] * midpoint - oy,
                );
                let a = 1.0 - travel / length;
                let b = 1.0 - (travel + span) / length;
                // Exact integral of the quadratic shutter weight across this
                // interval, normalized by its full [0,1] integral of one third.
                let weight = span / length * (a * a + a * b + b * b) / 3.0;
                exposure[3] += sample[3] * weight;
                for c in 0..3 {
                    exposure[c] += sample[c] * sample[3] * weight;
                }
                travel += span;
            }
            if exposure[3] > 0.0 {
                for c in 0..3 {
                    exposure[c] = mix(exposure[c] / exposure[3], colour[c], 0.16);
                }
                exposure[3] = (exposure[3] * 4.5 * opacity).clamp(0.0, 1.0);
                put(job, y * job.width + x, over(base, exposure));
            }
        }
    }
    0
}

fn water_drops(job: &mut Job) -> i32 {
    let Some((src, ox, oy, _, _)) = setup(job) else {
        return 1;
    };
    let scale = num(job.json, "water_drops_scale", 32.0).clamp(14.0, 1500.0);
    let amount = num(job.json, "water_drops_amount", 62.0).clamp(5.0, 100.0) / 100.0;
    let seed = num(job.json, "water_drops_seed", 53.0) as u32;
    let hi = tint(job.json, "water_drops_highlight", [0.96, 1.0, 1.0]);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let mut p = src.sample(x as f32, y as f32);
            if p[3] <= 0.0 {
                continue;
            }
            // Two independent size populations create a natural mix of beads and condensation.
            for population in 0..2 {
                let cell = scale * if population == 0 { 0.82 } else { 0.37 };
                let gx = floorf((ox + x as f32) / cell) as i32;
                let gy = floorf((oy + y as f32) / cell) as i32;
                for cy in gy - 1..=gy + 1 {
                    for cx in gx - 1..=gx + 1 {
                        let key = (cx as u32).wrapping_mul(0x9e3779b9)
                            ^ (cy as u32).wrapping_mul(0x85ebca6b)
                            ^ seed
                            ^ (population * 0x12345);
                        if hash(key) > amount * if population == 0 { 0.94 } else { 0.68 } {
                            continue;
                        }
                        let center_x = (cx as f32 + 0.15 + hash(key ^ 0x41) * 0.7) * cell;
                        let center_y = (cy as f32 + 0.15 + hash(key ^ 0x93) * 0.7) * cell;
                        let radius = cell * (0.13 + hash(key ^ 0x135) * 0.2);
                        let stretch = 0.85 + hash(key ^ 0x71) * 0.3;
                        let nx = (ox + x as f32 - center_x) / radius;
                        let ny = (oy + y as f32 - center_y) / (radius * stretch);
                        let r2 = nx * nx + ny * ny;
                        if r2 > 1.4 {
                            continue;
                        }
                        let r = sqrtf(r2);
                        let aa = (0.8 / radius).min(0.5);
                        let cover = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, r);
                        let shadow =
                            (1.0 - smoothstep(
                                0.98,
                                1.14,
                                sqrtf((nx - 0.1) * (nx - 0.1) + (ny - 0.14) * (ny - 0.14)),
                            )) * (1.0 - cover)
                                * 0.25;
                        for channel in &mut p[..3] {
                            *channel *= 1.0 - shadow;
                        }
                        if cover <= 0.0 {
                            continue;
                        }
                        let nz = sqrtf((1.0 - r2.min(0.999)).max(0.0));
                        let refr = src
                            .sample(x as f32 - nx * radius * 0.38, y as f32 - ny * radius * 0.38);
                        let glint = power8(dot([nx, ny, nz], [-0.35, -0.45, 0.822]))
                            * power8(dot([nx, ny, nz], [-0.35, -0.45, 0.822]));
                        let rim = smoothstep(0.68, 1.0, r)
                            * (0.14 + (nx * 0.5 + ny * 0.7).max(0.0) * 0.45);
                        let caustic =
                            power8((nx * 0.35 + ny * 0.5 + nz * 0.7).clamp(0.0, 1.0)) * 0.22;
                        for c in 0..3 {
                            let body = mix(p[c], refr[c], refr[3] * 0.85) * (1.0 - rim);
                            let lit = mix(body, hi[c], (glint * 0.93 + caustic).clamp(0.0, 0.96));
                            p[c] = mix(p[c], lit, cover);
                        }
                    }
                }
            }
            put(job, y * job.width + x, p);
        }
    }
    0
}

/// Find the lower surface at a particle's fixed horizontal anchor. This keeps its entire
/// silhouette connected, instead of independently extending each image column.
fn lower_anchor(
    src: &Surface,
    edges: &VerticalEdges,
    x: i32,
    y: i32,
    limit: i32,
) -> Option<(i32, [f32; 4])> {
    let sy = edges.at(x, y)?;
    (y - sy <= limit).then(|| (sy, src.sample(x as f32, sy as f32)))
}
// Join the shaft to the bulb at its equator, where the ellipse has a finite
// tangent. A quintic matches width, slope and curvature at both ends; blending
// into the top of a clipped ellipse introduces a normal jump and a visible ring.
fn drip_width(below: f32, len: f32, radius: f32) -> f32 {
    let bulb_height = (radius * 1.12).min(len * 0.32);
    let center = len - bulb_height;
    if below >= center {
        let v = (below - center) / bulb_height;
        return radius * sqrtf((1.0 - v * v).max(0.0));
    }
    let span = 2.0 * bulb_height;
    let start = center - span;
    if below <= start {
        let t = (below / len).clamp(0.0, 1.0);
        return radius * (0.32 + 0.70 * (1.0 - t) * (1.0 - t) * (1.0 - t));
    }
    let remaining = 1.0 - start / len;
    let a0 = radius * (0.32 + 0.70 * remaining * remaining * remaining);
    let a1 = -2.1 * radius * remaining * remaining * span / len;
    let a2 = 2.1 * radius * remaining * span * span / (len * len);
    let value = radius - a0 - a1 - a2;
    let slope = -a1 - 2.0 * a2;
    let curvature = -radius * span * span / (bulb_height * bulb_height) - 2.0 * a2;
    let a3 = 10.0 * value - 4.0 * slope + 0.5 * curvature;
    let a4 = -15.0 * value + 7.0 * slope - curvature;
    let a5 = 6.0 * value - 3.0 * slope + 0.5 * curvature;
    let u = (below - start) / span;
    a0 + u * (a1 + u * (a2 + u * (a3 + u * (a4 + u * a5))))
}

fn drip(job: &mut Job) -> i32 {
    let Some((src, ox, _, _, _)) = setup(job) else {
        return 1;
    };
    let length = num(job.json, "drip_length", 18.0).clamp(2.0, 150.0);
    let size = num(job.json, "drip_size", 100.0).clamp(25.0, 1000.0) / 100.0;
    let jitter = num(job.json, "drip_size_jitter", 100.0).clamp(0.0, 100.0) / 100.0;
    // The host requests this actual reach through the optional overlap export.
    let reach = (length * size).max(2.0);
    let amount = num(job.json, "drip_amount", 55.0).clamp(0.0, 100.0) / 100.0;
    if amount == 0.0 {
        return 0;
    }
    let Some(edges) = VerticalEdges::lower(&src) else {
        return 1;
    };
    let seed = num(job.json, "drip_seed", 59.0) as u32;
    let colour = tint(job.json, "drip_colour", [0.33, 0.79, 0.48]);
    let spacing = (length * 0.24).max(6.0) * size;
    let neighbors = ceilf((3.2 * 1.02 * size + 0.9 * size + 0.6) / spacing) as i32;
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let gx = floorf((ox + x as f32) / spacing) as i32;
            let mut layer = [0.0; 4];
            for cell in gx - neighbors..=gx + neighbors {
                let key = (cell as u32).wrapping_mul(0x9e3779b9) ^ seed;
                if hash(key) > amount {
                    continue;
                }
                let cx = (cell as f32 + 0.2 + 0.6 * hash(key ^ 0x125)) * spacing - ox;
                let radius = mix(1.0 + hash(key ^ 0x73) * 2.2, 2.1, 1.0 - jitter) * size;
                if (x as f32 - cx).abs() > radius * 1.02 + 0.9 * size + 0.6 {
                    continue;
                }
                let Some((anchor, source)) =
                    lower_anchor(&src, &edges, cx as i32, y as i32, reach as i32)
                else {
                    continue;
                };
                let below = y as f32 - anchor as f32;
                let len = (reach
                    * mix(0.3 + 0.7 * hash(key ^ 0x7f4a), 1.0, 1.0 - jitter)
                    * (0.45 + amount * 0.55))
                    .max(2.0);
                if below > len {
                    continue;
                }
                let t = below / len;
                let width = drip_width(below, len, radius);
                // Surface-of-revolution normal includes vertical curvature, so the
                // highlight rounds over the bulb instead of continuing as a stripe.
                let step = (radius * 0.025).max(0.05);
                let slope = (drip_width((below + step).min(len), len, radius)
                    - drip_width((below - step).max(0.0), len, radius))
                    / ((below + step).min(len) - (below - step).max(0.0)).max(0.001);
                let normal_scale = 1.0 / sqrtf(1.0 + slope * slope);
                let bend = sinf(t * 2.8) * (hash(key ^ 0x58) - 0.5) * 1.8 * size;
                let across = x as f32 - cx - bend;
                let cover = smoothstep(-0.6, 0.6, (width - across.abs()) * normal_scale);
                let radial_x = (across / width.max(0.001)).clamp(-1.0, 1.0);
                let nx = radial_x * normal_scale;
                let ny = -slope * normal_scale;
                let nz = sqrtf((1.0 - radial_x * radial_x).max(0.0)) * normal_scale;
                let light = (-nx * 0.40 - ny * 0.38 + nz * 0.83).max(0.0);
                let spec = power8(light) * power8(light);
                let rim = power8((nx * 0.68 + nz * 0.73).max(0.0));
                let mut paint = [0.0; 4];
                for c in 0..3 {
                    paint[c] = (mix(colour[c], source[c], 0.14) * (0.48 + light * 0.52)
                        + spec * 0.36
                        + rim * 0.055)
                        .clamp(0.0, 1.0);
                }
                paint[3] = cover * source[3];
                layer = over(paint, layer);
            }
            put(job, y * job.width + x, over(base, layer));
        }
    }
    0
}

fn snow(job: &mut Job) -> i32 {
    let Some((src, ox, oy, _, _)) = setup(job) else {
        return 1;
    };
    let depth = num(job.json, "snow_depth", 14.0).clamp(1.0, 500.0);
    let amount = num(job.json, "snow_amount", 72.0).clamp(0.0, 100.0) / 100.0;
    if amount == 0.0 {
        return 0;
    }
    let cool = tint(job.json, "snow_cool", [0.85, 0.95, 1.0]);
    let seed = num(job.json, "snow_seed", 61.0) as u32;
    let Some(d) = Distance::with_limit(&src, (depth * 0.32 + 3.0).max(64.0)) else {
        return 1;
    };
    let Some(edges) = VerticalEdges::upper(&src) else {
        return 1;
    };
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            let mut p = base;
            let ax = ox + x as f32;
            let ay = oy + y as f32;
            let mound = fbm(ax / (depth * 1.2 + 3.0), 0.7, seed);
            let height = depth * amount * (0.42 + mound * 0.58);
            let dist = d.at(x as i32, y as i32);
            let mut cover = 0.0_f32;
            let mut relative_height = 0.0_f32;
            if base[3] < 1.0 {
                let next_y = y as i32 + 1;
                let anchor = if src.alpha(x as i32, next_y) >= 0.5 {
                    Some(next_y)
                } else {
                    edges.at(x as i32, next_y)
                };
                if let Some(sy) = anchor {
                    let k = sy - y as i32;
                    if k <= ceilf(depth.max(24.0)) as i32 {
                        let (_, ny) = d.normal(x as i32, sy);
                        if ny < -0.28 {
                            cover = smoothstep(-0.65, 0.65, height * (-ny).min(1.0) - k as f32);
                            relative_height = k as f32 / height.max(0.5);
                        }
                    }
                }
            }
            if base[3] > 0.0 && dist >= 0.0 {
                let (_, ny) = d.normal(x as i32, y as i32);
                cover = cover.max(
                    (1.0 - smoothstep(0.0, (depth * 0.32 + 1.0) * amount, dist))
                        * smoothstep(0.22, 0.65, -ny),
                );
            }
            if cover > 0.0 {
                let grain = fbm(ax * 0.7, ay * 0.7, seed ^ 0x259);
                let sparkle = smoothstep(0.74, 0.86, grain) * 0.17;
                let illumination =
                    0.76 + 0.21 * smoothstep(0.0, 0.85, relative_height) + (grain - 0.5) * 0.045;
                let mut snow_colour = [0.0; 3];
                for c in 0..3 {
                    snow_colour[c] =
                        (mix(cool[c], 1.0, 0.48) * illumination + sparkle).clamp(0.0, 1.0);
                }
                p = over(rgba(snow_colour, cover), base);
            }
            put(job, y * job.width + x, p);
        }
    }
    0
}

fn icicles(job: &mut Job) -> i32 {
    let Some((src, ox, oy, _, _)) = setup(job) else {
        return 1;
    };
    let length = num(job.json, "icicles_length", 32.0).clamp(2.0, 150.0);
    let size = num(job.json, "icicles_size", 100.0).clamp(25.0, 1000.0) / 100.0;
    let jitter = num(job.json, "icicles_size_jitter", 100.0).clamp(0.0, 100.0) / 100.0;
    let reach = (length * size).max(2.0);
    let Some(edges) = VerticalEdges::lower(&src) else {
        return 1;
    };
    let density = num(job.json, "icicles_density", 66.0).clamp(5.0, 100.0) / 100.0;
    let seed = num(job.json, "icicles_seed", 67.0) as u32;
    let ice = tint(job.json, "icicles_tint", [0.74, 0.93, 1.0]);
    let spacing = (5.0 + length * 0.08) * size;
    let neighbors = ceilf((3.1 * size + 0.7 + 0.65 * size) / spacing) as i32;
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let base = src.sample(x as f32, y as f32);
            if base[3] >= 1.0 {
                continue;
            }
            let gx = floorf((ox + x as f32) / spacing) as i32;
            let mut layer = [0.0; 4];
            for cell in gx - neighbors..=gx + neighbors {
                let key = (cell as u32).wrapping_mul(0x9e3779b9) ^ seed;
                if hash(key) > density {
                    continue;
                }
                let canvas_cx = (cell as f32 + 0.22 + 0.56 * hash(key ^ 0x72)) * spacing;
                let cx = canvas_cx - ox;
                let radius = mix(1.3 + hash(key ^ 0xe73) * 1.8, 2.2, 1.0 - jitter) * size;
                if (x as f32 - cx).abs() > radius + 0.7 + 0.65 * size {
                    continue;
                }
                let Some((anchor, source)) =
                    lower_anchor(&src, &edges, cx as i32, y as i32, reach as i32)
                else {
                    continue;
                };
                let below = y as f32 - anchor as f32;
                let len = reach * mix(0.3 + 0.7 * hash(key ^ 0xe31), 1.0, 1.0 - jitter);
                if below > len {
                    continue;
                }
                let t = below / len.max(1.0);
                let width =
                    (radius * (1.0 - t) * (1.0 - t) + 0.15) * (1.0 - smoothstep(0.94, 1.0, t));
                // Form the subpixel axis in canvas coordinates: adding the bend to
                // a band-local center rounds differently in small, narrow icicles.
                let axis = canvas_cx + (hash(key ^ 0x974) - 0.5) * t * 1.3 * size;
                let across = (ox + x as f32) - axis;
                let cover = smoothstep(-0.55, 0.55, width - across.abs());
                if cover == 0.0 {
                    continue;
                }
                let nx = (across / width.max(0.2)).clamp(-1.0, 1.0);
                let facet = 0.32 + 0.35 * (1.0 - nx.abs()) + 0.2 * (-nx).max(0.0);
                let ridge = (1.0 - smoothstep(0.06, 0.32, (nx + 0.33).abs())) * 0.6;
                let rings = 0.035 * sinf((oy + y as f32) * (1.1 + hash(key ^ 0x513) * 0.7));
                let mut shard = [0.0; 4];
                for c in 0..3 {
                    shard[c] =
                        (ice[c] * (facet + rings) + ridge + 0.07 * source[c]).clamp(0.0, 1.0);
                }
                shard[3] = cover * (0.5 + 0.4 * nx.abs() + ridge * 0.14) * source[3];
                layer = over(shard, layer);
            }
            put(job, y * job.width + x, over(base, layer));
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drip_neck_to_bulb_has_no_surface_normal_jump() {
        // A discontinuous slope creates a ring in the specular lighting even
        // when the silhouette's width itself is continuous. Check the upper
        // surface; the rounded bottom legitimately has a vertical tangent.
        for (len, radius) in [(300.0_f32, 21.0_f32), (40.0, 2.1), (2.0, 8.0)] {
            let center = len - (radius * 1.12).min(len * 0.32);
            let step = len.min(radius) * 0.001;
            let mut previous: Option<f32> = None;
            let mut max_jump = 0.0_f32;
            let mut y = step;
            while y < center - step {
                let slope = (drip_width(y + step, len, radius) - drip_width(y - step, len, radius))
                    / (2.0 * step);
                let normal_angle = libm::atanf(slope);
                if let Some(last) = previous {
                    max_jump = max_jump.max((normal_angle - last).abs());
                }
                previous = Some(normal_angle);
                y += step;
            }
            assert!(
                max_jump < 0.04,
                "visible normal discontinuity for length {len}, radius {radius}: {max_jump} radians"
            );
        }
    }

    fn input_pixels() -> Vec<f32> {
        let mut pixels = vec![0.0_f32; 64 * 64 * 4];
        for y in 24..=40 {
            for x in 24..40 {
                let i = (y * 64 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[0.25, 0.5, 0.75, 1.0]);
            }
        }
        pixels
    }

    fn render(name: &str, extra: &str) -> Vec<f32> {
        let effect = EFFECTS.iter().find(|e| e.name == name).unwrap();
        let mut pixels = input_pixels();
        let mut json =
            std::string::String::from("{\"x\":0,\"y\":0,\"canvasWidth\":64,\"canvasHeight\":64,");
        json.push_str(extra);
        json.push('}');
        let mut job = Job {
            px: &mut pixels,
            width: 64,
            height: 64,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: json.as_bytes(),
        };
        assert_eq!((effect.render)(&mut job), 0, "{name}");
        pixels
    }

    fn chrome_subject(alpha_shape: bool, extra: &str) -> Vec<f32> {
        let mut pixels = vec![0.0; 128 * 128 * 4];
        for y in 0..128 {
            for x in 0..128 {
                if !alpha_shape || (16..112).contains(&x) && (16..112).contains(&y) {
                    pixels[(y * 128 + x) * 4..][..4].copy_from_slice(&[0.42, 0.42, 0.42, 1.0]);
                }
            }
        }
        let json =
            std::format!("{{\"x\":0,\"y\":0,\"canvasWidth\":128,\"canvasHeight\":128,{extra}}}");
        let mut job = Job {
            px: &mut pixels,
            width: 128,
            height: 128,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: json.as_bytes(),
        };
        assert_eq!(chrome(&mut job), 0);
        pixels
    }

    #[test]
    fn chrome_has_bright_and_dark_reflections_across_opaque_and_cutout_surfaces() {
        for alpha_shape in [false, true] {
            let output = chrome_subject(alpha_shape, "\"chrome_strength\":78");
            // Inspect the face, excluding the silhouette bevel. Chrome must not just
            // tint or darken the flat interior of a selected photograph.
            let samples: Vec<f32> = (32..96)
                .flat_map(|y| (32..96).map(move |x| y * 128 + x))
                .map(|i| {
                    output[i * 4] * 0.2126 + output[i * 4 + 1] * 0.7152 + output[i * 4 + 2] * 0.0722
                })
                .collect();
            let bright = samples.iter().filter(|&&v| v > 0.80).count();
            let dark = samples.iter().filter(|&&v| v < 0.24).count();
            assert!(
                bright > samples.len() / 20,
                "missing chrome highlights: {bright} / {}",
                samples.len()
            );
            assert!(
                dark > samples.len() / 20,
                "missing reflected dark flags: {dark} / {}",
                samples.len()
            );
            if alpha_shape {
                assert_eq!(output[3], 0.0, "Chrome must preserve the shape alpha");
            }
        }
    }

    #[test]
    fn chrome_angle_moves_reflections_and_scale_changes_surface_curvature() {
        let baseline = chrome_subject(false, "\"chrome_angle\":35,\"chrome_scale\":34");
        for params in [
            "\"chrome_angle\":125,\"chrome_scale\":34",
            "\"chrome_angle\":35,\"chrome_scale\":80",
        ] {
            let changed = chrome_subject(false, params);
            let difference: f32 = baseline
                .iter()
                .zip(changed.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();
            assert!(
                difference / baseline.len() as f32 > 0.025,
                "Chrome control has no visible effect on the face: {params}"
            );
        }
    }

    #[test]
    fn chrome_reflections_use_selection_bounds_independently_of_document_size() {
        let expected = chrome_subject(false, "\"chrome_angle\":35");
        let mut output = vec![0.42; 128 * 128 * 4];
        for pixel in output.as_chunks_mut::<4>().0 {
            pixel[3] = 1.0;
        }
        let json = br#"{"chrome_angle":35,"_image":{"x":256,"y":384,"canvasWidth":4096,"canvasHeight":3072,"shapeX":256,"shapeY":384,"shapeWidth":128,"shapeHeight":128}}"#;
        let mut job = Job {
            px: &mut output,
            width: 128,
            height: 128,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json,
        };
        assert_eq!(chrome(&mut job), 0);
        assert_eq!(output, expected);
    }

    #[test]
    fn material_geometry_has_solid_snow_and_paint_and_shaded_ice() {
        let snow = render("snow", "\"snow_amount\":100,\"snow_depth\":14");
        assert!(
            (24..40).any(|x| snow[(23 * 64 + x) * 4 + 3] > 0.98),
            "snow must form a solid cap"
        );
        let paint = render("drip", "\"drip_amount\":100,\"drip_length\":40");
        assert!(
            (24..40).any(|x| paint[(43 * 64 + x) * 4 + 3] > 0.95),
            "paint must form an opaque liquid body"
        );
        let ice = render("icicles", "\"icicles_density\":100,\"icicles_length\":40");
        let samples: Vec<f32> = ice
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(i, p)| i / 64 > 40 && p[3] > 0.3)
            .map(|(_, p)| p[0])
            .collect();
        let lo = samples.iter().copied().fold(1.0, f32::min);
        let hi = samples.iter().copied().fold(0.0, f32::max);
        assert!(
            hi - lo > 0.15,
            "ice must have reflective facets rather than one flat tint"
        );
    }

    #[test]
    fn exposure_trails_retain_source_colour() {
        let output = render(
            "motion_trail",
            "\"motion_trail_angle\":0,\"motion_trail_length\":20,\"motion_trail_colour\":\"#ff0000\"",
        );
        let p = &output[(32 * 64 + 20) * 4..][..4];
        assert!(p[3] > 0.0);
        assert!(
            p[2] > p[0],
            "a blue object should keep its colour in an exposure trail"
        );
    }

    #[test]
    fn gaussian_shadow_has_a_continuous_penumbra() {
        let output = render(
            "shadow",
            "\"shadow_distance\":20,\"shadow_angle\":0,\"shadow_softness\":10,\"shadow_opacity\":100",
        );
        let alphas: Vec<f32> = (43..63).map(|x| output[(32 * 64 + x) * 4 + 3]).collect();
        let levels = alphas
            .windows(2)
            .filter(|a| (a[0] - a[1]).abs() > 0.001)
            .count();
        assert!(
            levels >= 16,
            "a soft shadow must not split into a few offset silhouettes: {levels}"
        );
    }

    #[test]
    fn material_settings_within_sixty_four_pixels_match_the_legacy_host_halo() {
        let width = 320;
        let mut source = vec![0.0; width * width * 4];
        for y in 0..width {
            for x in 0..width {
                let dx = x as f32 - 113.0;
                let dy = y as f32 - 146.0;
                let ex = x as f32 - 193.0;
                let ey = y as f32 - 147.0;
                if dx * dx + dy * dy < 43.0 * 43.0 || ex * ex + ey * ey < 27.0 * 27.0 {
                    let i = (y * width + x) * 4;
                    source[i..i + 4].copy_from_slice(&[
                        0.25 + x as f32 / 700.0,
                        0.4,
                        y as f32 / 400.0,
                        1.0,
                    ]);
                }
            }
        }
        let params = r#""bevel_size":24,"chrome_scale":80,"glass_refraction":24,"extrude_length":40,"shadow_distance":40,"shadow_softness":12,"motion_trail_length":40,"water_drops_scale":64,"drip_length":60,"drip_size":100,"drip_size_jitter":0,"drip_amount":100,"snow_depth":24,"icicles_length":60,"icicles_size":100,"icicles_size_jitter":0,"icicles_density":100"#;
        for effect in EFFECTS {
            let cases: &[&str] = if matches!(effect.name, "drip" | "icicles") {
                &[
                    params,
                    r#""drip_length":2,"drip_size":25,"drip_size_jitter":100,"drip_amount":100,"icicles_length":2,"icicles_size":25,"icicles_size_jitter":100,"icicles_density":100"#,
                    r#""drip_length":60,"drip_size":25,"drip_size_jitter":0,"drip_amount":100,"icicles_length":60,"icicles_size":25,"icicles_size_jitter":0,"icicles_density":100"#,
                ]
            } else {
                &[params]
            };
            for params in cases {
                let mut whole = source.clone();
                let json = std::format!(
                    "{{\"x\":0,\"y\":0,\"canvasWidth\":320,\"canvasHeight\":320,{params}}}"
                );
                let mut job = Job {
                    px: &mut whole,
                    width,
                    height: width,
                    channels: 4,
                    format: 8 | crate::HAS_ALPHA | (3 << 16),
                    json: json.as_bytes(),
                };
                assert_eq!((effect.render)(&mut job), 0);
                let mut tile = vec![0.0; 192 * 192 * 4];
                for y in 0..192 {
                    for x in 0..192 {
                        tile[(y * 192 + x) * 4..][..4]
                            .copy_from_slice(&source[((y + 64) * width + x + 64) * 4..][..4]);
                    }
                }
                let json = std::format!(
                    "{{\"x\":64,\"y\":64,\"canvasWidth\":320,\"canvasHeight\":320,\"overlap\":64,{params}}}"
                );
                let mut job = Job {
                    px: &mut tile,
                    width: 192,
                    height: 192,
                    channels: 4,
                    format: 8 | crate::HAS_ALPHA | (3 << 16),
                    json: json.as_bytes(),
                };
                assert_eq!((effect.render)(&mut job), 0);
                let mut error = 0.0_f32;
                for y in 64..128 {
                    for x in 64..128 {
                        for c in 0..4 {
                            error = error.max(
                                (tile[(y * 192 + x) * 4 + c]
                                    - whole[((y + 64) * width + x + 64) * 4 + c])
                                    .abs(),
                            );
                        }
                    }
                }
                assert!(
                    error < 0.0001,
                    "{} depends on pixels beyond the host halo: {error}; {params}",
                    effect.name
                );
            }
        }
    }

    #[test]
    fn lists_ten_distinct_object_material_effects() {
        let names = [
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
        ];
        assert_eq!(EFFECTS.len(), names.len());
        for (effect, name) in EFFECTS.iter().zip(names) {
            assert_eq!(effect.name, name);
            assert!(!effect.params.is_empty());
        }
    }

    #[test]
    fn renderers_keep_a_transparent_test_subject_finite_and_editable() {
        for effect in EFFECTS {
            let mut pixels = [0.0_f32; 64 * 64 * 4];
            for y in 12..52 {
                for x in 12..52 {
                    let i = (y * 64 + x) * 4;
                    pixels[i..i + 4].copy_from_slice(&[0.25, 0.5, 0.75, 1.0]);
                }
            }
            let before = pixels;
            let json = b"{\"x\":0,\"y\":0,\"canvasWidth\":64,\"canvasHeight\":64}";
            let mut job = Job {
                px: &mut pixels,
                width: 64,
                height: 64,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json,
            };
            assert_eq!((effect.render)(&mut job), 0, "{}", effect.name);
            assert!(job.px.iter().all(|v| v.is_finite()), "{}", effect.name);
            assert_ne!(
                *job.px, before,
                "{} should make a visible change",
                effect.name
            );
        }
    }

    #[test]
    fn snow_caps_grow_above_the_subject() {
        let output = render("snow", "\"snow_amount\":100");
        assert!(output[(23 * 64 + 32) * 4 + 3] > 0.0);
    }

    #[test]
    fn icicles_extend_below_lower_facing_edges() {
        let output = render("icicles", "\"icicles_density\":100,\"icicles_length\":40");
        assert!(output[(41 * 64 + 32) * 4 + 3] > 0.0);
    }

    #[test]
    fn drips_extend_down_from_the_nearest_subject_edge() {
        let output = render("drip", "\"drip_amount\":100,\"drip_length\":40");
        let extends = (24..40).any(|x| (41..64).any(|y| output[(y * 64 + x) * 4 + 3] > 0.0));
        assert!(extends);
    }

    #[test]
    fn zero_strength_controls_leave_the_input_unchanged() {
        for (name, params) in [
            ("bevel", "\"bevel_depth\":0"),
            ("chrome", "\"chrome_strength\":0"),
            (
                "glass",
                "\"glass_opacity\":0,\"glass_highlight\":0,\"glass_refraction\":0",
            ),
            ("shadow", "\"shadow_opacity\":0"),
            ("motion_trail", "\"motion_trail_opacity\":0"),
            ("drip", "\"drip_amount\":0"),
            ("snow", "\"snow_amount\":0"),
        ] {
            let output = render(name, params);
            let baseline = input_pixels();
            assert_eq!(output, baseline, "zero strength changed {name}");
        }
    }
}

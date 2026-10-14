//! Seeded warped Voronoi cracks and missing paint exposing a procedural undercoat.
use super::Effect;
use super::common::*;
use crate::{Job, colour, num};
pub const EFFECT: Effect = Effect {
    name: "cracked_paint",
    params: PARAMS,
    render,
};
const PARAMS: &str = r##"
    "cracked_paint_scale":{"type":"number","default":70,"group":"Cracked Paint","label":"Crack spacing","visibleWhen":{"effect":"cracked_paint"},"min":8,"max":1500},
    "cracked_paint_width":{"type":"number","default":0.7,"group":"Cracked Paint","label":"Crack width","visibleWhen":{"effect":"cracked_paint"},"min":0.1,"max":30},
    "cracked_paint_chipping":{"type":"number","default":0,"group":"Cracked Paint","label":"Chipping","visibleWhen":{"effect":"cracked_paint"},"min":0,"max":100},
    "cracked_paint_colour":{"type":"color","default":"#199494","group":"Cracked Paint","label":"Paint color","visibleWhen":{"effect":"cracked_paint"}},
    "cracked_paint_undercoat":{"type":"color","default":"#48372c","group":"Cracked Paint","label":"Undercoat","visibleWhen":{"effect":"cracked_paint"}},
    "cracked_paint_grain":{"type":"number","default":30,"group":"Cracked Paint","label":"Surface grain","visibleWhen":{"effect":"cracked_paint"},"min":0,"max":100},
    "cracked_paint_seed":{"type":"int","default":103,"group":"Cracked Paint","label":"Random seed","visibleWhen":{"effect":"cracked_paint"},"min":0,"max":9999}"##;
use libm::{floorf, sinf, sqrtf};
// Document-aligned jittered Voronoi cells: fixed 5x5 search, independent of image size.
fn cracks(x: f32, y: f32, scale: f32, seed: u32) -> (f32, f32) {
    let u = x / scale + 0.14 * sinf(y / scale * 2.9) + 0.08 * sinf(x / scale * 1.1);
    let v = y / scale + 0.13 * sinf(x / scale * 2.1);
    let (ix, iy) = (floorf(u) as i32, floorf(v) as i32);
    let (mut first, mut second, mut cell) = (f32::MAX, f32::MAX, 0.0);
    for yy in iy - 2..=iy + 2 {
        for xx in ix - 2..=ix + 2 {
            let id =
                (xx as u32).wrapping_mul(0x9e3779b9) ^ (yy as u32).wrapping_mul(0x85ebca6b) ^ seed;
            let dx = u - (xx as f32 + 0.15 + 0.7 * hash(id));
            let dy = v - (yy as f32 + 0.15 + 0.7 * hash(id ^ 917));
            let distance = dx * dx + dy * dy;
            if distance < first {
                second = first;
                first = distance;
                cell = hash(id ^ 731);
            } else if distance < second {
                second = distance;
            }
        }
    }
    ((sqrtf(second) - sqrtf(first)) * scale, cell)
}
fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(src) = Surface::new(job) else {
        return 3;
    };
    let scale = num(job.json, "cracked_paint_scale", 70.0).clamp(8.0, 1500.0);
    let width = num(job.json, "cracked_paint_width", 0.7).clamp(0.1, 30.0);
    let chips = num(job.json, "cracked_paint_chipping", 0.0).clamp(0.0, 100.0) / 100.0;
    let grain = num(job.json, "cracked_paint_grain", 30.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "cracked_paint_seed", 103.0).clamp(0.0, 9999.0) as u32;
    let paint = colour(job.json, "cracked_paint_colour", [0.10, 0.58, 0.58]);
    let under = colour(job.json, "cracked_paint_undercoat", [0.28, 0.22, 0.17]);
    let (ox, oy, _, _) = canvas(job);
    let (x0, y0, x1, y1) = output_bounds(job);
    for y in y0..y1 {
        for x in x0..x1 {
            let a = src.alpha(x as i32, y as i32);
            if a == 0.0 {
                write(job, y * job.width + x, [0.0; 4]);
                continue;
            }
            let (ax, ay) = (ox + x as f32, oy + y as f32);
            let (gap, cell) = cracks(ax, ay, scale, seed);
            let crack = 1.0 - smoothstep(width, width + 1.2, gap);
            let n = noise(ax / (scale * 0.24), ay / (scale * 0.24), seed ^ 57);
            let chipped = smoothstep(0.92 - chips * 0.65, 1.0 - chips * 0.65, n)
                * smoothstep(1.0 - chips, 1.02 - chips, cell)
                * chips.min(0.01)
                / 0.01;
            let lost = crack.max(chipped);
            let lip = exp_lip(gap, width);
            let texture = (noise(ax * 0.8, ay * 0.8, seed ^ 23) - 0.5) * grain * 0.14;
            let mut out = [0.0, 0.0, 0.0, a];
            for c in 0..3 {
                out[c] = mix(
                    paint[c] * (0.88 + lip + texture),
                    under[c] * (0.60 + 0.22 * n),
                    lost,
                )
                .clamp(0.0, 1.0);
            }
            write(job, y * job.width + x, out);
        }
    }
    0
}
fn exp_lip(gap: f32, width: f32) -> f32 {
    (1.0 - smoothstep(width + 1.2, width + 3.0, gap)) * smoothstep(width, width + 1.2, gap) * 0.14
}

//! Spectral foil response, microgrooves and a silhouette bevel.
use super::Effect;
use super::common::*;
use crate::{Job, colour, num};
pub const EFFECT: Effect = Effect {
    name: "holographic_foil",
    params: PARAMS,
    render,
};
const PARAMS: &str = r##"
    "holographic_foil_scale":{"type":"number","default":250,"group":"Holographic Foil","label":"Rainbow scale","visibleWhen":{"effect":"holographic_foil"},"min":10,"max":1500},
    "holographic_foil_angle":{"type":"number","default":25,"group":"Holographic Foil","label":"Groove angle","visibleWhen":{"effect":"holographic_foil"},"min":-180,"max":180},
    "holographic_foil_shift":{"type":"number","default":0,"group":"Holographic Foil","label":"Color shift","visibleWhen":{"effect":"holographic_foil"},"min":-180,"max":180},
    "holographic_foil_bevel":{"type":"number","default":18,"group":"Holographic Foil","label":"Bevel size","visibleWhen":{"effect":"holographic_foil"},"min":0,"max":500},
    "holographic_foil_strength":{"type":"number","default":75,"group":"Holographic Foil","label":"Rainbow strength","visibleWhen":{"effect":"holographic_foil"},"min":0,"max":100},
    "holographic_foil_colour":{"type":"color","default":"#c2dbed","group":"Holographic Foil","label":"Metal base","visibleWhen":{"effect":"holographic_foil"}},
    "holographic_foil_grooves":{"type":"number","default":35,"group":"Holographic Foil","label":"Microgrooves","visibleWhen":{"effect":"holographic_foil"},"min":0,"max":100}"##;
use libm::{cosf, expf, sinf, sqrtf};
fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(src) = Surface::new(job) else {
        return 3;
    };
    let bevel = num(job.json, "holographic_foil_bevel", 18.0).clamp(0.0, 500.0);
    let Some(d) = Distance::with_limit(&src, bevel + 4.0) else {
        return 3;
    };
    let scale = num(job.json, "holographic_foil_scale", 250.0).clamp(10.0, 1500.0);
    let angle = num(job.json, "holographic_foil_angle", 25.0)
        .clamp(-180.0, 180.0)
        .to_radians();
    let shift = num(job.json, "holographic_foil_shift", 0.0).clamp(-180.0, 180.0) / 360.0;
    let strength = num(job.json, "holographic_foil_strength", 75.0).clamp(0.0, 100.0) / 100.0;
    let grooves = num(job.json, "holographic_foil_grooves", 35.0).clamp(0.0, 100.0) / 100.0;
    let base = colour(job.json, "holographic_foil_colour", [0.76, 0.86, 0.93]);
    let (ox, oy, _, _) = canvas(job);
    let (x0, y0, x1, y1) = output_bounds(job);
    for y in y0..y1 {
        for x in x0..x1 {
            let a = src.alpha(x as i32, y as i32);
            if a == 0.0 {
                write(job, y * job.width + x, [0.0; 4]);
                continue;
            }
            let (nx, ny) = d.normal(x as i32, y as i32);
            let slope = (1.0 - smoothstep(0.0, bevel.max(0.01), d.at(x as i32, y as i32)))
                * 0.75
                * (if bevel > 0.0 { 1.0 } else { 0.0 });
            let norm = sqrtf(1.0 + slope * slope);
            let (nx, ny, nz) = (nx * slope / norm, ny * slope / norm, 1.0 / norm);
            let (ax, ay) = (ox + x as f32, oy + y as f32);
            let u = ax * cosf(angle) + ay * sinf(angle);
            let phase = u / scale + ny * 0.72 + nx * 0.43 + 0.15 * sinf(ay / 130.0) + shift;
            let studio = 0.45
                + 0.5 * nz
                + 0.36 * expf(-square((ny + 0.55) / 0.14))
                + 0.42 * expf(-square((nx - 0.48) / 0.10));
            let shine = expf(-(square(nx + 0.22) + square(ny + 0.44)) / 0.035) * 0.5;
            let grain = 1.0 - grooves * 0.12 * (1.0 - cosf(u * 2.5));
            let mut out = [0.0, 0.0, 0.0, a];
            for c in 0..3 {
                let offset = [0.0, -1.0 / 3.0, 1.0 / 3.0][c];
                let rainbow = 0.55 + 0.45 * cosf(core::f32::consts::TAU * (phase + offset));
                out[c] = (mix(base[c], rainbow, strength) * studio * grain + shine).clamp(0.0, 1.0);
            }
            write(job, y * job.width + x, out);
        }
    }
    0
}

fn square(x: f32) -> f32 {
    x * x
}

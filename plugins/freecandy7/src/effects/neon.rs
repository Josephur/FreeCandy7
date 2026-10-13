//! Procedural glass tubes following the source silhouette, including interior holes.
use super::Effect;
use super::common::{
    Distance, Surface, canvas, mix, noise, output_bounds, over, smoothstep, supported, write,
};
use crate::{Job, colour, num};
use libm::expf;

pub const EFFECT: Effect = Effect {
    name: "neon_tubes",
    params: PARAMS,
    render,
};

const PARAMS: &str = r##"
    "neon_tubes_width":{"type":"number","min":2,"max":500,"default":8,"group":"Neon Tubes","label":"Tube width","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_glow":{"type":"number","min":0,"max":1250,"default":28,"group":"Neon Tubes","label":"Glow radius","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_brightness":{"type":"number","min":0,"max":100,"default":85,"group":"Neon Tubes","label":"Brightness","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_colour":{"type":"color","default":"#ff428a","group":"Neon Tubes","label":"Tube colour","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_glass":{"type":"number","min":0,"max":100,"default":55,"group":"Neon Tubes","label":"Glass reflections","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_breaks":{"type":"number","min":0,"max":100,"default":0,"group":"Neon Tubes","label":"Broken sections","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_variation":{"type":"number","min":0,"max":100,"default":0,"group":"Neon Tubes","label":"Brightness variation","visibleWhen":{"effect":"neon_tubes"}},
    "neon_tubes_seed":{"type":"int","min":0,"max":9999,"default":149,"group":"Neon Tubes","label":"Random seed","visibleWhen":{"effect":"neon_tubes"}}"##;

fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(src) = Surface::new(job) else {
        return 3;
    };
    let mut inside = false;
    let mut outside = false;
    for y in 0..job.height {
        for x in 0..job.width {
            let a = src.alpha(x as i32, y as i32);
            inside |= a >= 0.5;
            outside |= a < 0.5;
            if inside && outside {
                break;
            }
        }
        if inside && outside {
            break;
        }
    }
    // An unselected opaque photograph has no silhouette to trace.
    if !inside || !outside {
        return 0;
    }
    let radius = num(job.json, "neon_tubes_width", 8.0).clamp(2.0, 500.0) * 0.5;
    let glow = num(job.json, "neon_tubes_glow", 28.0).clamp(0.0, 1250.0);
    let Some(distance) = Distance::with_limit(&src, radius + glow + 4.0) else {
        return 3;
    };
    let brightness = num(job.json, "neon_tubes_brightness", 85.0).clamp(0.0, 100.0) / 100.0;
    let glass = num(job.json, "neon_tubes_glass", 55.0).clamp(0.0, 100.0) / 100.0;
    let breaks = num(job.json, "neon_tubes_breaks", 0.0).clamp(0.0, 100.0) / 100.0;
    let variation = num(job.json, "neon_tubes_variation", 0.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "neon_tubes_seed", 149.0).clamp(0.0, 9999.0) as u32;
    let tint = colour(job.json, "neon_tubes_colour", [1.0, 0.259, 0.541]);
    let (ox, oy, _, _) = canvas(job);
    let (x0, y0, x1, y1) = output_bounds(job);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = distance.at(x as i32, y as i32).abs();
            let away = (d - radius).max(0.0);
            let body = 1.0 - smoothstep(radius - 0.6, radius + 0.6, d);
            let halo = if glow > 0.0 {
                expf(-3.0 * away / glow) * (1.0 - smoothstep(glow * 0.75, glow, away)) * 0.48
            } else {
                0.0
            };
            let ax = ox + x as f32;
            let ay = oy + y as f32;
            let intact = if breaks == 0.0 {
                1.0
            } else if breaks == 1.0 {
                0.0
            } else {
                1.0 - smoothstep(
                    1.0 - breaks - 0.06,
                    1.0 - breaks + 0.06,
                    noise(ax / 43.0, ay / 43.0, seed),
                )
            };
            let light =
                brightness * (1.0 - variation * 0.75 * noise(ax / 57.0, ay / 57.0, seed ^ 71));
            let cross_section = d / (radius * 0.3).max(0.8);
            let core = expf(-3.0 * cross_section * cross_section);
            let (nx, ny) = distance.normal(x as i32, y as i32);
            let highlight = (nx * 0.55 - ny * 0.83).max(0.0);
            let highlight2 = highlight * highlight;
            let highlight4 = highlight2 * highlight2;
            let reflection = highlight4 * highlight4 * glass * 0.32;
            let white = (core * 0.92 + reflection).min(1.0);
            let tube = [
                mix(tint[0], 1.0, white) * light,
                mix(tint[1], 1.0, white) * light,
                mix(tint[2], 1.0, white) * light,
                body * intact,
            ];
            let aura = [tint[0], tint[1], tint[2], halo * intact * light];
            write(job, y * job.width + x, over(tube, aura));
        }
    }
    0
}

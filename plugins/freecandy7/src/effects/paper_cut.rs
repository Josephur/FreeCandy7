//! Stacked silhouette sheets with document-aligned fibers and optional corrugated edges.
use super::Effect;
use super::common::*;
use crate::{Job, colour, num};
pub const EFFECT: Effect = Effect {
    name: "paper_cut",
    params: PARAMS,
    render,
};
const PARAMS: &str = r##"
    "paper_cut_layers":{"type":"int","default":8,"group":"Paper Cut","label":"Layer count","visibleWhen":{"effect":"paper_cut"},"min":1,"max":24},
    "paper_cut_depth":{"type":"number","default":36,"group":"Paper Cut","label":"Stack depth","visibleWhen":{"effect":"paper_cut"},"min":0,"max":500},
    "paper_cut_angle":{"type":"number","default":57,"group":"Paper Cut","label":"Stack direction","visibleWhen":{"effect":"paper_cut"},"min":-180,"max":180},
    "paper_cut_material":{"type":"choice","default":"paper","group":"Paper Cut","label":"Material","visibleWhen":{"effect":"paper_cut"},"options":["paper","cardboard"]},
    "paper_cut_colour":{"type":"color","default":"#fa5c4f","group":"Paper Cut","label":"Top sheet color","visibleWhen":{"effect":"paper_cut"}},
    "paper_cut_grain":{"type":"number","default":40,"group":"Paper Cut","label":"Surface grain","visibleWhen":{"effect":"paper_cut"},"min":0,"max":100},
    "paper_cut_shadow":{"type":"number","default":55,"group":"Paper Cut","label":"Contact shadows","visibleWhen":{"effect":"paper_cut"},"min":0,"max":100}"##;
use libm::{cosf, sinf};
fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(src) = Surface::new(job) else {
        return 3;
    };
    let layers = num(job.json, "paper_cut_layers", 8.0).clamp(1.0, 24.0) as usize;
    let depth = num(job.json, "paper_cut_depth", 36.0).clamp(0.0, 500.0);
    let angle = num(job.json, "paper_cut_angle", 57.0)
        .clamp(-180.0, 180.0)
        .to_radians();
    let (dx, dy) = (cosf(angle), sinf(angle));
    let cardboard = crate::find_value(job.json, b"paper_cut_material")
        .is_some_and(|v| crate::parse_str(v) == b"cardboard");
    let top = colour(job.json, "paper_cut_colour", [0.98, 0.36, 0.31]);
    let grain = num(job.json, "paper_cut_grain", 40.0).clamp(0.0, 100.0) / 100.0;
    let shadow = num(job.json, "paper_cut_shadow", 55.0).clamp(0.0, 100.0) / 100.0;
    let paper = [
        [0.09, 0.27, 0.34],
        [0.08, 0.40, 0.47],
        [0.14, 0.54, 0.57],
        [0.31, 0.65, 0.62],
        [0.65, 0.77, 0.65],
        [0.92, 0.86, 0.68],
        [0.99, 0.61, 0.38],
    ];
    let (ox, oy, _, _) = canvas(job);
    let (x0, y0, x1, y1) = output_bounds(job);
    let Some(clearance) = SourceClearance::new(&src, (ox as i32, oy as i32), depth + 12.0) else {
        return 3;
    };
    let mut offsets = [(0.0, 0.0, 0.0); 24];
    for (i, offset) in offsets.iter_mut().enumerate().take(layers) {
        let t = if layers > 1 {
            i as f32 / (layers - 1) as f32
        } else {
            0.0
        };
        *offset = (t * depth * dx, t * depth * dy, t);
    }
    for y in y0..y1 {
        for x in x0..x1 {
            let mut out = [0.0; 4];
            if clearance.at(x as i32, y as i32) > depth + 8.0 {
                write(job, y * job.width + x, out);
                continue;
            }
            let (ax, ay) = (ox + x as f32, oy + y as f32);
            let mut alphas = [0.0; 24];
            let mut visible = layers;
            // An opaque sheet hides every deeper sheet and its shadow.
            for i in 0..layers {
                alphas[i] = src.sample_alpha(x as f32 - offsets[i].0, y as f32 - offsets[i].1);
                if alphas[i] == 1.0 {
                    visible = i + 1;
                    break;
                }
            }
            for i in (0..visible).rev() {
                let (sx, sy, t) = (
                    x as f32 - offsets[i].0,
                    y as f32 - offsets[i].1,
                    offsets[i].2,
                );
                let a = alphas[i];
                // A small fixed convolution gives each real layer its own contact shadow.
                let shade = if a == 1.0 || shadow == 0.0 {
                    0.0
                } else {
                    (src.sample_alpha(sx - 2.0, sy - 3.0) * 0.4
                        + src.sample_alpha(sx - 4.0, sy - 3.0) * 0.15
                        + src.sample_alpha(sx, sy - 3.0) * 0.15
                        + src.sample_alpha(sx - 2.0, sy - 5.0) * 0.15
                        + src.sample_alpha(sx - 2.0, sy - 1.0) * 0.15)
                        * shadow
                        * 0.35
                };
                if shade > 0.0 {
                    out = over([0.04, 0.035, 0.025, shade], out);
                }
                if a == 0.0 {
                    continue;
                }
                let mut col = if i == 0 {
                    top
                } else if cardboard {
                    let k = 1.0 - t * 0.68;
                    [top[0] * k, top[1] * k * 0.94, top[2] * k * 0.88]
                } else {
                    paper[(layers - 1 - i) * 7 / layers]
                };
                let mut texture = 1.0 + (noise(ax * 0.7, ay * 0.7, 817) - 0.5) * grain * 0.13;
                if cardboard {
                    if i > 0 {
                        let flute = sinf(ax * 0.36 + i as f32 * 0.8).max(0.0);
                        texture -= 0.38 * flute * flute * flute;
                    } else {
                        texture += 0.012 * sinf(ax * 0.5);
                    }
                }
                for c in &mut col {
                    *c = (*c * texture).clamp(0.0, 1.0);
                }
                out = over([col[0], col[1], col[2], a], out);
            }
            write(job, y * job.width + x, out);
        }
    }
    0
}

//! Stamped aluminium tread: sparse alternating lancet ribs on a continuous sheet.
//! All geometry and reflections are evaluated locally; there is no image input.
use super::common::{canvas, mix, noise, output_bounds, smoothstep, supported, write};
use crate::Job;
use libm::{cosf, expf, floorf, sinf, sqrtf};
fn sq(x: f32) -> f32 {
    x * x
}

/// A shallow pressed ridge, pointed at both ends. Adjacent cells turn through
/// ninety degrees, as in single-bar commercial diamond tread sheet.
fn rib_height(x: f32, y: f32, pitch: f32, relief: f32) -> f32 {
    let cx = floorf(x / pitch);
    let cy = floorf(y / pitch);
    let dx = x - (cx + 0.5) * pitch;
    let dy = y - (cy + 0.5) * pitch;
    let handed = if (cx as i32 ^ cy as i32) & 1 == 0 {
        1.0
    } else {
        -1.0
    };
    let a = (dx + dy * handed) * core::f32::consts::FRAC_1_SQRT_2 / (pitch * 0.47);
    let b = (-dx * handed + dy) * core::f32::consts::FRAC_1_SQRT_2 / (pitch * 0.095);
    let taper = (1.0 - a * a).max(0.0);
    let profile = (taper - b.abs()).max(0.0);
    // A small smoothing radius rounds the rolled lip; the two main faces
    // remain planar enough to produce the sharply opposed reflections.
    let rounded = profile * profile / (profile + 0.12);
    rounded * pitch * (0.018 + relief * 0.065)
}

/// Conservatively include the ridge, its derivative samples and contact shadow.
/// Flat sheet pixels do not need four copies of the expensive reflection shader.
fn near_rib(x: f32, y: f32, pitch: f32, radius: f32) -> bool {
    if radius >= pitch * 0.1 {
        return true;
    }
    let cx = floorf(x / pitch);
    let cy = floorf(y / pitch);
    let dx = x - (cx + 0.5) * pitch;
    let dy = y - (cy + 0.5) * pitch;
    let handed = if (cx as i32 ^ cy as i32) & 1 == 0 {
        1.0
    } else {
        -1.0
    };
    let a = (dx + dy * handed).abs() * core::f32::consts::FRAC_1_SQRT_2;
    let b = (-dx * handed + dy).abs() * core::f32::consts::FRAC_1_SQRT_2;
    let inner_a = (a - radius).max(0.0) / (pitch * 0.47);
    inner_a <= 1.0 && b <= (1.0 - inner_a * inner_a) * pitch * 0.095 + radius
}

fn environment(x: f32, y: f32, rough: f32) -> f32 {
    let blur = 0.018 + rough * 0.23;
    let horizon = smoothstep(-0.16 - blur, 0.04 + blur, y);
    let softbox = expf(-sq((x + 0.52) / (0.07 + rough * 0.27)))
        * smoothstep(-0.65, -0.2, y)
        * (1.0 - smoothstep(0.75, 1.2, y));
    let upper = expf(-sq((y - 0.54) / (0.11 + rough * 0.23)));
    let dark_flag =
        smoothstep(0.14 - blur, 0.18 + blur, x) * (1.0 - smoothstep(0.43 - blur, 0.5 + blur, x));
    (0.13 + horizon * 0.75 + softbox * 1.3 + upper * 0.7 - dark_flag * 0.45).max(0.045)
}

#[derive(Clone, Copy)]
struct Settings {
    scale: f32,
    detail: f32,
    relief: f32,
    rough: f32,
    light_cos: f32,
    light_sin: f32,
    contrast: f32,
    seed: u32,
    color: [f32; 3],
}

fn sample(x: f32, y: f32, c: Settings) -> [f32; 3] {
    let h = rib_height(x, y, c.scale, c.relief);
    let epsilon = (c.scale * 0.007).clamp(0.2, 0.7);
    let dx = (rib_height(x + epsilon, y, c.scale, c.relief)
        - rib_height(x - epsilon, y, c.scale, c.relief))
        / (2.0 * epsilon);
    let dy = (rib_height(x, y + epsilon, c.scale, c.relief)
        - rib_height(x, y - epsilon, c.scale, c.relief))
        / (2.0 * epsilon);
    // Small, broad sheet waviness bends the environment without turning the
    // sheet itself into a checkerboard of different metal colors.
    let warp = (noise(x / (c.scale * 8.0), y / (c.scale * 7.0), c.seed + 3) - 0.5) * 0.65;
    let nx = -dx + warp;
    let ny =
        -dy + (noise(x / (c.scale * 6.0) + 37.0, y / (c.scale * 9.0), c.seed + 6) - 0.5) * 0.72;
    let inv = 1.0 / sqrtf(nx * nx + ny * ny + 1.0);
    let rx = 2.0 * nx * inv * inv;
    let ry = 2.0 * ny * inv * inv;
    let ca = c.light_cos;
    let sa = c.light_sin;
    let ex = rx * ca + ry * sa;
    let ey = -rx * sa + ry * ca + 0.16;
    let env = environment(ex, ey, c.rough);
    let shade = mix(0.91, env, c.contrast);
    let contact = (rib_height(x - 0.85, y - 0.95, c.scale, c.relief) - h).max(0.0);
    let occlusion = 1.0 - (contact / (c.scale * 0.1)).clamp(0.0, 0.55);
    let grain = (noise(x * 1.7, y * 1.7, c.seed + 1) - 0.5) * (0.007 + c.rough * 0.025) * c.detail;
    let brushing = (noise(x / (c.scale * 2.5), y * 2.0, c.seed + 2) - 0.5) * 0.014 * c.rough;
    c.color
        .map(|v| (v * shade * occlusion + grain + brushing).clamp(0.0, 1.0))
}

pub(super) fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let n = |suffix: &str, default: f32| {
        super::textures::read_num(job, "diamond_plate", suffix, default)
    };
    let light = n("lighting", 315.0).to_radians();
    let c = Settings {
        scale: n("scale", 34.0).clamp(8.0, 96.0),
        detail: n("detail", 55.0).clamp(0.0, 100.0) / 100.0,
        relief: n("relief", 48.0).clamp(0.0, 100.0) / 100.0,
        rough: n("roughness", 24.0).clamp(0.0, 100.0) / 100.0,
        light_cos: cosf(light),
        light_sin: sinf(light),
        contrast: n("contrast", 85.0).clamp(0.0, 100.0) / 100.0,
        seed: n("seed", 19.0).clamp(0.0, 9999.0) as u32,
        color: super::textures::read_colour(job, "diamond_plate", "colour", [0.61, 0.65, 0.66]),
    };
    let angle = n("direction", 0.0).to_radians();
    let ca = cosf(angle);
    let sa = sinf(angle);
    let preserve = super::textures::read_bool(job, "diamond_plate", "preserve_alpha", false);
    let has_alpha = job.format & crate::HAS_ALPHA != 0;
    let (ox, oy, cw, ch) = canvas(job);
    let factor = super::textures::adaptive_scale(job, "diamond_plate");
    let (left, top, right, bottom) = output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let px = (ox + x as f32 + 0.5 - cw * 0.5) / factor;
            let py = (oy + y as f32 + 0.5 - ch * 0.5) / factor;
            let tx = px * ca + py * sa;
            let ty = -px * sa + py * ca;
            let mut rgb;
            if near_rib(tx, ty, c.scale, 2.0 + 0.5 / factor) {
                rgb = [0.0; 3];
                for (jx, jy) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
                    let ax = px + jx / factor;
                    let ay = py + jy / factor;
                    let tap = sample(ax * ca + ay * sa, -ax * sa + ay * ca, c);
                    for i in 0..3 {
                        rgb[i] += tap[i] * 0.25;
                    }
                }
            } else {
                rgb = sample(tx, ty, c);
            }
            let i = y * job.width + x;
            let a = if preserve && has_alpha {
                job.px[i * job.channels + job.channels - 1]
            } else {
                1.0
            };
            write(job, i, [rgb[0], rgb[1], rgb[2], a]);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn most_of_tread_sheet_is_flat_and_adjacent_ribs_are_perpendicular() {
        let s = 40.0;
        let raised = (0..160)
            .flat_map(|y| (0..160).map(move |x| (x, y)))
            .filter(|&(x, y)| rib_height(x as f32 / 4.0, y as f32 / 4.0, s, 0.5) > 0.01)
            .count();
        let coverage = raised as f32 / (160 * 160) as f32;
        assert!(
            (0.06..0.22).contains(&coverage),
            "raised coverage {coverage}"
        );
        assert!(rib_height(26.0, 26.0, s, 0.5) > 0.2);
        assert_eq!(rib_height(26.0, 14.0, s, 0.5), 0.0);
        assert!(rib_height(66.0, 14.0, s, 0.5) > 0.2);
        assert_eq!(rib_height(66.0, 26.0, s, 0.5), 0.0);
    }
    #[test]
    fn ribs_taper_to_points_and_do_not_make_raised_tile_boundaries() {
        for x in 0..80 {
            assert_eq!(rib_height(x as f32, 40.0, 40.0, 0.5), 0.0);
        }
        let center = rib_height(20.0, 20.0, 40.0, 0.5);
        assert!(center > rib_height(30.0, 30.0, 40.0, 0.5));
        assert_eq!(rib_height(34.0, 34.0, 40.0, 0.5), 0.0);
    }
    #[test]
    fn antialias_region_covers_every_nearby_rib_sample() {
        let radius = 2.5;
        let mut flat = 0;
        for y in -80..80 {
            for x in -80..80 {
                let (x, y) = (x as f32 * 0.5, y as f32 * 0.5);
                if near_rib(x, y, 34.0, radius) {
                    continue;
                }
                flat += 1;
                for k in 0..32 {
                    let a = k as f32 * core::f32::consts::TAU / 32.0;
                    assert_eq!(
                        rib_height(x + cosf(a) * radius, y + sinf(a) * radius, 34.0, 0.5),
                        0.0
                    );
                }
            }
        }
        assert!(
            flat > 160 * 160 / 2,
            "flat sheet should skip redundant supersampling"
        );
    }
}

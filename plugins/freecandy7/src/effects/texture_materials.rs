//! Deterministic procedural fields for the non-fur texture effects.
//!
//! Coordinates are absolute canvas coordinates (rotated by the caller where the
//! effect has a direction control). Every field depends only on those coordinates
//! and its settings, so independently rendered bands produce identical pixels.

use super::common::{fbm, mix, noise, smoothstep};
use libm::{floorf, sqrtf};

/// Return a material colour for one of the non-fur texture effects.
///
/// `kind` matches the existing texture renderer's stable internal indices:
/// 1 brick, 2 brushed metal, 3 clouds, 5 marble, 6 reptile
/// skin, 7 rust, 8 stone wall, 9 texture noise, 10 weave, and 11 wood. `a`
/// and `b` retain the existing effect-specific swatch convention: brick and
/// stone use `a` for the body and `b` for joints; single-colour metals use `a`;
/// two-colour fields blend from `b` into `a`.
#[allow(clippy::too_many_arguments)]
pub(super) fn sample(
    kind: u8,
    u: f32,
    v: f32,
    x: f32,
    y: f32,
    scale: f32,
    detail: f32,
    seed: u32,
    a: [f32; 3],
    b: [f32; 3],
    variant: u8,
) -> Option<[f32; 3]> {
    let s = scale.max(1.0);
    let d = detail.clamp(0.0, 1.0);
    let body = match kind {
        1 => brick(u, v, s, d, seed, a, b),
        2 => brushed_metal(u, v, s, d, seed, a),
        3 => clouds(x, y, s, d, seed, a, b),
        5 => marble(x, y, s, d, seed, a, b),
        6 => reptile(u, v, s, d, seed, a, b),
        7 => rust(x, y, s, d, seed, a, b),
        8 => stone_wall(x, y, s, d, seed, [a, b]),
        9 => texture_noise(x, y, s, d, seed, [a, b], variant),
        10 => weave(u, v, s, d, seed, a, b),
        11 => wood(u, v, s, d, seed, a, b),
        _ => return None,
    };
    Some(body.map(|channel| channel.clamp(0.0, 1.0)))
}

fn blend(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}

fn shade(c: [f32; 3], l: f32) -> [f32; 3] {
    [c[0] * l, c[1] * l, c[2] * l]
}

fn brick(u: f32, v: f32, s: f32, d: f32, seed: u32, brick: [f32; 3], mortar: [f32; 3]) -> [f32; 3] {
    let course_h = s * 0.48;
    let row = floorf(v / course_h) as i32;
    let stagger = if row & 1 == 0 { 0.0 } else { s * 0.5 };
    let col = floorf((u + stagger) / s) as i32;
    let lx = (u + stagger) - floorf((u + stagger) / s) * s;
    let ly = v - floorf(v / course_h) * course_h;
    let id = (col as u32).wrapping_mul(0x9e37_79b9) ^ (row as u32).wrapping_mul(0x85eb_ca6b) ^ seed;
    let edge = (s * 0.048).clamp(1.2, 5.5);
    // A small, irregular squeeze in the mortar softens the perfect CAD grid.
    let wobble = (noise(u / (s * 2.4), v / (s * 1.7), seed + 4) - 0.5) * edge * 0.24;
    let inset = lx
        .min(s - lx)
        .min(ly * s / course_h)
        .min((course_h - ly) * s / course_h)
        + wobble;
    let joint = 1.0 - smoothstep(edge * 0.52, edge * 1.16, inset);
    let bevel = smoothstep(edge * 1.1, edge * 2.3, inset)
        * (1.0 - smoothstep(edge * 2.3, edge * 4.8, inset));
    let coarse = fbm(u / (s * 0.43), v / (s * 0.38), seed + 7);
    let grain = noise(u / (s * 0.055 + 0.3), v / (s * 0.065 + 0.3), seed + 8);
    let per_brick = 0.88 + noise(col as f32, row as f32, id) * 0.22;
    let face = 0.87 + (coarse - 0.5) * (0.25 + d * 0.22) + (grain - 0.5) * (0.12 + d * 0.12);
    let face = face * per_brick - bevel * (0.11 + d * 0.05);
    let joint_colour = shade(
        mortar,
        0.72 + 0.2 * fbm(u / (s * 0.2), v / (s * 0.2), seed + 9),
    );
    let brick_colour = shade(brick, face.clamp(0.48, 1.13));
    blend(joint_colour, brick_colour, 1.0 - joint)
}

fn brushed_metal(u: f32, v: f32, s: f32, d: f32, seed: u32, metal: [f32; 3]) -> [f32; 3] {
    // A long-grain anisotropic field, with scratches that stay parallel to the
    // brushing direction instead of forming the old cross-hatched noise.
    let grain = fbm(v / (s * 0.055 + 0.2), u / (s * 4.8 + 4.0), seed);
    let fine = noise(v / (s * 0.012 + 0.12), u / (s * 7.0 + 3.0), seed + 1);
    let scratch = noise(v / 0.75, u / (s * 8.0 + 8.0), seed + 2);
    let scratch_cut = if scratch > 0.965 {
        (scratch - 0.965) * 9.0
    } else if scratch < 0.035 {
        -(0.035 - scratch) * 3.0
    } else {
        0.0
    };
    let broad = 0.5 + 0.5 * libm::cosf(v / (s * 1.7 + 2.0) + (grain - 0.5) * 1.1);
    let l = (0.82
        + (grain - 0.5) * (0.28 + d * 0.2)
        + (fine - 0.5) * (0.08 + d * 0.08)
        + libm::powf(broad, 18.0) * (0.13 + d * 0.09)
        + scratch_cut)
        .clamp(0.28, 1.19);
    shade(metal, l)
}

fn clouds(x: f32, y: f32, s: f32, d: f32, seed: u32, light: [f32; 3], dark: [f32; 3]) -> [f32; 3] {
    let low = fbm(x / (s * 1.25), y / (s * 0.9), seed);
    let broad = fbm(x / (s * 3.7), y / (s * 2.8), seed + 5);
    let erosion = fbm(x / (s * 0.34), y / (s * 0.31), seed + 11);
    let q = low * 0.68 + broad * 0.32 + (erosion - 0.5) * (0.08 + d * 0.18);
    // Cloud body transitions have a broad, luminous rim at the edge of the
    // denser lobes, which gives the field volume without looking cell-thresholded.
    let body = smoothstep(0.34 - d * 0.04, 0.68, q);
    let silver_lining = libm::powf((1.0 - (q - 0.53).abs() / 0.18).clamp(0.0, 1.0), 2.0) * body;
    let cool_shadow = (0.9 + (broad - 0.5) * 0.34).clamp(0.7, 1.1);
    let base = blend(
        dark,
        light,
        (body * 0.88 + silver_lining * (0.08 + d * 0.12)).clamp(0.0, 1.0),
    );
    shade(base, cool_shadow)
}

fn marble(x: f32, y: f32, s: f32, d: f32, seed: u32, light: [f32; 3], dark: [f32; 3]) -> [f32; 3] {
    // Isotropic mineral variation plus warped contour veins makes a branching
    // stone network. The previous one-axis sine read as a single horizontal seam.
    let warp_x = fbm(x / (s * 1.9), y / (s * 1.6), seed);
    let warp_y = fbm(x / (s * 1.35) + 17.0, y / (s * 1.8) - 9.0, seed + 2);
    let wx = x + (warp_x - 0.5) * s * (1.0 + d * 1.1);
    let wy = y + (warp_y - 0.5) * s * (0.9 + d * 1.0);
    let cloud = fbm(wx / (s * 1.0), wy / (s * 0.92), seed + 4);
    let veins = fbm(wx / (s * 0.39), wy / (s * 0.36), seed + 7);
    let branches = fbm(wx / (s * 0.19), wy / (s * 0.22), seed + 10);
    let distance = (veins - 0.5).abs();
    let branch_distance = (branches - 0.51).abs();
    let main_vein = 1.0 - smoothstep(0.012, 0.052 + d * 0.012, distance);
    let fine_vein = 1.0 - smoothstep(0.008, 0.027, branch_distance);
    let halo = 1.0 - smoothstep(0.045, 0.17, distance);
    let grain = noise(wx / (s * 0.048 + 0.1), wy / (s * 0.05 + 0.1), seed + 13);
    let mottling = fbm(wx / (s * 0.17 + 0.2), wy / (s * 0.16 + 0.2), seed + 15);
    let color_t =
        (0.52 + cloud * 0.31 + (mottling - 0.5) * (0.12 + d * 0.08) + (grain - 0.5) * 0.04)
            .clamp(0.0, 1.0);
    let base = blend(dark, light, color_t);
    let vein = (main_vein * 0.70 + fine_vein * 0.30).clamp(0.0, 1.0);
    let veined = blend(base, dark, vein * (0.42 + d * 0.12));
    let softened = blend(veined, light, halo * (0.06 + d * 0.05));
    softened.map(|c| (c * (0.94 + mottling * 0.11 + (grain - 0.5) * d * 0.06)).clamp(0.0, 1.0))
}

fn reptile(u: f32, v: f32, s: f32, d: f32, seed: u32, light: [f32; 3], dark: [f32; 3]) -> [f32; 3] {
    // Staggered oval shingles with tapered sides and a curved lower edge. The
    // previous hex field read as honeycomb; the elongated domes and overlap seams
    // better match the rounded keratin scales in the reference images.
    let pitch_x = s * 0.94;
    let pitch_y = s * 0.62;
    let row = floorf(v / pitch_y) as i32;
    let shift = if row & 1 == 0 { 0.0 } else { pitch_x * 0.5 };
    let col = floorf((u + shift) / pitch_x) as i32;
    let fx = (u + shift) / pitch_x - floorf((u + shift) / pitch_x) - 0.5;
    let fy = v / pitch_y - floorf(v / pitch_y) - 0.5;
    let ix = col as f32;
    let iy = row as f32;
    let jitter_x = (noise(ix, iy, seed) - 0.5) * 0.08;
    let jitter_y = (noise(ix + 23.0, iy - 9.0, seed + 1) - 0.5) * 0.07;
    let py = fy - jitter_y;
    let px = fx - jitter_x - py * 0.16;
    let taper = 0.49 * (1.0 - py * 0.14);
    let edge_warp = (noise(ix + 7.0, iy - 3.0, seed + 5) - 0.5) * 0.025;
    let rx = px / (taper + edge_warp);
    let curve = 1.0 - (px / 0.5) * (px / 0.5);
    let ry = (py + curve * 0.045) / 0.59;
    let radius = sqrtf(rx * rx + ry * ry);
    let edge = 1.0 - smoothstep(0.94, 1.08, radius);
    let dome = sqrtf((1.0 - radius * radius).max(0.0));
    let ridge_x = (px + 0.15) * 2.3;
    let ridge_y = (py - 0.17) * 1.8;
    let ridge = libm::powf(
        (1.0 - sqrtf(ridge_x * ridge_x + ridge_y * ridge_y)).clamp(0.0, 1.0),
        5.0,
    );
    let pores = noise(u / (s * 0.07 + 0.14), v / (s * 0.06 + 0.14), seed + 4);
    let id_tint = noise(ix, iy, seed + 3);
    let base = blend(
        dark,
        light,
        (0.28 + id_tint * 0.36 + (dome - 0.5) * 0.15).clamp(0.0, 1.0),
    );
    let l = (0.62 + dome * 0.29 + ridge * (0.22 + d * 0.12) + (pores - 0.5) * (0.09 + d * 0.08))
        .clamp(0.3, 1.18);
    let plate = shade(base, l);
    let seam = shade(dark, 0.68 + pores * 0.18);
    blend(seam, plate, edge)
}

fn rust(x: f32, y: f32, s: f32, d: f32, seed: u32, oxide: [f32; 3], iron: [f32; 3]) -> [f32; 3] {
    // Oxide grows as connected islands, with a thin transition halo, flaky
    // granulation, dark pinholes and remnants of cooler exposed steel.
    let broad = fbm(x / (s * 1.28), y / (s * 0.94), seed);
    let medium = fbm(x / (s * 0.34), y / (s * 0.39), seed + 4);
    let edge_noise = fbm(x / (s * 0.13), y / (s * 0.11), seed + 8);
    let boundary = broad * 0.62 + medium * (0.24 + d * 0.06) + edge_noise * (0.14 - d * 0.06);
    let coverage = smoothstep(0.46 - d * 0.035, 0.57 + d * 0.02, boundary);
    let halo = libm::powf(
        (1.0 - ((boundary - 0.51).abs() / 0.15)).clamp(0.0, 1.0),
        2.0,
    );
    let grain = noise(x / (s * 0.035 + 0.18), y / (s * 0.045 + 0.18), seed + 10);
    let pits = smoothstep(
        0.72,
        0.9,
        noise(x / (s * 0.075 + 0.2), y / (s * 0.065 + 0.2), seed + 14),
    );
    let oxide_t =
        (0.28 + medium * 0.42 + (grain - 0.5) * (0.2 + d * 0.18) + halo * 0.14).clamp(0.0, 1.0);
    let rust_color = shade(
        blend(iron, oxide, oxide_t),
        (0.69 + medium * 0.30 + (grain - 0.5) * 0.12 - pits * 0.54).clamp(0.18, 1.05),
    );
    let steel = shade(iron, 0.72 + broad * 0.38 - pits * 0.28);
    blend(steel, rust_color, coverage)
}

fn stone_wall(x: f32, y: f32, s: f32, d: f32, seed: u32, colors: [[f32; 3]; 2]) -> [f32; 3] {
    let [stone, mortar] = colors;
    // Jittered Voronoi centers create fitted, naturally varied ashlar blocks.
    // The distance gap provides a stable joint width, while nearest-center
    // offsets act as a soft, directional face relief.
    let cell_w = s;
    let cell_h = s * 0.72;
    let gx = floorf(x / cell_w) as i32;
    let gy = floorf(y / cell_h) as i32;
    let mut first = 9999.0_f32;
    let mut second = 9999.0_f32;
    let mut nearest_dx = 0.0;
    let mut nearest_dy = 0.0;
    let mut tint = 0.5;
    let mut block_id = (0, 0);
    for yy in -1..=1 {
        for xx in -1..=1 {
            let ix = gx + xx;
            let iy = gy + yy;
            let jx = (noise(ix as f32, iy as f32, seed) - 0.5) * cell_w * 0.32;
            let jy = (noise(ix as f32 + 81.0, iy as f32 - 17.0, seed + 1) - 0.5) * cell_h * 0.30;
            let dx = x - (ix as f32 + 0.5) * cell_w - jx;
            let dy = y - (iy as f32 + 0.5) * cell_h - jy;
            let dist = sqrtf(dx * dx + dy * dy);
            if dist < first {
                second = first;
                first = dist;
                nearest_dx = dx;
                nearest_dy = dy;
                tint = noise(ix as f32, iy as f32, seed + 3);
                block_id = (ix, iy);
            } else if dist < second {
                second = dist;
            }
        }
    }
    let joint_width =
        s * (0.045 + (noise(block_id.0 as f32, block_id.1 as f32, seed + 5) - 0.5) * 0.012);
    let joint = 1.0 - smoothstep(joint_width, joint_width * 2.2, second - first);
    let bevel = smoothstep(joint_width * 1.2, joint_width * 3.4, second - first)
        * (1.0 - smoothstep(joint_width * 3.5, joint_width * 8.0, second - first));
    let coarse = fbm(x / (s * 0.38), y / (s * 0.34), seed + 7);
    let grain = fbm(x / (s * 0.075 + 0.2), y / (s * 0.08 + 0.2), seed + 9);
    let face = 0.72
        + tint * 0.16
        + (coarse - 0.5) * (0.28 + d * 0.15)
        + (grain - 0.5) * (0.17 + d * 0.12)
        + (-(nearest_dx + nearest_dy) / s).clamp(-0.5, 0.5) * 0.10
        - bevel * 0.11;
    let face_color = shade(stone, face.clamp(0.32, 1.12));
    let mortar_color = shade(mortar, 0.72 + coarse * 0.22 + (grain - 0.5) * 0.08);
    blend(face_color, mortar_color, joint)
}

fn texture_noise(
    x: f32,
    y: f32,
    s: f32,
    d: f32,
    seed: u32,
    colors: [[f32; 3]; 2],
    variant: u8,
) -> [f32; 3] {
    let [light, dark] = colors;
    if variant == 1 {
        let qx = x / s;
        let qy = y / s;
        let gx = floorf(qx) as i32;
        let gy = floorf(qy) as i32;
        let mut first = 9999.0_f32;
        let mut second = 9999.0_f32;
        let mut tint = 0.5;
        for yy in -1..=1 {
            for xx in -1..=1 {
                let ix = gx + xx;
                let iy = gy + yy;
                let fx = ix as f32 + 0.16 + noise(ix as f32, iy as f32, seed) * 0.68;
                let fy =
                    iy as f32 + 0.16 + noise(ix as f32 + 41.0, iy as f32 - 19.0, seed + 1) * 0.68;
                let dx = qx - fx;
                let dy = qy - fy;
                let dist = sqrtf(dx * dx + dy * dy);
                if dist < first {
                    second = first;
                    first = dist;
                    tint = noise(ix as f32, iy as f32, seed + 3);
                } else if dist < second {
                    second = dist;
                }
            }
        }
        let edge = 1.0 - smoothstep(0.035, 0.16, second - first);
        let bevel = smoothstep(0.025, 0.13, second - first)
            * (1.0 - smoothstep(0.14, 0.37, second - first));
        let inside = fbm(x / (s * 0.31), y / (s * 0.29), seed + 7);
        blend(
            dark,
            light,
            (0.24 + tint * 0.44 + inside * (0.23 + d * 0.12) + bevel * 0.12 - edge * 0.26)
                .clamp(0.0, 1.0),
        )
    } else {
        let f = fbm(x / s, y / s, seed);
        let warp = fbm(x / (s * 0.72), y / (s * 0.72), seed + 4) - 0.5;
        let ridges = 1.0 - (f * 2.0 - 1.0).abs();
        let micro = fbm(x / (s * 0.18 + 0.2), y / (s * 0.18 + 0.2), seed + 8);
        let fleck = noise(x / (s * 0.035 + 0.12), y / (s * 0.04 + 0.12), seed + 12);
        let value = (0.42
            + warp * 0.24
            + libm::powf(ridges.clamp(0.0, 1.0), 1.8) * (0.25 + d * 0.15)
            + (micro - 0.5) * (0.16 + d * 0.18)
            + (fleck - 0.5) * d * 0.08)
            .clamp(0.0, 1.0);
        blend(dark, light, value)
    }
}

fn weave(u: f32, v: f32, s: f32, d: f32, seed: u32, light: [f32; 3], dark: [f32; 3]) -> [f32; 3] {
    let thread = (s * 0.48).max(2.0);
    let ix = floorf(u / thread) as i32;
    let iy = floorf(v / thread) as i32;
    let fx = (u / thread - floorf(u / thread) - 0.5) * 2.0;
    let fy = (v / thread - floorf(v / thread) - 0.5) * 2.0;
    let vertical = 1.0 - smoothstep(0.52, 0.97, fx.abs());
    let horizontal = 1.0 - smoothstep(0.52, 0.97, fy.abs());
    let vertical_over = (ix ^ iy) & 1 == 0;
    let top = if vertical_over { vertical } else { horizontal };
    let under = if vertical_over { horizontal } else { vertical };
    let center = if vertical_over { fx.abs() } else { fy.abs() };
    let fiber = noise(u / (thread * 0.045 + 0.1), v / (thread * 0.55 + 0.2), seed);
    let crossfiber = noise(
        u / (thread * 0.42 + 0.2),
        v / (thread * 0.04 + 0.1),
        seed + 2,
    );
    let id = noise(ix as f32, iy as f32, seed + 4);
    let raised = libm::powf((1.0 - center).clamp(0.0, 1.0), 0.7);
    let yarn_l = (0.62
        + raised * 0.36
        + (fiber - 0.5) * (0.12 + d * 0.12)
        + (crossfiber - 0.5) * 0.06
        + (id - 0.5) * 0.07)
        .clamp(0.3, 1.15);
    let top_color = shade(
        blend(dark, light, (0.42 + id * 0.14).clamp(0.0, 1.0)),
        yarn_l,
    );
    let shadow = shade(dark, 0.58 + (fiber - 0.5) * 0.1);
    let cloth = blend(shadow, top_color, top);
    let interlace = (1.0 - top) * under * (0.1 + d * 0.06);
    cloth.map(|c| (c + interlace).clamp(0.0, 1.0))
}

fn wood(u: f32, v: f32, s: f32, d: f32, seed: u32, light: [f32; 3], dark: [f32; 3]) -> [f32; 3] {
    // Growth lines run along the board; latewood bands and elongated pores
    // supply grain, while deterministic knot centers bend the lines into eyes.
    let cell_x = s * 3.1;
    let cell_y = s * 2.6;
    let gx = floorf(u / cell_x) as i32;
    let gy = floorf(v / cell_y) as i32;
    let mut nearest = 9999.0_f32;
    let mut knot_dx = 0.0;
    let mut knot_dy = 0.0;
    let mut knot_seed = 0.5;
    for yy in -1..=1 {
        for xx in -1..=1 {
            let ix = gx + xx;
            let iy = gy + yy;
            let cx = (ix as f32 + 0.22 + noise(ix as f32, iy as f32, seed + 1) * 0.56) * cell_x;
            let cy =
                (iy as f32 + 0.24 + noise(ix as f32 + 31.0, iy as f32 - 17.0, seed + 2) * 0.52)
                    * cell_y;
            let dx = (u - cx) / (s * 0.80);
            let dy = (v - cy) / (s * 0.58);
            let radius = sqrtf(dx * dx + dy * dy);
            if radius < nearest {
                nearest = radius;
                knot_dx = u - cx;
                knot_dy = v - cy;
                knot_seed = noise(ix as f32, iy as f32, seed + 3);
            }
        }
    }
    let angle = libm::atan2f(knot_dy, knot_dx);
    let knot = libm::expf(-nearest * 1.15);
    let broad = fbm(u / (s * 2.0), v / (s * 0.48), seed + 5);
    let cross = fbm(u / (s * 0.66), v / (s * 0.29), seed + 7);
    let flow = (broad - 0.5) * s * (0.20 + d * 0.24)
        + (cross - 0.5) * s * 0.14
        + libm::sinf(angle) * knot * s * 0.78;
    let grain_v = v + flow;
    let phase = grain_v / (s * 0.19 + 0.8)
        + (fbm(u / (s * 1.1), grain_v / (s * 0.42), seed + 9) - 0.5) * (0.82 + d * 0.72);
    let ring = 0.5 + 0.5 * libm::sinf(phase);
    let latewood = libm::powf((1.0 - ring).clamp(0.0, 1.0), 4.4);
    let pores = noise(
        u / (s * 0.48 + 0.3),
        grain_v / (s * 0.032 + 0.12),
        seed + 11,
    );
    let flecks = fbm(u / (s * 0.24 + 0.2), grain_v / (s * 0.12 + 0.2), seed + 13);
    let ray = noise(u / (s * 2.4), grain_v / (s * 0.026 + 0.1), seed + 15);
    let knot_rings = 0.5 + 0.5 * libm::sinf(nearest * 15.0 + knot_seed * 8.0);
    let eye = knot * (0.5 + knot_rings * 0.5);
    let eye_line = libm::powf((1.0 - (knot_rings - 0.5).abs() * 3.3).clamp(0.0, 1.0), 4.0) * knot;
    let fine_grain = libm::powf((1.0 - ring).clamp(0.0, 1.0), 11.0);
    let value = (0.34
        + ring * 0.42
        + (flecks - 0.5) * (0.12 + d * 0.10)
        + (pores - 0.5) * (0.20 + d * 0.1)
        + (ray - 0.5) * 0.08
        + fine_grain * (0.06 + d * 0.04)
        - latewood * (0.13 + d * 0.04)
        - eye * 0.12
        + eye_line * 0.08)
        .clamp(0.0, 1.0);
    let base = blend(dark, light, value);
    // Dark elliptical heart and two fine growth rings make the knots legible
    // without overpowering the long grain at normal viewing size.
    let knot_ring = libm::powf((1.0 - (knot_rings - 0.5).abs() * 3.0).clamp(0.0, 1.0), 5.0) * knot;
    let darkening = (knot * 0.20 - knot_ring * 0.13).clamp(0.0, 0.24);
    base.map(|c| (c * (1.0 - darkening)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::sample;

    const PAIR: [([f32; 3], [f32; 3]); 11] = [
        ([0.61, 0.28, 0.19], [0.78, 0.77, 0.72]),
        ([0.53, 0.59, 0.61], [0.53, 0.59, 0.61]),
        ([0.93, 0.96, 1.0], [0.13, 0.28, 0.47]),
        ([0.61, 0.65, 0.66], [0.61, 0.65, 0.66]),
        ([0.91, 0.86, 0.79], [0.47, 0.35, 0.33]),
        ([0.74, 0.5, 0.23], [0.2, 0.12, 0.07]),
        ([0.79, 0.48, 0.22], [0.22, 0.27, 0.28]),
        ([0.66, 0.65, 0.6], [0.32, 0.31, 0.28]),
        ([0.91, 0.84, 0.74], [0.22, 0.15, 0.25]),
        ([0.22, 0.39, 0.53], [0.08, 0.17, 0.27]),
        ([0.71, 0.39, 0.18], [0.21, 0.09, 0.05]),
    ];

    #[test]
    fn field_materials_are_finite_bounded_and_deterministic() {
        for kind in (1..=11).filter(|kind| *kind != 4) {
            let (a, b) = PAIR[(kind - 1) as usize];
            for y in -32..33 {
                for x in -32..33 {
                    let p = sample(
                        kind,
                        x as f32 * 2.3,
                        y as f32 * 1.9,
                        x as f32 * 2.3,
                        y as f32 * 1.9,
                        37.0,
                        0.63,
                        29,
                        a,
                        b,
                        u8::from(kind == 9),
                    )
                    .unwrap();
                    let q = sample(
                        kind,
                        x as f32 * 2.3,
                        y as f32 * 1.9,
                        x as f32 * 2.3,
                        y as f32 * 1.9,
                        37.0,
                        0.63,
                        29,
                        a,
                        b,
                        u8::from(kind == 9),
                    )
                    .unwrap();
                    assert!(
                        p.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)),
                        "kind {kind}: {p:?}"
                    );
                    assert_eq!(p, q, "kind {kind} must be deterministic");
                }
            }
        }
    }

    #[test]
    fn unsupported_kind_is_absent_and_noise_variants_differ() {
        assert!(sample(0, 0.0, 0.0, 0.0, 0.0, 10.0, 0.5, 1, [1.0; 3], [0.0; 3], 0).is_none());
        let a = sample(
            9, 22.0, 16.0, 22.0, 16.0, 23.0, 0.55, 41, PAIR[8].0, PAIR[8].1, 0,
        )
        .unwrap();
        let b = sample(
            9, 22.0, 16.0, 22.0, 16.0, 23.0, 0.55, 41, PAIR[8].0, PAIR[8].1, 1,
        )
        .unwrap();
        assert_ne!(a, b);
    }
}

//! Offline, deterministic coat synthesis. Authored albedo is separated from
//! high-frequency hair relief, then both are remapped and shaded in local code.
//! Assets contain pixels only: no image decoder, model, inference or network.
use super::common::{Surface, canvas, hash, mix, noise, output_bounds, supported, write};
use crate::Job;
use libm::{cosf, floorf, log2f, sinf};

const SIZE: usize = 768;
const LEVELS: usize = 10;
const COATS: [&[u8]; 6] = [
    include_bytes!("../../assets/fur/golden.rgb"),
    include_bytes!("../../assets/fur/giraffe.rgb"),
    include_bytes!("../../assets/fur/leopard.rgb"),
    include_bytes!("../../assets/fur/zebra.rgb"),
    include_bytes!("../../assets/fur/tiger.rgb"),
    include_bytes!("../../assets/fur/cheetah.rgb"),
];

fn luminance(c: [f32; 3]) -> f32 {
    c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
}

fn level_sample(data: &[u8], x: f32, y: f32, level: usize) -> [f32; 3] {
    let mut offset = 0;
    let mut size = SIZE;
    for _ in 0..level {
        offset += size * size * 3;
        size /= 2;
    }
    let factor = size as f32 / SIZE as f32;
    let tx = x * factor;
    let ty = y * factor;
    let xx = tx - floorf(tx / size as f32) * size as f32;
    let yy = ty - floorf(ty / size as f32) * size as f32;
    let ix = (floorf(xx) as usize).min(size - 1);
    let iy = (floorf(yy) as usize).min(size - 1);
    let fx = xx - floorf(xx);
    let fy = yy - floorf(yy);
    let pixel = |px: usize, py: usize, c: usize| {
        data[offset + ((py % size) * size + px % size) * 3 + c] as f32 / 255.0
    };
    let mut rgb = [0.0; 3];
    for (c, v) in rgb.iter_mut().enumerate() {
        *v = mix(
            mix(pixel(ix, iy, c), pixel(ix + 1, iy, c), fx),
            mix(pixel(ix, iy + 1, c), pixel(ix + 1, iy + 1, c), fx),
            fy,
        );
    }
    rgb
}

fn sample(data: &[u8], x: f32, y: f32, lod: f32) -> [f32; 3] {
    let lod = lod.clamp(0.0, (LEVELS - 1) as f32);
    let low = floorf(lod) as usize;
    let a = level_sample(data, x, y, low);
    if lod - (low as f32) < 0.001 || low == LEVELS - 1 {
        return a;
    }
    let b = level_sample(data, x, y, low + 1);
    [
        mix(a[0], b[0], lod - low as f32),
        mix(a[1], b[1], lod - low as f32),
        mix(a[2], b[2], lod - low as f32),
    ]
}

fn number(job: &Job, key: &str, default: f32, lo: f32, hi: f32) -> f32 {
    crate::num(job.json, key, default).clamp(lo, hi)
}

pub(super) fn render(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let coat = match crate::find_value(job.json, b"animal_fur_pattern").map(crate::parse_str) {
        Some(b"giraffe") => 1,
        Some(b"leopard") => 2,
        Some(b"zebra") => 3,
        Some(b"tiger") => 4,
        Some(b"cheetah") => 5,
        _ => 0,
    };
    let data = COATS[coat];
    let scale = number(job, "animal_fur_scale", 38.0, 4.0, 160.0);
    let detail = number(job, "animal_fur_detail", 58.0, 0.0, 100.0) / 58.0;
    let length = number(job, "animal_fur_length", 12.0, 1.0, 60.0);
    let density = number(job, "animal_fur_density", 65.0, 10.0, 100.0) / 65.0;
    let flow = number(job, "animal_fur_flow", 25.0, 0.0, 100.0) / 100.0;
    let relief = number(job, "animal_fur_relief", 35.0, 0.0, 100.0) / 100.0;
    let angle = number(job, "animal_fur_direction", 90.0, -360.0, 360.0).to_radians();
    let light = number(job, "animal_fur_lighting", 315.0, -360.0, 360.0).to_radians();
    let edge = number(job, "animal_fur_edge", 0.0, 0.0, 30.0);
    let seed = number(job, "animal_fur_seed", 7.0, 0.0, 9999.0) as u32;
    let natural = super::textures::read_bool(job, "animal_fur", "natural_colors", true);
    let preserve = super::textures::read_bool(job, "animal_fur", "preserve_alpha", false);
    let light_color = super::textures::read_colour(job, "animal_fur", "light", [0.84, 0.68, 0.26]);
    let dark_color = super::textures::read_colour(job, "animal_fur", "dark", [0.14, 0.09, 0.04]);
    let source = if preserve && edge > 0.0 {
        match Surface::new(job) {
            Some(s) => Some(s),
            None => return 3,
        }
    } else {
        None
    };
    let (ox, oy, cw, ch) = canvas(job);
    let factor = super::textures::adaptive_scale(job, "animal_fur");
    let (ca, sa) = (cosf(angle), sinf(angle));
    let (lx, ly) = (cosf(light), sinf(light));
    let shift_x = hash(seed.wrapping_mul(37)) * SIZE as f32;
    let shift_y = hash(seed.wrapping_mul(71)) * SIZE as f32;
    let k = SIZE as f32 / (scale * 8.0);
    let hair_x = 2.5 * density;
    let hair_y = 2.5 * 12.0 / length;
    let lod = log2f((k / factor).max(1.0));
    let hair_lod = log2f((hair_x.max(hair_y) / factor).max(1.0));
    let has_alpha = job.format & crate::HAS_ALPHA != 0;
    let (left, top, right, bottom) = output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let px = ox + x as f32 + 0.5 - cw * 0.5;
            let py = oy + y as f32 + 0.5 - ch * 0.5;
            let u = (px * ca + py * sa) / factor;
            let v = (-px * sa + py * ca) / factor;
            // Smooth curl changes both strand flow and pigment edges, independent
            // of image dimensions and host tile/band scheduling.
            let curl = (noise(u / 74.0, v / 61.0, seed + 81) - 0.5) * flow * 36.0;
            let bend = (noise(u / 137.0, v / 121.0, seed + 82) - 0.5) * flow * 24.0;
            let a = -(v + curl) * k + shift_x;
            let b = (u + bend) * k + shift_y;
            let base = sample(data, a, b, (lod + 1.4).max(2.0));
            let hx = -(v + curl) * hair_x + shift_x;
            let hy = (u + bend) * hair_y + shift_y;
            let fine = luminance(sample(data, hx, hy, hair_lod));
            let soft = luminance(sample(data, hx, hy, (hair_lod + 1.4).max(2.0)));
            // Division removes the pigment from the hair layer, retaining its
            // local strand contrast. Limit it near almost-black pigment borders.
            let fibre = ((fine + 0.07) / (soft + 0.07)).clamp(0.5, 1.8);
            let relief_x = luminance(sample(data, hx + 1.5, hy, hair_lod)) - fine;
            let relief_y = luminance(sample(data, hx, hy + 1.5, hair_lod)) - fine;
            let light_across = -lx * sa + ly * ca;
            let light_along = -lx * ca - ly * sa;
            let diffuse = 1.0 + (relief_x * light_across + relief_y * light_along) * relief * 3.5;
            let glint = (relief_x * light_across + relief_y * light_along).max(0.0) * relief * 0.32;
            let brightness = mix(1.0, fibre, detail) * diffuse;
            let mut rgb = base;
            if !natural {
                let value = ((luminance(base) - 0.035) / 0.72).clamp(0.0, 1.0);
                for c in 0..3 {
                    rgb[c] = mix(dark_color[c], light_color[c], value);
                }
            }
            for value in &mut rgb {
                *value = (*value * brightness + glint).clamp(0.0, 1.0);
            }
            let index = y * job.width + x;
            let mut alpha = if preserve && has_alpha {
                job.px[index * job.channels + job.channels - 1]
            } else {
                1.0
            };
            if let Some(src) = &source {
                // Extend tiny, tapered code-generated guard hairs from existing
                // opacity. Max reach 30px fits the 64px host overlap.
                let strand = noise((v + curl) * 1.35, u / (length + 1.0), seed + 95);
                let reach = edge * (0.18 + 0.82 * strand);
                for tap in 1..=6 {
                    let t = tap as f32 / 6.0;
                    let cover =
                        src.sample_alpha(x as f32 - ca * reach * t, y as f32 - sa * reach * t);
                    let taper = (1.0 - t * 0.85) * ((strand - 0.35) * 2.4).clamp(0.0, 1.0);
                    alpha = alpha.max(cover * taper);
                }
            }
            write(job, index, [rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_mips_are_complete_and_periodic() {
        let expected = (0..LEVELS)
            .map(|l| (SIZE >> l) * (SIZE >> l) * 3)
            .sum::<usize>();
        for data in COATS {
            assert_eq!(data.len(), expected);
            for lod in [0.0, 1.5, 6.0, 9.0] {
                let a = sample(data, -23.25, 38.75, lod);
                let b = sample(data, 744.75, 806.75, lod);
                for c in 0..3 {
                    assert!((a[c] - b[c]).abs() < 0.0001);
                }
            }
        }
    }
    #[test]
    fn filtering_reduces_subpixel_texture_energy() {
        let data = COATS[0];
        let mut sharp = 0.0;
        let mut smooth = 0.0;
        for x in 0..500 {
            sharp += (luminance(sample(data, x as f32, 145.0, 0.0))
                - luminance(sample(data, x as f32 + 1.0, 145.0, 0.0)))
            .abs();
            smooth += (luminance(sample(data, x as f32, 145.0, 4.0))
                - luminance(sample(data, x as f32 + 1.0, 145.0, 4.0)))
            .abs();
        }
        assert!(smooth < sharp * 0.3);
    }

    #[test]
    fn severe_minification_reaches_a_single_filtered_texel() {
        for data in COATS {
            let average = sample(data, 0.0, 0.0, 20.0);
            for (x, y) in [(143.0, -271.0), (9245.5, 618.0), (-173.0, 739.0)] {
                let value = sample(data, x, y, 20.0);
                for c in 0..3 {
                    assert!((average[c] - value[c]).abs() < 0.00001);
                }
            }
        }
    }
}

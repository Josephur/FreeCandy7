//! Shared, deterministic imaging primitives. Spatial reads are bounded by the manifest overlap.
use crate::{HAS_ALPHA, Job, alloc, num};
use libm::{floorf, sqrtf};

pub fn hash(mut n: u32) -> f32 {
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846ca68b);
    n ^= n >> 16;
    (n >> 8) as f32 / 16777216.0
}
pub fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a).max(0.00001)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let ix = floorf(x);
    let iy = floorf(y);
    let tx = x - ix;
    let ty = y - iy;
    let h = |a: i32, b: i32| {
        hash((a as u32).wrapping_mul(0x9e3779b9) ^ (b as u32).wrapping_mul(0x85ebca6b) ^ seed)
    };
    let a = ix as i32;
    let b = iy as i32;
    mix(
        mix(h(a, b), h(a.wrapping_add(1), b), tx * tx * (3.0 - 2.0 * tx)),
        mix(
            h(a, b.wrapping_add(1)),
            h(a.wrapping_add(1), b.wrapping_add(1)),
            tx * tx * (3.0 - 2.0 * tx),
        ),
        ty * ty * (3.0 - 2.0 * ty),
    )
}
pub fn fbm(x: f32, y: f32, seed: u32) -> f32 {
    (noise(x, y, seed) * 0.5
        + noise(x * 2.03, y * 2.03, seed.wrapping_add(1)) * 0.25
        + noise(x * 4.09, y * 4.09, seed.wrapping_add(2)) * 0.125
        + noise(x * 8.17, y * 8.17, seed.wrapping_add(3)) * 0.0625)
        / 0.9375
}
pub fn supported(job: &Job) -> bool {
    let mode = (job.format >> 16) & 255;
    let colors = job
        .channels
        .saturating_sub(usize::from(job.format & HAS_ALPHA != 0));
    ((matches!(mode, 0 | 1 | 8) && colors == 1) || (matches!(mode, 2 | 3) && colors == 3))
        && job
            .width
            .checked_mul(job.height)
            .and_then(|n| n.checked_mul(job.channels))
            == Some(job.px.len())
}
pub fn canvas(job: &Job) -> (f32, f32, f32, f32) {
    (
        num(job.json, "x", 0.0),
        num(job.json, "y", 0.0),
        num(job.json, "canvasWidth", job.width as f32).max(1.0),
        num(job.json, "canvasHeight", job.height as f32).max(1.0),
    )
}
/// The host discards its halo after rendering; retain it as source context only.
pub fn output_bounds(job: &Job) -> (usize, usize, usize, usize) {
    // Opted-in hosts may omit repeated canvas-edge pixels in large contexts.
    // The source sampler already clamps there; these offsets identify the core.
    if crate::find_value(job.json, b"outputWidth").is_some() {
        let x = num(job.json, "outputX", 0.0).max(0.0) as usize;
        let y = num(job.json, "outputY", 0.0).max(0.0) as usize;
        let w = num(job.json, "outputWidth", 0.0).max(0.0) as usize;
        let h = num(job.json, "outputHeight", 0.0).max(0.0) as usize;
        return (
            x.min(job.width),
            y.min(job.height),
            x.saturating_add(w).min(job.width),
            y.saturating_add(h).min(job.height),
        );
    }
    let pad = num(job.json, "overlap", 0.0).max(0.0) as usize;
    if pad.saturating_mul(2) >= job.width || pad.saturating_mul(2) >= job.height {
        return (0, 0, job.width, job.height);
    }
    (pad, pad, job.width - pad, job.height - pad)
}

pub fn over(top: [f32; 4], bottom: [f32; 4]) -> [f32; 4] {
    let ta = top[3].clamp(0.0, 1.0);
    let ba = bottom[3].clamp(0.0, 1.0);
    let a = ta + ba * (1.0 - ta);
    if a <= 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0, 0.0, 0.0, a];
    for c in 0..3 {
        out[c] = (top[c] * ta + bottom[c] * ba * (1.0 - ta)) / a;
    }
    out
}
/// Write straight RGB(A) into the document's RGB or grayscale sample layout.
pub fn write(job: &mut Job, index: usize, rgba: [f32; 4]) {
    let start = index * job.channels;
    let alpha = job.format & HAS_ALPHA != 0;
    let colors = job.channels - usize::from(alpha);
    if colors == 1 {
        job.px[start] = rgba[0] * 0.2126 + rgba[1] * 0.7152 + rgba[2] * 0.0722;
    } else {
        job.px[start..start + 3].copy_from_slice(&rgba[..3]);
    }
    if alpha {
        job.px[start + colors] = rgba[3].clamp(0.0, 1.0);
    }
}

pub struct Surface {
    pixels: &'static mut [f32],
    pub width: usize,
    pub height: usize,
    channels: usize,
    alpha_channel: bool,
}
impl Surface {
    pub fn new(job: &Job) -> Option<Self> {
        if !supported(job) {
            return None;
        }
        let pixels = alloc::<f32>(job.px.len())?;
        pixels.copy_from_slice(job.px);
        Some(Self {
            pixels,
            width: job.width,
            height: job.height,
            channels: job.channels,
            alpha_channel: job.format & HAS_ALPHA != 0,
        })
    }
    fn pixel(&self, x: i32, y: i32) -> [f32; 4] {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        let p = (y * self.width + x) * self.channels;
        let colors = self.channels - usize::from(self.alpha_channel);
        let a = if self.alpha_channel {
            self.pixels[p + colors].clamp(0.0, 1.0)
        } else {
            1.0
        };
        if colors == 1 {
            [self.pixels[p], self.pixels[p], self.pixels[p], a]
        } else {
            [self.pixels[p], self.pixels[p + 1], self.pixels[p + 2], a]
        }
    }
    pub fn alpha(&self, x: i32, y: i32) -> f32 {
        if !self.alpha_channel {
            return 1.0;
        }
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        self.pixels[(y * self.width + x) * self.channels + self.channels - 1].clamp(0.0, 1.0)
    }
    /// Bilinear mask sampling avoids computing color when only coverage is needed.
    pub fn sample_alpha(&self, x: f32, y: f32) -> f32 {
        if !self.alpha_channel {
            return 1.0;
        }
        let x = x.clamp(0.0, self.width as f32 - 1.0);
        let y = y.clamp(0.0, self.height as f32 - 1.0);
        let ix = x as i32;
        let iy = y as i32;
        let tx = x - ix as f32;
        let ty = y - iy as f32;
        let a = self.alpha(ix, iy);
        if tx == 0.0 && ty == 0.0 {
            return a;
        }
        mix(
            mix(a, self.alpha(ix + 1, iy), tx),
            mix(self.alpha(ix, iy + 1), self.alpha(ix + 1, iy + 1), tx),
            ty,
        )
    }
    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        let x = x.clamp(0.0, self.width as f32 - 1.0);
        let y = y.clamp(0.0, self.height as f32 - 1.0);
        let ix = floorf(x) as i32;
        let iy = floorf(y) as i32;
        let tx = x - ix as f32;
        let ty = y - iy as f32;
        if tx == 0.0 && ty == 0.0 {
            return self.pixel(ix, iy);
        }
        let p = [
            self.pixel(ix, iy),
            self.pixel(ix + 1, iy),
            self.pixel(ix, iy + 1),
            self.pixel(ix + 1, iy + 1),
        ];
        let weights = [
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ];
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[3] += p[i][3] * weights[i];
            for c in 0..3 {
                out[c] += p[i][c] * p[i][3] * weights[i];
            }
        }
        if out[3] > 0.0 {
            for c in 0..3 {
                out[c] /= out[3];
            }
        }
        out
    }
}

/// Nearest vertical silhouette edge in each column. One linear pass replaces
/// per-pixel searches proportional to the length of a drip, icicle, or snow cap.
pub struct VerticalEdges {
    rows: &'static mut [u32],
    width: usize,
    height: usize,
}
impl VerticalEdges {
    pub fn lower(src: &Surface) -> Option<Self> {
        Self::build(src, false)
    }
    pub fn upper(src: &Surface) -> Option<Self> {
        Self::build(src, true)
    }
    fn build(src: &Surface, upper: bool) -> Option<Self> {
        let rows = alloc::<u32>(src.width.checked_mul(src.height)?)?;
        let anchors = alloc::<u32>(src.width)?;
        for row in 0..src.height {
            let y = if upper { src.height - 1 - row } else { row };
            let neighbor = y as i32 + if upper { -1 } else { 1 };
            for x in 0..src.width {
                if src.alpha(x as i32, y as i32) >= 0.5 && src.alpha(x as i32, neighbor) < 0.5 {
                    anchors[x] = y as u32 + 1;
                }
                rows[y * src.width + x] = anchors[x];
            }
        }
        Some(Self {
            rows,
            width: src.width,
            height: src.height,
        })
    }
    pub fn at(&self, x: i32, y: i32) -> Option<i32> {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        self.rows[y * self.width + x]
            .checked_sub(1)
            .map(|row| row as i32)
    }
}

/// Exact Euclidean distance to the nearest opposite-alpha pixel; positive inside.
/// The saturation distance must exceed the visible radius and its normal stencil.
pub struct Distance {
    values: &'static mut [f32],
    width: usize,
    height: usize,
}
impl Distance {
    pub fn new(src: &Surface) -> Option<Self> {
        Self::with_limit(src, 64.0)
    }
    pub fn with_limit(src: &Surface, limit: f32) -> Option<Self> {
        Self::with_threshold(src, limit, 0.5)
    }
    pub fn with_threshold(src: &Surface, limit: f32, threshold: f32) -> Option<Self> {
        let limit = limit.clamp(32.0, 1536.0);
        let n = src.width.checked_mul(src.height)?;
        let values = alloc::<f32>(n)?;
        let inside = |x: usize, y: usize| src.alpha(x as i32, y as i32) >= threshold;
        for y in 0..src.height {
            for x in 0..src.width {
                let here = inside(x, y);
                let edge = (x > 0 && inside(x - 1, y) != here)
                    || (y > 0 && inside(x, y - 1) != here)
                    || (x + 1 < src.width && inside(x + 1, y) != here)
                    || (y + 1 < src.height && inside(x, y + 1) != here);
                values[y * src.width + x] = if edge { 0.5 } else { limit };
            }
        }
        // Exact separable squared Euclidean transform. Unlike a chamfer field this
        // does not introduce eight preferred normal directions into glossy materials.
        let max_len = src.width.max(src.height);
        let line = alloc::<f32>(max_len)?;
        let result = alloc::<f32>(max_len)?;
        let sites = alloc::<usize>(max_len)?;
        let cuts = alloc::<f32>(max_len + 1)?;
        for value in values.iter_mut() {
            *value = if *value < 1.0 { 0.0 } else { limit * limit };
        }
        for y in 0..src.height {
            line[..src.width].copy_from_slice(&values[y * src.width..(y + 1) * src.width]);
            squared_distance(&line[..src.width], &mut result[..src.width], sites, cuts);
            values[y * src.width..(y + 1) * src.width].copy_from_slice(&result[..src.width]);
        }
        for x in 0..src.width {
            for y in 0..src.height {
                line[y] = values[y * src.width + x];
            }
            squared_distance(&line[..src.height], &mut result[..src.height], sites, cuts);
            for y in 0..src.height {
                let distance = (sqrtf(result[y]) + 0.5).min(limit);
                values[y * src.width + x] = if inside(x, y) { distance } else { -distance };
            }
        }
        Some(Self {
            values,
            width: src.width,
            height: src.height,
        })
    }
    pub fn at(&self, x: i32, y: i32) -> f32 {
        self.values[y.clamp(0, self.height as i32 - 1) as usize * self.width
            + x.clamp(0, self.width as i32 - 1) as usize]
    }
    pub fn normal(&self, x: i32, y: i32) -> (f32, f32) {
        // A wider central difference reduces stair steps from the raster alpha mask.
        let dx = self.at(x - 2, y) - self.at(x + 2, y);
        let dy = self.at(x, y - 2) - self.at(x, y + 2);
        let l = sqrtf(dx * dx + dy * dy).max(0.0001);
        (dx / l, dy / l)
    }
}

/// Conservative clearance to any occupied source pixel. This is a search accelerator,
/// never a replacement for the full-resolution source mask or the rendered image.
/// Cells are aligned to document coordinates so independent bands agree on their grid.
pub struct SourceClearance {
    values: &'static mut [f32],
    width: usize,
    height: usize,
    offset_x: usize,
    offset_y: usize,
    source_width: usize,
    source_height: usize,
    first_row: usize,
    last_row: usize,
    first_column: usize,
    last_column: usize,
}
impl SourceClearance {
    const CELL: usize = 4;

    pub fn new(src: &Surface, origin: (i32, i32), limit: f32) -> Option<Self> {
        let offset_x = origin.0.rem_euclid(Self::CELL as i32) as usize;
        let offset_y = origin.1.rem_euclid(Self::CELL as i32) as usize;
        let width = (src.width + offset_x).div_ceil(Self::CELL);
        let height = (src.height + offset_y).div_ceil(Self::CELL);
        let values = alloc::<f32>(width.checked_mul(height)?)?;
        let far_distance = limit / Self::CELL as f32 + 3.0;
        let far = far_distance * far_distance;
        values.fill(far);
        let (mut first_row, mut last_row) = (src.height, 0);
        // Inspect every original alpha sample. Even a single-pixel source marks its cell.
        if src.alpha_channel {
            for y in 0..src.height {
                let mut occupied_row = false;
                let row = (y + offset_y) / Self::CELL * width;
                let pixels =
                    &src.pixels[y * src.width * src.channels..(y + 1) * src.width * src.channels];
                for (x, pixel) in pixels.chunks_exact(src.channels).enumerate() {
                    if pixel[src.channels - 1] >= 0.5 {
                        values[row + (x + offset_x) / Self::CELL] = 0.0;
                        occupied_row = true;
                    }
                }
                if occupied_row {
                    first_row = first_row.min(y);
                    last_row = y;
                }
            }
        } else {
            first_row = 0;
            last_row = src.height - 1;
            values.fill(0.0);
        }
        let (mut first_column, mut last_column) = (width, 0);
        for row in values.chunks_exact(width) {
            for (x, &value) in row.iter().enumerate() {
                if value == 0.0 {
                    first_column = first_column.min(x);
                    last_column = last_column.max(x);
                }
            }
        }
        let max_len = width.max(height);
        let line = alloc::<f32>(max_len)?;
        let result = alloc::<f32>(max_len)?;
        let sites = alloc::<usize>(max_len)?;
        let cuts = alloc::<f32>(max_len + 1)?;
        for y in 0..height {
            line[..width].copy_from_slice(&values[y * width..(y + 1) * width]);
            squared_distance(&line[..width], &mut result[..width], sites, cuts);
            values[y * width..(y + 1) * width].copy_from_slice(&result[..width]);
        }
        for x in 0..width {
            for y in 0..height {
                line[y] = values[y * width + x];
            }
            squared_distance(&line[..height], &mut result[..height], sites, cuts);
            for y in 0..height {
                // Subtract both cells' half diagonals. Every point in the query cell is
                // then at least this far from every occupied pixel in the nearest cell.
                values[y * width + x] =
                    ((sqrtf(result[y]) - core::f32::consts::SQRT_2) * Self::CELL as f32).max(0.0);
            }
        }
        Some(Self {
            values,
            width,
            height,
            offset_x,
            offset_y,
            source_width: src.width,
            source_height: src.height,
            first_row,
            last_row,
            first_column,
            last_column,
        })
    }

    pub fn source_columns(&self) -> (f32, f32) {
        (
            if self.first_column == 0 {
                f32::NEG_INFINITY
            } else {
                (self.first_column * Self::CELL) as f32 - self.offset_x as f32
            },
            if self.last_column + 1 == self.width {
                f32::INFINITY
            } else {
                ((self.last_column + 1) * Self::CELL) as f32 - self.offset_x as f32
            },
        )
    }

    pub fn source_rows(&self) -> Option<(f32, f32)> {
        (self.first_row < self.source_height).then_some((
            self.first_row as f32,
            if self.last_row + 1 == self.source_height {
                f32::INFINITY
            } else {
                self.last_row as f32
            },
        ))
    }

    pub fn at(&self, x: i32, y: i32) -> f32 {
        let x = x.clamp(0, self.source_width as i32 - 1) as usize;
        let y = y.clamp(0, self.source_height as i32 - 1) as usize;
        let column = (x + self.offset_x) / Self::CELL;
        let row = (y + self.offset_y) / Self::CELL;
        debug_assert!(column < self.width && row < self.height);
        self.values[row * self.width + column]
    }
}

/// Lower envelope of the parabolas (x-i)^2 + input[i], evaluated in linear time.
fn squared_distance(input: &[f32], output: &mut [f32], sites: &mut [usize], cuts: &mut [f32]) {
    let mut last = 0usize;
    sites[0] = 0;
    cuts[0] = f32::NEG_INFINITY;
    cuts[1] = f32::INFINITY;
    for q in 1..input.len() {
        let qf = q as f32;
        let mut p = sites[last];
        let mut cut =
            ((input[q] + qf * qf) - (input[p] + (p as f32) * (p as f32))) / (2.0 * (q - p) as f32);
        while last > 0 && cut <= cuts[last] {
            last -= 1;
            p = sites[last];
            cut = ((input[q] + qf * qf) - (input[p] + (p as f32) * (p as f32)))
                / (2.0 * (q - p) as f32);
        }
        last += 1;
        sites[last] = q;
        cuts[last] = cut;
        cuts[last + 1] = f32::INFINITY;
    }
    let mut region = 0;
    for (q, value) in output.iter_mut().enumerate() {
        while cuts[region + 1] < (q as f32) {
            region += 1;
        }
        let diff = q as f32 - sites[region] as f32;
        *value = diff * diff + input[sites[region]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_rgb_does_not_bleed_through_interpolation() {
        let mut p = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0];
        let job = Job {
            px: &mut p,
            width: 2,
            height: 1,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let s = Surface::new(&job).unwrap();
        assert_eq!(s.sample(0.5, 0.0), [0.0, 0.0, 1.0, 0.5]);
    }
    #[test]
    fn distance_is_signed_and_follows_alpha_boundary() {
        let mut p = vec![0.0; 9 * 9 * 4];
        for y in 2..7 {
            for x in 2..7 {
                p[(y * 9 + x) * 4 + 3] = 1.0;
            }
        }
        let job = Job {
            px: &mut p,
            width: 9,
            height: 9,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let s = Surface::new(&job).unwrap();
        let d = Distance::new(&s).unwrap();
        assert!((d.at(0, 0) + sqrtf(5.0) + 0.5).abs() < 0.0001);
        assert!(d.at(4, 4) > 2.0);
        assert!(d.at(0, 0) < -1.0);
        assert!(d.normal(2, 4).0 < -0.9);
    }
    #[test]
    fn large_distance_field_reaches_fifteen_hundred_pixels_without_a_false_near_edge() {
        let mut p = vec![0.0; 1800 * 4];
        p[100 * 4 + 3] = 1.0;
        let job = Job {
            px: &mut p,
            width: 1800,
            height: 1,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let source = Surface::new(&job).unwrap();
        let legacy = Distance::new(&source).unwrap();
        let large = Distance::with_limit(&source, 1504.0).unwrap();
        assert_eq!(legacy.at(1600, 0), -64.0);
        assert_eq!(large.at(1600, 0), -1499.5);
        assert_eq!(large.at(1799, 0), -1504.0);
    }
    #[test]
    fn source_clearance_never_oversteps_an_original_single_pixel_source() {
        let (width, height) = (177, 131);
        let points = [(0, 0), (16, 15), (79, 63), (114, 111), (176, 130)];
        let mut pixels = vec![0.0; width * height * 4];
        for (x, y) in points {
            pixels[(y * width + x) * 4 + 3] = 1.0;
        }
        let job = Job {
            px: &mut pixels,
            width,
            height,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let source = Surface::new(&job).unwrap();
        let field = SourceClearance::new(&source, (-19, 37), 1504.0).unwrap();
        for y in 0..height {
            for x in 0..width {
                let exact = points
                    .iter()
                    .map(|&(sx, sy)| {
                        let dx = x as f32 - sx as f32;
                        let dy = y as f32 - sy as f32;
                        sqrtf(dx * dx + dy * dy)
                    })
                    .fold(f32::INFINITY, f32::min);
                let bound = field.at(x as i32, y as i32);
                assert!(
                    bound <= exact + 0.0001,
                    "unsafe bound at {x},{y}: {bound} > {exact}"
                );
            }
        }
        assert!(
            field.at(164, 13) > 50.0,
            "empty space must allow a useful skip"
        );
    }

    #[test]
    fn source_clearance_is_aligned_across_shifted_rendering_bands() {
        let (width, height) = (337, 291);
        let points = [(154, 129), (255, 206)];
        let mut pixels = vec![0.0; width * height * 4];
        for (x, y) in points {
            pixels[(y * width + x) * 4 + 3] = 1.0;
        }
        let mut crop = vec![0.0; 231 * 197 * 4];
        for y in 0..197 {
            crop[y * 231 * 4..(y + 1) * 231 * 4].copy_from_slice(
                &pixels[((y + 53) * width + 71) * 4..((y + 53) * width + 302) * 4],
            );
        }
        let job = Job {
            px: &mut pixels,
            width,
            height,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let band = Job {
            px: &mut crop,
            width: 231,
            height: 197,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let source = Surface::new(&job).unwrap();
        let crop_source = Surface::new(&band).unwrap();
        let field = SourceClearance::new(&source, (-19, 37), 1504.0).unwrap();
        let cropped = SourceClearance::new(&crop_source, (52, 90), 1504.0).unwrap();
        for y in 53..250 {
            for x in 71..302 {
                assert_eq!(field.at(x, y), cropped.at(x - 71, y - 53));
            }
        }
    }

    #[test]
    fn vertical_edges_keep_nearest_disconnected_surfaces_and_do_not_invent_canvas_edges() {
        let mut p = vec![0.0; 12 * 4];
        for y in [0, 1, 5, 6, 10, 11] {
            p[y * 4 + 3] = 1.0;
        }
        let job = Job {
            px: &mut p,
            width: 1,
            height: 12,
            channels: 4,
            format: 8 | 256 | (3 << 16),
            json: b"{}",
        };
        let source = Surface::new(&job).unwrap();
        let lower = VerticalEdges::lower(&source).unwrap();
        let upper = VerticalEdges::upper(&source).unwrap();
        assert_eq!(lower.at(0, 0), None);
        assert_eq!(lower.at(0, 4), Some(1));
        assert_eq!(lower.at(0, 9), Some(6));
        assert_eq!(lower.at(0, 11), Some(6));
        assert_eq!(upper.at(0, 0), Some(5));
        assert_eq!(upper.at(0, 4), Some(5));
        assert_eq!(upper.at(0, 9), Some(10));
        assert_eq!(upper.at(0, 11), None);
    }
    #[test]
    fn compositing_uses_straight_alpha() {
        let p = over([1.0, 0.0, 0.0, 0.5], [0.0, 0.0, 1.0, 0.5]);
        assert_eq!(p[3], 0.75);
        assert!((p[0] - 2.0 / 3.0).abs() < 1e-6);
    }
}

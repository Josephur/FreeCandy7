//! Clean-room silhouette lighting, atmospheric effects, and bounded optical warps.

use super::Effect;
use super::common::{
    Distance, SourceClearance, Surface, canvas, fbm, hash, mix, noise, over, smoothstep, supported,
    write,
};
use crate::{Job, colour, find_value, num, parse_str, point};
use core::f32::consts::PI;
use libm::{atan2f, cosf, expf, floorf, powf, sinf, sqrtf};

pub const EFFECTS: &[Effect] = &[
    Effect {
        name: "backlight",
        params: BACKLIGHT_PARAMS,
        render: backlight,
    },
    Effect {
        name: "corona",
        params: CORONA_PARAMS,
        render: corona,
    },
    Effect {
        name: "electrify",
        params: ELECTRIFY_PARAMS,
        render: electrify,
    },
    Effect {
        name: "fire",
        params: FIRE_PARAMS,
        render: fire,
    },
    Effect {
        name: "glow",
        params: GLOW_PARAMS,
        render: glow,
    },
    Effect {
        name: "smoke",
        params: SMOKE_PARAMS,
        render: smoke,
    },
    Effect {
        name: "star",
        params: STAR_PARAMS,
        render: star,
    },
    Effect {
        name: "swirl",
        params: SWIRL_PARAMS,
        render: swirl,
    },
    Effect {
        name: "ripples",
        params: RIPPLES_PARAMS,
        render: ripples,
    },
];

const BACKLIGHT_PARAMS: &str = r##"    "backlight_radius": {"type":"number","min":4,"max":1500,"default":34,"group":"Backlight","label":"Radius","visibleWhen":{"effect":"backlight"}},
    "backlight_strength": {"type":"number","min":0,"max":100,"default":72,"group":"Backlight","label":"Strength","visibleWhen":{"effect":"backlight"}},
    "backlight_rays": {"type":"number","min":0,"max":100,"default":42,"group":"Backlight","label":"Rays","visibleWhen":{"effect":"backlight"}},
    "backlight_colour": {"type":"color","default":"#b9dfff","group":"Backlight","label":"Colour","visibleWhen":{"effect":"backlight"}},
    "backlight_seed": {"type":"int","min":0,"max":9999,"default":101,"group":"Backlight","label":"Random seed","visibleWhen":{"effect":"backlight"}}"##;
const CORONA_PARAMS: &str = r##"    "corona_radius": {"type":"number","min":8,"max":1500,"default":38,"group":"Corona","label":"Radius","visibleWhen":{"effect":"corona"}},
    "corona_rays": {"type":"number","min":0,"max":100,"default":72,"group":"Corona","label":"Rays","visibleWhen":{"effect":"corona"}},
    "corona_strength": {"type":"number","min":0,"max":100,"default":70,"group":"Corona","label":"Strength","visibleWhen":{"effect":"corona"}},
    "corona_colour": {"type":"color","default":"#ffbd53","group":"Corona","label":"Colour","visibleWhen":{"effect":"corona"}},
    "corona_seed": {"type":"int","min":0,"max":9999,"default":103,"group":"Corona","label":"Random seed","visibleWhen":{"effect":"corona"}}"##;
const ELECTRIFY_PARAMS: &str = r##"    "electrify_strength": {"type":"number","min":0,"max":100,"default":78,"group":"Electrify","label":"Strength","visibleWhen":{"effect":"electrify"}},
    "electrify_reach": {"type":"number","min":4,"max":1500,"default":38,"group":"Electrify","label":"Reach","visibleWhen":{"effect":"electrify"}},
    "electrify_density": {"type":"number","min":0,"max":100,"default":66,"group":"Electrify","label":"Density","visibleWhen":{"effect":"electrify"}},
    "electrify_colour": {"type":"color","default":"#b9eaff","group":"Electrify","label":"Colour","visibleWhen":{"effect":"electrify"}},
    "electrify_seed": {"type":"int","min":0,"max":9999,"default":107,"group":"Electrify","label":"Random seed","visibleWhen":{"effect":"electrify"}}"##;
const FIRE_PARAMS: &str = r##"    "fire_intensity": {"type":"number","min":0,"max":100,"default":75,"group":"Fire","label":"Intensity","visibleWhen":{"effect":"fire"}},
    "fire_height": {"type":"number","min":8,"max":1500,"default":42,"group":"Fire","label":"Height","visibleWhen":{"effect":"fire"}},
    "fire_detail": {"type":"number","min":0,"max":100,"default":66,"group":"Fire","label":"Detail","visibleWhen":{"effect":"fire"}},
    "fire_seed": {"type":"int","min":0,"max":9999,"default":109,"group":"Fire","label":"Random seed","visibleWhen":{"effect":"fire"}}"##;
const GLOW_PARAMS: &str = r##"    "glow_radius": {"type":"number","min":2,"max":1500,"default":27,"group":"Glow","label":"Radius","visibleWhen":{"effect":"glow"}},
    "glow_strength": {"type":"number","min":0,"max":100,"default":68,"group":"Glow","label":"Strength","visibleWhen":{"effect":"glow"}},
    "glow_colour": {"type":"color","default":"#60aaff","group":"Glow","label":"Colour","visibleWhen":{"effect":"glow"}},
    "glow_inner_colour": {"type":"color","default":"#e8f6ff","group":"Glow","label":"Inner colour","visibleWhen":{"effect":"glow"}}"##;
const SMOKE_PARAMS: &str = r##"    "smoke_density": {"type":"number","min":0,"max":100,"default":62,"group":"Smoke","label":"Density","visibleWhen":{"effect":"smoke"}},
    "smoke_height": {"type":"number","min":8,"max":1500,"default":48,"group":"Smoke","label":"Height","visibleWhen":{"effect":"smoke"}},
    "smoke_detail": {"type":"number","min":0,"max":100,"default":64,"group":"Smoke","label":"Detail","visibleWhen":{"effect":"smoke"}},
    "smoke_colour": {"type":"color","default":"#aaaeb5","group":"Smoke","label":"Colour","visibleWhen":{"effect":"smoke"}},
    "smoke_seed": {"type":"int","min":0,"max":9999,"default":113,"group":"Smoke","label":"Random seed","visibleWhen":{"effect":"smoke"}}"##;
const STAR_PARAMS: &str = r##"    "star_style": {"type":"choice","options":["polygon","flare"],"default":"polygon","group":"Star","label":"Style","visibleWhen":{"effect":"star"}},
    "star_center": {"type":"point","default":[50,50],"group":"Star","label":"Centre","visibleWhen":{"effect":"star"}},
    "star_angle": {"type":"angle","default":0,"group":"Star","label":"Angle","visibleWhen":{"effect":"star"}},
    "star_depth": {"type":"number","min":15,"max":75,"default":42,"group":"Star","label":"Depth","visibleWhen":{"effect":"star","star_style":"polygon"}},
    "star_radius": {"type":"number","min":4,"max":1500,"default":38,"group":"Star","label":"Radius","visibleWhen":{"effect":"star"}},
    "star_spikes": {"type":"int","min":3,"max":24,"default":8,"group":"Star","label":"Spikes","visibleWhen":{"effect":"star"}},
    "star_strength": {"type":"number","min":0,"max":100,"default":80,"group":"Star","label":"Strength","visibleWhen":{"effect":"star"}},
    "star_colour": {"type":"color","default":"#fff3c2","group":"Star","label":"Colour","visibleWhen":{"effect":"star"}},
    "star_seed": {"type":"int","min":0,"max":9999,"default":127,"group":"Star","label":"Random seed","visibleWhen":{"effect":"star","star_style":"polygon"}}"##;
const SWIRL_PARAMS: &str = r##"    "swirl_style": {"type":"choice","options":["marble","distort"],"default":"marble","group":"Swirl","label":"Style","visibleWhen":{"effect":"swirl"}},
    "swirl_detail": {"type":"number","min":0,"max":100,"default":62,"group":"Swirl","label":"Detail","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_colour_a": {"type":"color","default":"#63361f","group":"Swirl","label":"Base colour","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_colour_b": {"type":"color","default":"#c07d3f","group":"Swirl","label":"Secondary colour","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_vein_colour": {"type":"color","default":"#ead0a0","group":"Swirl","label":"Vein colour","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_scale": {"type":"number","min":4,"max":60,"default":26,"group":"Swirl","label":"Scale","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_seed": {"type":"int","min":0,"max":9999,"default":131,"group":"Swirl","label":"Random seed","visibleWhen":{"effect":"swirl","swirl_style":"marble"}},
    "swirl_center": {"type":"point","default":[50,50],"group":"Swirl","label":"Centre","visibleWhen":{"effect":"swirl","swirl_style":"distort"}},
    "swirl_radius": {"type":"number","min":4,"max":1500,"default":40,"group":"Swirl","label":"Radius","visibleWhen":{"effect":"swirl","swirl_style":"distort"}},
    "swirl_amount": {"type":"number","min":0,"max":100,"default":58,"group":"Swirl","label":"Amount","visibleWhen":{"effect":"swirl"}},
    "swirl_direction": {"type":"choice","options":["clockwise","counterclockwise"],"default":"clockwise","group":"Swirl","label":"Direction","visibleWhen":{"effect":"swirl","swirl_style":"distort"}}"##;
const RIPPLES_PARAMS: &str = r##"    "ripples_center": {"type":"point","default":[50,50],"group":"Ripples","label":"Centre","visibleWhen":{"effect":"ripples"}},
    "ripples_radius": {"type":"number","min":8,"max":1500,"default":42,"group":"Ripples","label":"Radius","visibleWhen":{"effect":"ripples"}},
    "ripples_amount": {"type":"number","min":0,"max":100,"default":56,"group":"Ripples","label":"Amount","visibleWhen":{"effect":"ripples"}},
    "ripples_frequency": {"type":"number","min":1,"max":8,"default":4,"group":"Ripples","label":"Frequency","visibleWhen":{"effect":"ripples"}}"##;

fn cap01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Cache the hash values at integer noise lattice sites, not the rendered effect.
/// Sampling retains the original cubic interpolation and every procedural octave.
struct NoiseLattice {
    values: &'static mut [f32],
    x: i32,
    y: i32,
    width: usize,
    height: usize,
}

struct CachedNoise {
    seed: u32,
    lattice: Option<NoiseLattice>,
}

/// Noise coordinates stay well within exact single-precision integers. For that
/// range, truncation plus a negative-fraction correction gives the same floor as
/// libm without its general IEEE exponent/mantissa path on every noise sample.
fn noise_floor(x: f32) -> f32 {
    if x > -8_388_608.0 && x < 8_388_608.0 && x != 0.0 {
        let integer = x as i32;
        (integer - i32::from((integer as f32) > x)) as f32
    } else {
        floorf(x)
    }
}

impl CachedNoise {
    fn new(bounds: (f32, f32, f32, f32), seed: u32) -> Self {
        let (x, y) = (floorf(bounds.0) as i32, floorf(bounds.1) as i32);
        let width = (floorf(bounds.2) as i32 - x + 2).max(0) as usize;
        let height = (floorf(bounds.3) as i32 - y + 2).max(0) as usize;
        // Near the original scale a lattice could approach the image's pixel count.
        // Bound auxiliary memory and use the identical uncached function if necessary.
        let lattice = width
            .checked_mul(height)
            .filter(|&n| n > 0 && n <= 1_000_000)
            .and_then(crate::alloc::<f32>)
            .map(|values| {
                for row in 0..height {
                    for column in 0..width {
                        let a = (x + column as i32) as u32;
                        let b = (y + row as i32) as u32;
                        values[row * width + column] =
                            hash(a.wrapping_mul(0x9e3779b9) ^ b.wrapping_mul(0x85ebca6b) ^ seed);
                    }
                }
                NoiseLattice {
                    values,
                    x,
                    y,
                    width,
                    height,
                }
            });
        Self { seed, lattice }
    }

    fn sample(&self, x: f32, y: f32) -> f32 {
        let Some(lattice) = &self.lattice else {
            return noise(x, y, self.seed);
        };
        let (ix, iy) = (noise_floor(x), noise_floor(y));
        let (column, row) = (ix as i32 - lattice.x, iy as i32 - lattice.y);
        if column < 0
            || row < 0
            || column as usize + 1 >= lattice.width
            || row as usize + 1 >= lattice.height
        {
            return noise(x, y, self.seed);
        }
        let (tx, ty) = (x - ix, y - iy);
        let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let p = row as usize * lattice.width + column as usize;
        mix(
            mix(lattice.values[p], lattice.values[p + 1], sx),
            mix(
                lattice.values[p + lattice.width],
                lattice.values[p + lattice.width + 1],
                sx,
            ),
            sy,
        )
    }
}

fn noise_bounds(bounds: (f32, f32, f32, f32), x: f32, y: f32) -> (f32, f32, f32, f32) {
    (bounds.0 * x, bounds.1 * y, bounds.2 * x, bounds.3 * y)
}

struct CachedFbm {
    octaves: [CachedNoise; 4],
}

impl CachedFbm {
    fn new(bounds: (f32, f32, f32, f32), seed: u32) -> Self {
        Self {
            octaves: [
                CachedNoise::new(bounds, seed),
                CachedNoise::new(noise_bounds(bounds, 2.03, 2.03), seed.wrapping_add(1)),
                CachedNoise::new(noise_bounds(bounds, 4.09, 4.09), seed.wrapping_add(2)),
                CachedNoise::new(noise_bounds(bounds, 8.17, 8.17), seed.wrapping_add(3)),
            ],
        }
    }

    fn sample(&self, x: f32, y: f32) -> f32 {
        (self.octaves[0].sample(x, y) * 0.5
            + self.octaves[1].sample(x * 2.03, y * 2.03) * 0.25
            + self.octaves[2].sample(x * 4.09, y * 4.09) * 0.125
            + self.octaves[3].sample(x * 8.17, y * 8.17) * 0.0625)
            / 0.9375
    }
}

fn cached_fbm(cache: &Option<CachedFbm>, x: f32, y: f32, seed: u32) -> f32 {
    cache
        .as_ref()
        .map_or_else(|| fbm(x, y, seed), |cache| cache.sample(x, y))
}

fn rgb(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], cap01(a)]
}
fn px(job: &Job, i: usize) -> [f32; 4] {
    let n = job.channels;
    match n {
        1 => [job.px[i], job.px[i], job.px[i], 1.0],
        2 => [job.px[i], job.px[i], job.px[i], job.px[i + 1]],
        3 => [job.px[i], job.px[i + 1], job.px[i + 2], 1.0],
        _ => [job.px[i], job.px[i + 1], job.px[i + 2], job.px[i + n - 1]],
    }
}
fn put_over(job: &mut Job, pixel: usize, top: [f32; 4]) {
    let i = pixel * job.channels;
    let bottom = px(job, i);
    write(job, pixel, over(top, bottom));
}
/// Emission adds light to existing opaque samples and supplies straight colour on transparency.
/// Ordinary source-over would darken a bright background with a coloured halo.
fn put_light(job: &mut Job, pixel: usize, tint: [f32; 3], energy: f32) {
    if energy <= 0.0001 {
        return;
    }
    let bottom = px(job, pixel * job.channels);
    let coverage = 1.0 - expf(-energy.max(0.0));
    let alpha = bottom[3] + coverage * (1.0 - bottom[3]);
    let mut lit = [0.0, 0.0, 0.0, alpha];
    for c in 0..3 {
        // Screen the premultiplied contributions, then return straight-alpha colour.
        // This also avoids a dark fringe where a light overlaps an antialiased subject edge.
        lit[c] = (1.0 - (1.0 - bottom[c] * bottom[3]) * (1.0 - tint[c] * coverage)) / alpha;
    }
    write(job, pixel, lit);
}

trait SourceDistance {
    fn at(&self, x: i32, y: i32) -> f32;
}
impl SourceDistance for Distance {
    fn at(&self, x: i32, y: i32) -> f32 {
        self.at(x, y)
    }
}

/// Large plumes need a safe empty-space search bound, not a full-resolution distance
/// transform. Source intersections still use the original bilinear alpha samples.
struct PlumeDistance<'a> {
    source: &'a Surface,
    exact: Option<Distance>,
    clearance: Option<SourceClearance>,
}
impl<'a> PlumeDistance<'a> {
    fn new(source: &'a Surface, height: f32, origin: (f32, f32)) -> Option<Self> {
        let (exact, clearance) = if height <= 60.0 {
            (Some(Distance::with_limit(source, height + 4.0)?), None)
        } else {
            (
                None,
                Some(SourceClearance::new(
                    source,
                    (origin.0 as i32, origin.1 as i32),
                    height + 4.0,
                )?),
            )
        };
        Some(Self {
            source,
            exact,
            clearance,
        })
    }
    fn source_columns(&self) -> (f32, f32) {
        self.clearance.as_ref().map_or(
            (f32::NEG_INFINITY, f32::INFINITY),
            SourceClearance::source_columns,
        )
    }
    fn source_rows(&self) -> Option<(f32, f32)> {
        if let Some(clearance) = &self.clearance {
            clearance.source_rows()
        } else {
            plume_source_rows(self.source)
        }
    }
}
impl SourceDistance for PlumeDistance<'_> {
    fn at(&self, x: i32, y: i32) -> f32 {
        if let Some(exact) = &self.exact {
            return exact.at(x, y);
        }
        if self.source.alpha(x, y) >= 0.5 {
            return 0.5;
        }
        -self
            .clearance
            .as_ref()
            .map_or(0.0, |field| field.at(x, y))
            .max(0.5)
    }
}

/// Trace a mask without skipping narrow sources. Distance-guided advances cross empty space
/// quickly; the two-pixel allowance covers raster distance and bilinear interpolation error.
fn trace_source(
    surface: &Surface,
    distance: &impl SourceDistance,
    interval: (f32, f32),
    max_speed: f32,
    origin: (f32, f32),
    path: impl Fn(f32) -> (f32, f32),
) -> Option<f32> {
    // Split the world position before translating its integer index into the buffer.
    // Adding a large halo offset to a fractional position otherwise rounds interpolation
    // differently between a replicated buffer and a clipped one.
    let coverage = |x: f32, y: f32| {
        let (ix, iy) = (floorf(x), floorf(y));
        let (tx, ty) = (x - ix, y - iy);
        let (sx, sy) = ((ix - origin.0) as i32, (iy - origin.1) as i32);
        mix(
            mix(surface.alpha(sx, sy), surface.alpha(sx + 1, sy), tx),
            mix(surface.alpha(sx, sy + 1), surface.alpha(sx + 1, sy + 1), tx),
            ty,
        )
    };
    let (mut previous, mut travel) = (interval.0, interval.0);
    loop {
        let (x, y) = path(travel);
        if coverage(x, y) >= 0.5 {
            let (mut lo, mut hi) = (previous, travel);
            for _ in 0..4 {
                let mid = (lo + hi) * 0.5;
                let (x, y) = path(mid);
                if coverage(x, y) >= 0.5 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return Some((lo + hi) * 0.5);
        }
        if travel >= interval.1 {
            return None;
        }
        let clearance = distance
            .at((floorf(x) - origin.0) as i32, (floorf(y) - origin.1) as i32)
            .abs();
        let step = (clearance - 2.0).max(0.5) / max_speed.max(1.0);
        previous = travel;
        travel = (travel + step).min(interval.1);
    }
}

/// A rising plume cannot meet alpha above/below this range. Preserve the sampler's
/// repeated lower edge if that edge is occupied; otherwise stop before empty clamped rows.
fn plume_source_rows(surface: &Surface) -> Option<(f32, f32)> {
    let mut first = None;
    let mut last = 0;
    for y in 0..surface.height {
        if (0..surface.width).any(|x| surface.alpha(x as i32, y as i32) >= 0.5) {
            first.get_or_insert(y);
            last = y;
        }
    }
    first.map(|first| {
        (
            first as f32,
            if last + 1 == surface.height {
                f32::INFINITY
            } else {
                last as f32
            },
        )
    })
}

/// Locate the emitting silhouette below a gently curved plume. Preserve the legacy sampling
/// at its original sizes; longer plumes use conservative tracing that resolves thin lettering.
fn plume_rise(
    surface: &Surface,
    distance: &impl SourceDistance,
    position: (f32, f32),
    height: f32,
    bend: f32,
    origin: (f32, f32),
    source_rows: (f32, f32),
) -> Option<f32> {
    let (x, y) = position;
    if height > 60.0 {
        let start = (source_rows.0 - y - 1.0).max(0.0);
        let end = height.min(source_rows.1 - y + 1.0);
        if start > end {
            return None;
        }
        return trace_source(
            surface,
            distance,
            (start, end),
            1.0 + 2.0 * bend.abs() / height,
            origin,
            |rise| {
                (
                    x + origin.0 - bend * (rise / height) * (rise / height),
                    y + origin.1 + rise,
                )
            },
        )
        .map(|rise| rise / height);
    }
    let step = height / 24.0;
    let coverage =
        |rise: f32| surface.sample_alpha(x - bend * (rise / height) * (rise / height), y + rise);
    for i in 1..=24 {
        let hi = i as f32 * step;
        if coverage(hi) >= 0.5 {
            let (mut lo, mut hi) = (hi - step, hi);
            for _ in 0..3 {
                let mid = (lo + hi) * 0.5;
                if coverage(mid) >= 0.5 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return Some((lo + hi) * 0.5 / height);
        }
    }
    None
}

fn centered(json: &[u8], name: &str, w: f32, h: f32, ox: f32, oy: f32) -> (f32, f32) {
    let (x, y) = point(json, name, (50.0, 50.0));
    (x * w - ox, y * h - oy)
}
fn image_size(job: &Job) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = canvas(job);
    (x, y, w, h)
}

fn backlight(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(s) = Surface::new(job) else {
        return 3;
    };
    let radius = num(job.json, "backlight_radius", 34.0).clamp(4.0, 1500.0);
    let Some(d) = Distance::with_limit(&s, radius + 4.0) else {
        return 3;
    };
    let (ox, oy, cw, ch) = image_size(job);
    let strength = num(job.json, "backlight_strength", 72.0).clamp(0.0, 100.0) / 100.0;
    let rays = num(job.json, "backlight_rays", 42.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "backlight_seed", 101.0) as u32;
    let col = colour(job.json, "backlight_colour", [0.725, 0.875, 1.0]);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let sd = d.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -radius {
                continue;
            }
            let r = -sd;
            let (vx, vy) = (x as f32 + ox - cw * 0.5, y as f32 + oy - ch * 0.5);
            let len = sqrtf(vx * vx + vy * vy).max(0.001);
            let (ux, uy) = (vx / len, vy / len);
            // Sampling the direction vector instead of a polar angle avoids a seam at +/- pi.
            let shaft = powf(fbm(ux * 19.0, uy * 19.0, seed), 3.0) * 4.2;
            let sharp = powf(noise(ux * 67.0, uy * 67.0, seed + 9), 7.0);
            let fade = 1.0 - smoothstep(radius * 0.62, radius, r);
            let mist = expf(-r / (radius * 0.26));
            // Rays must have actual source coverage behind them along the light direction.
            // This prevents a false starburst at the middle of transparent holes in a subject.
            let source_ray = if radius > 60.0 {
                if trace_source(&s, &d, (0.0, radius), 1.0, (ox, oy), |travel| {
                    (x as f32 + ox - ux * travel, y as f32 + oy - uy * travel)
                })
                .is_some()
                {
                    1.0
                } else {
                    0.0
                }
            } else {
                let mut coverage = 0.0_f32;
                for step in 1..=8 {
                    let travel = radius * step as f32 / 8.0;
                    coverage = coverage
                        .max(s.sample_alpha(x as f32 - ux * travel, y as f32 - uy * travel));
                }
                coverage
            };
            let (nx, ny) = d.normal(x as i32, y as i32);
            let facing = smoothstep(0.0, 0.45, nx * ux + ny * uy);
            let beam =
                (shaft * 0.8 + sharp * 0.28) * expf(-r / (radius * 0.58)) * source_ray * facing;
            let energy = strength * fade * (mist * (0.85 - rays * 0.35) + beam * rays * 1.6);
            let tint = mix3([1.0; 3], col, smoothstep(0.0, radius * 0.25, r));
            put_light(job, y * job.width + x, tint, energy);
        }
    }
    0
}

fn corona(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let radius = num(job.json, "corona_radius", 38.0).clamp(8.0, 1500.0);
    let Some(distance) = Distance::with_limit(&surface, radius + 4.0) else {
        return 3;
    };
    let (ox, oy, _, _) = image_size(job);
    let strength = num(job.json, "corona_strength", 70.0).clamp(0.0, 100.0) / 100.0;
    let rays = num(job.json, "corona_rays", 72.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "corona_seed", 103.0) as u32;
    let tint = colour(job.json, "corona_colour", [1.0, 0.741, 0.325]);
    let scale = (radius / 60.0).max(1.0);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let sd = distance.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -radius {
                continue;
            }
            let dist = -sd;
            let (nx, ny) = distance.normal(x as i32, y as i32);
            let (gx, gy) = ((ox + x as f32) / scale, (oy + y as f32) / scale);
            // Advect fine emission out from its nearest edge, then curl it sideways. This gives
            // separate tapered streamers instead of a solid blurred band around the silhouette.
            let curl = fbm(gx * 0.043, gy * 0.043, seed) - 0.5;
            let qx = gx - nx * (dist / scale) * 0.79 - ny * curl * (dist / scale) * 0.82;
            let qy = gy - ny * (dist / scale) * 0.79 + nx * curl * (dist / scale) * 0.82;
            let strand = fbm(qx * 0.16, qy * 0.16, seed + 5);
            let small = noise(qx * 0.46 + curl * 3.0, qy * 0.46, seed + 13);
            let rise = dist / radius;
            let length = 0.24 + strand * 0.96;
            let tongue = 1.0 - smoothstep(length * 0.47, length, rise);
            let filament = smoothstep(0.40, 0.71, strand * 0.76 + small * 0.24);
            let fade = 1.0 - smoothstep(0.8, 1.0, rise);
            let core = expf(-dist / 2.0);
            let energy = strength
                * fade
                * (core * 1.65
                    + expf(-rise * 5.0) * 0.19
                    + tongue * filament * (0.25 + rays * 2.3));
            let hot = cap01(core * 0.8 + filament * tongue * (1.0 - rise) * 0.48);
            put_light(
                job,
                y * job.width + x,
                mix3(tint, [1.0, 0.99, 0.91], hot),
                energy,
            );
        }
    }
    0
}
/// Rasterise a short plasma channel with an anti-aliased white core and Gaussian air glow.
fn arc_segment(
    field: &mut [[f32; 2]],
    width: usize,
    height: usize,
    a: (f32, f32),
    b: (f32, f32),
    power: f32,
    origin: (f32, f32),
) {
    // Rasterise long diagonals in short pieces: one large axis-aligned box wastes work
    // quadratically with reach, whereas these boxes follow only the narrow plasma channel.
    let pieces = libm::ceilf((b.0 - a.0).abs().max((b.1 - a.1).abs()) / 12.0).max(1.0) as usize;
    if pieces > 1 {
        let mut previous = a;
        for i in 1..=pieces {
            let t = i as f32 / pieces as f32;
            let next = (mix(a.0, b.0, t), mix(a.1, b.1, t));
            arc_piece(field, width, height, previous, next, power, origin);
            previous = next;
        }
    } else {
        arc_piece(field, width, height, a, b, power, origin);
    }
}

fn arc_piece(
    field: &mut [[f32; 2]],
    width: usize,
    height: usize,
    a: (f32, f32),
    b: (f32, f32),
    power: f32,
    origin: (f32, f32),
) {
    let reach = 4.0;
    let x0 = floorf(a.0.min(b.0) - origin.0 - reach).max(0.0) as usize;
    let y0 = floorf(a.1.min(b.1) - origin.1 - reach).max(0.0) as usize;
    let x1 = (libm::ceilf(a.0.max(b.0) - origin.0 + reach) as usize).min(width);
    let y1 = (libm::ceilf(a.1.max(b.1) - origin.1 + reach) as usize).min(height);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let inv_len = 1.0 / (dx * dx + dy * dy).max(0.00001);
    for y in y0..y1 {
        for x in x0..x1 {
            let (gx, gy) = (x as f32 + origin.0, y as f32 + origin.1);
            let t = cap01(((gx - a.0) * dx + (gy - a.1) * dy) * inv_len);
            let (rx, ry) = (gx - a.0 - t * dx, gy - a.1 - t * dy);
            let d2 = rx * rx + ry * ry;
            let cell = &mut field[y * width + x];
            cell[0] = cell[0].max(power * (1.0 - smoothstep(0.18, 1.15, sqrtf(d2))));
            cell[1] = cell[1].max(power * expf(-d2 * 0.23));
        }
    }
}

fn electrify(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let reach = num(job.json, "electrify_reach", 38.0).clamp(4.0, 1500.0);
    let Some(distance) = Distance::with_limit(&surface, reach + 4.0) else {
        return 3;
    };
    let Some(field) = crate::alloc::<[f32; 2]>(job.width * job.height) else {
        return 3;
    };
    let (ox, oy, cw, ch) = image_size(job);
    let strength = num(job.json, "electrify_strength", 78.0).clamp(0.0, 100.0) / 100.0;
    let density = num(job.json, "electrify_density", 66.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "electrify_seed", 107.0) as u32;
    let tint = colour(job.json, "electrify_colour", [0.725, 0.918, 1.0]);
    // Generate branching polylines at deterministic boundary pixels. Longer channels need
    // fewer roots per pixel of edge to keep their density and raster cost useful on large art.
    let root_scale = sqrtf((reach / 60.0).max(1.0));
    for y in 0..job.height {
        let gy = oy + y as f32;
        if gy < 0.0 || gy >= ch {
            continue;
        }
        for x in 0..job.width {
            let gx = ox + x as f32;
            if gx < 0.0 || gx >= cw {
                continue;
            }
            let sd = distance.at(x as i32, y as i32);
            if !(0.0..=0.6).contains(&sd) {
                continue;
            }
            let key = (gx as i32 as u32).wrapping_mul(0x9e3779b9)
                ^ (gy as i32 as u32).wrapping_mul(0x85ebca6b)
                ^ seed;
            if hash(key) > (0.025 + density * 0.115) / root_scale {
                continue;
            }
            let (nx, ny) = distance.normal(x as i32, y as i32);
            let len = (reach - 4.0).max(1.0) * (0.35 + hash(key ^ 71) * 0.51);
            let mut previous = (gx, gy);
            let mut branch_start = previous;
            for j in 1..=12 {
                let t = j as f32 / 12.0;
                let jag = (hash(key.wrapping_add(j * 523)) - 0.5) * len * 0.15;
                let side = sinf(t * 5.0 + hash(key ^ 991) * (2.0 * PI)) * len * 0.12 * t + jag;
                let next = (gx + nx * len * t - ny * side, gy + ny * len * t + nx * side);
                arc_segment(
                    field,
                    job.width,
                    job.height,
                    previous,
                    next,
                    (1.0 - t * 0.74) * (0.8 + density * 0.3),
                    (ox, oy),
                );
                if j == 6 {
                    branch_start = next;
                }
                previous = next;
            }
            let sign = if hash(key ^ 47) < 0.5 { -1.0 } else { 1.0 };
            let mut previous = branch_start;
            for j in 1..=6 {
                let t = j as f32 / 6.0;
                let side =
                    sign * len * 0.28 * t + (hash(key.wrapping_add(j * 773)) - 0.5) * len * 0.09;
                let next = (
                    branch_start.0 + nx * len * 0.35 * t - ny * side,
                    branch_start.1 + ny * len * 0.35 * t + nx * side,
                );
                arc_segment(
                    field,
                    job.width,
                    job.height,
                    previous,
                    next,
                    0.45 * (1.0 - t * 0.85),
                    (ox, oy),
                );
                previous = next;
            }
        }
    }
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let sd = distance.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -reach {
                continue;
            }
            let [core, aura] = field[y * job.width + x];
            let rim = expf(sd * 1.25) * 0.18;
            let air = expf(sd / (reach * 0.22)) * density * 0.1;
            let energy = strength * (core * 3.4 + aura * 0.7 + rim + air);
            let hot = core / (core + aura * 0.24 + rim + 0.001);
            put_light(job, y * job.width + x, mix3(tint, [1.0; 3], hot), energy);
        }
    }
    0
}
fn fire(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let intensity = num(job.json, "fire_intensity", 75.0).clamp(0.0, 100.0) / 100.0;
    if intensity == 0.0 {
        return 0;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let height = num(job.json, "fire_height", 42.0).clamp(8.0, 1500.0);
    let (ox, oy, _, _) = image_size(job);
    let Some(distance) = PlumeDistance::new(&surface, height, (ox, oy)) else {
        return 3;
    };
    let detail = num(job.json, "fire_detail", 66.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "fire_seed", 109.0) as u32;
    let Some(source_rows) = distance.source_rows() else {
        return 0;
    };
    let scale = (height / 60.0).max(1.0);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    // A curved plume bends by at most 27.5% of its height. Skip columns whose
    // paths cannot intersect any source, before evaluating procedural noise.
    let columns = distance.source_columns();
    let left = left.max((columns.0 - height * 0.275 - 1.0).max(0.0) as usize);
    let right = right.min((columns.1 + height * 0.275 + 1.0).max(0.0) as usize);
    let bounds = (
        (ox + left as f32) / scale,
        (oy + top as f32) / scale,
        (ox + right as f32) / scale,
        (oy + bottom as f32) / scale,
    );
    // At enlarged sizes the bend contributes at most16.5 scaled pixels and the
    // independent turbulence contributes5.5. Cache their entire possible domain.
    let warped_bounds = (bounds.0 - 22.0, bounds.1, bounds.2 + 22.0, bounds.3);
    let flow_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(bounds, 0.035, 0.025), seed));
    let warp_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(bounds, 0.06, 0.05), seed + 17));
    let body_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(warped_bounds, 0.10, 0.026), seed + 4));
    let lace_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(warped_bounds, 0.28, 0.065), seed + 9));
    for y in top..bottom {
        if height > 60.0
            && (y as f32 > source_rows.1 + 1.0 || y as f32 + height < source_rows.0 - 1.0)
        {
            continue;
        }
        for x in left..right {
            let sd = distance.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -height {
                continue;
            }
            let (gx, gy) = ((ox + x as f32) / scale, (oy + y as f32) / scale);
            let flow = cached_fbm(&flow_cache, gx * 0.035, gy * 0.025, seed);
            let bend = (flow - 0.5) * height * 0.55;
            let Some(rise) = plume_rise(
                &surface,
                &distance,
                (x as f32, y as f32),
                height,
                bend,
                (ox, oy),
                source_rows,
            ) else {
                continue;
            };
            let u = gx - (bend / scale) * rise
                + (cached_fbm(&warp_cache, gx * 0.06, gy * 0.05, seed + 17) - 0.5)
                    * (3.0 + rise * 8.0);
            let body = cached_fbm(&body_cache, u * 0.10, gy * 0.026, seed + 4);
            let lace = cached_fbm(&lace_cache, u * 0.28, gy * 0.065, seed + 9);
            // An intermittent thin reaction sheet surrounds a much hotter, bright interior.
            let fuel = body * 0.73 + lace * 0.27;
            let flame_height = 0.28 + fuel * 1.02;
            let front = flame_height - rise;
            let shell = smoothstep(-0.05, 0.13, front);
            let holes = smoothstep(0.19 + rise * detail * 0.25, 0.51 + rise * 0.1, fuel);
            let fade = 1.0 - smoothstep(0.82, 1.0, rise);
            let heat = cap01((front * 1.3 + fuel * 0.3) * holes);
            let warm = mix3(
                [0.92, 0.065, 0.002],
                [1.0, 0.52, 0.025],
                smoothstep(0.0, 0.45, heat),
            );
            let hot = mix3(warm, [1.0, 0.98, 0.74], smoothstep(0.35, 0.88, heat));
            let energy = intensity * shell * holes * fade * (0.85 + heat * 3.2);
            put_light(job, y * job.width + x, hot, energy);
        }
    }
    0
}
fn glow(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let radius = num(job.json, "glow_radius", 27.0).clamp(2.0, 1500.0);
    let Some(distance) = Distance::with_limit(&surface, radius + 4.0) else {
        return 3;
    };
    let strength = num(job.json, "glow_strength", 68.0).clamp(0.0, 100.0) / 100.0;
    let outer = colour(job.json, "glow_colour", [0.376, 0.667, 1.0]);
    let inner = colour(job.json, "glow_inner_colour", [0.91, 0.965, 1.0]);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let sd = distance.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -radius {
                continue;
            }
            let dist = -sd;
            let narrow = expf(-0.5 * powf(dist / (1.0 + radius * 0.075), 2.0));
            let broad = expf(-0.5 * powf(dist / (radius * 0.34), 2.0));
            let falloff = 1.0 - smoothstep(radius * 0.75, radius, dist);
            let tint = mix3(outer, inner, narrow / (narrow + broad * 0.3 + 0.001));
            put_light(
                job,
                y * job.width + x,
                tint,
                strength * falloff * (narrow * 1.7 + broad * 0.42),
            );
        }
    }
    0
}
fn smoke(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let density = num(job.json, "smoke_density", 62.0).clamp(0.0, 100.0) / 100.0;
    if density == 0.0 {
        return 0;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let height = num(job.json, "smoke_height", 48.0).clamp(8.0, 1500.0);
    let (ox, oy, _, _) = image_size(job);
    let Some(distance) = PlumeDistance::new(&surface, height, (ox, oy)) else {
        return 3;
    };
    let detail = num(job.json, "smoke_detail", 64.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "smoke_seed", 113.0) as u32;
    let tint = colour(job.json, "smoke_colour", [0.667, 0.682, 0.71]);
    let Some(source_rows) = distance.source_rows() else {
        return 0;
    };
    let scale = (height / 60.0).max(1.0);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    // A curved plume bends by at most 27.5% of its height. Skip columns whose
    // paths cannot intersect any source, before evaluating procedural noise.
    let columns = distance.source_columns();
    let left = left.max((columns.0 - height * 0.275 - 1.0).max(0.0) as usize);
    let right = right.min((columns.1 + height * 0.275 + 1.0).max(0.0) as usize);
    let bounds = (
        (ox + left as f32) / scale,
        (oy + top as f32) / scale,
        (ox + right as f32) / scale,
        (oy + bottom as f32) / scale,
    );
    let warped = (
        bounds.0 - 29.5,
        bounds.1 - 6.0,
        bounds.2 + 29.5,
        bounds.3 + 6.0,
    );
    let broad_bounds = (warped.0 - 4.0, warped.1 - 6.0, warped.2, warped.3);
    let middle_bounds = (
        warped.0 * 0.094,
        warped.1 * 0.088 - 1.0,
        warped.2 * 0.094 + 1.8,
        warped.3 * 0.088,
    );
    let fine_bounds = (
        warped.0 * 0.19,
        warped.1 * 0.17,
        warped.2 * 0.19 + 1.2,
        warped.3 * 0.17,
    );
    let flow_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(bounds, 0.03, 0.032), seed));
    let broad_cache =
        (height > 60.0).then(|| CachedFbm::new(noise_bounds(broad_bounds, 0.056, 0.060), seed + 6));
    let middle_cache = (height > 60.0).then(|| CachedNoise::new(middle_bounds, seed + 19));
    let fine_cache = (height > 60.0).then(|| CachedFbm::new(fine_bounds, seed + 10));
    for y in top..bottom {
        if height > 60.0
            && (y as f32 > source_rows.1 + 1.0 || y as f32 + height < source_rows.0 - 1.0)
        {
            continue;
        }
        for x in left..right {
            let sd = distance.at(x as i32, y as i32);
            if sd >= 0.0 || sd < -height {
                continue;
            }
            let (gx, gy) = ((ox + x as f32) / scale, (oy + y as f32) / scale);
            let flow = cached_fbm(&flow_cache, gx * 0.03, gy * 0.032, seed);
            let bend = (flow - 0.5) * height * 0.55;
            let Some(rise) = plume_rise(
                &surface,
                &distance,
                (x as f32, y as f32),
                height,
                bend,
                (ox, oy),
                source_rows,
            ) else {
                continue;
            };
            let qx =
                gx - (bend / scale) * rise + sinf(gy * 0.045 + flow * 5.0) * (4.0 + rise * 9.0);
            let qy = gy + (flow - 0.5) * 12.0;
            let broad = cached_fbm(&broad_cache, qx * 0.056, qy * 0.060, seed + 6);
            let middle = middle_cache.as_ref().map_or_else(
                || noise(qx * 0.094 + broad * 1.8, qy * 0.088 - flow, seed + 19),
                |cache| cache.sample(qx * 0.094 + broad * 1.8, qy * 0.088 - flow),
            );
            let fine = cached_fbm(&fine_cache, qx * 0.19 + broad * 1.2, qy * 0.17, seed + 10);
            // Overlapping smooth density scales form billows; absolute/ridged noise here would
            // turn the volume into bright connected veins resembling a solid cellular surface.
            let volume = broad * 0.64 + middle * 0.25 + fine * 0.11;
            let body = smoothstep(0.28 + rise * 0.12, 0.68, volume);
            let fade = 1.0 - smoothstep(0.68, 1.0, rise);
            let optical_depth = body * fade * (0.8 + density * 3.1);
            let alpha = (1.0 - expf(-optical_depth)) * density;
            if alpha < 0.002 {
                continue;
            }
            // A broad upstream density sample approximates light loss through the plume.
            // Keep surface gradients soft: smoke is a participating volume, not a bevel map.
            let upstream = cached_fbm(
                &broad_cache,
                (qx - 4.0) * 0.056,
                (qy - 6.0) * 0.060,
                seed + 6,
            );
            let transmission = expf(-smoothstep(0.26, 0.75, upstream) * 1.4);
            let lighting = (0.38
                + transmission * 0.64
                + (broad - upstream) * 0.9
                + (fine - 0.5) * detail * 0.065)
                .clamp(0.3, 1.02);
            let mut c = tint;
            for channel in &mut c {
                *channel = (*channel * lighting).clamp(0.0, 1.0);
            }
            put_over(job, y * job.width + x, rgb(c, alpha));
        }
    }
    0
}
fn star(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let (ox, oy, cw, ch) = image_size(job);
    let (cx, cy) = centered(job.json, "star_center", cw, ch, ox, oy);
    let radius = num(job.json, "star_radius", 38.0).clamp(4.0, 1500.0);
    let spikes = num(job.json, "star_spikes", 8.0).clamp(3.0, 24.0) as u32;
    let strength = num(job.json, "star_strength", 80.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "star_seed", 127.0) as u32;
    let depth = num(job.json, "star_depth", 42.0).clamp(15.0, 75.0) / 100.0;
    let angle = num(job.json, "star_angle", 0.0) * PI / 180.0;
    let flare = find_value(job.json, b"star_style").is_some_and(|v| parse_str(v) == b"flare");
    let tint = colour(job.json, "star_colour", [1.0, 0.953, 0.761]);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let r = sqrtf(dx * dx + dy * dy);
            if r > radius {
                continue;
            }
            let a0 = atan2f(dy, dx) - angle;
            let sector = 2.0 * PI / spikes as f32;
            let cycles = a0 / sector;
            let phase = cycles - floorf(cycles);
            if flare {
                // Finite-width diffraction streaks, a white optical core and a broad faint halo.
                // Width is measured perpendicular to the ray so the tip does not fan outward.
                let nearest = (phase - 0.5).abs() * sector;
                let perpendicular = r * sinf(nearest);
                let beam_width = 0.45 + radius * 0.012;
                let beam = expf(-perpendicular * perpendicular / (beam_width * beam_width));
                let taper = powf(1.0 - smoothstep(0.0, radius, r), 1.55);
                let core = expf(-r * r / (radius * radius * 0.006));
                let halo = expf(-r * r / (radius * radius * 0.10));
                let energy = strength * (core * 5.0 + beam * taper * 1.5 + halo * 0.16);
                put_light(job, y * job.width + x, mix3(tint, [1.0; 3], core), energy);
            } else {
                let triangle = 1.0 - (2.0 * phase - 1.0).abs();
                let index = floorf(a0 / sector) as i32;
                let jitter = 0.94 + hash((index as u32) ^ seed) * 0.12;
                let edge = radius * jitter * (depth + (1.0 - depth) * triangle);
                let alpha = strength * (1.0 - smoothstep(-0.75, 0.75, r - edge));
                if alpha > 0.004 {
                    let facet =
                        (0.82 + cosf(a0 - 2.35) * 0.15 + (triangle - 0.5) * 0.13).clamp(0.58, 1.0);
                    let bevel = expf(-(r - edge + 1.0).abs() * 1.6) * (0.5 + cosf(a0 + 2.35) * 0.5);
                    let c = mix3(
                        [tint[0] * facet, tint[1] * facet, tint[2] * facet],
                        [1.0; 3],
                        bevel * 0.32,
                    );
                    put_over(job, y * job.width + x, rgb(c, alpha));
                }
            }
        }
    }
    0
}
fn swirl(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(surface) = Surface::new(job) else {
        return 3;
    };
    let (ox, oy, cw, ch) = image_size(job);
    let (cx, cy) = centered(job.json, "swirl_center", cw, ch, ox, oy);
    let radius = num(job.json, "swirl_radius", 40.0).clamp(4.0, 1500.0);
    let amount = num(job.json, "swirl_amount", 58.0).clamp(0.0, 100.0) / 100.0;
    if amount <= 0.0 {
        return 0;
    }
    let scale = num(job.json, "swirl_scale", 26.0).clamp(4.0, 60.0);
    let detail = num(job.json, "swirl_detail", 62.0).clamp(0.0, 100.0) / 100.0;
    let seed = num(job.json, "swirl_seed", 131.0) as u32;
    let distort = find_value(job.json, b"swirl_style").is_some_and(|v| parse_str(v) == b"distort");
    let ccw = find_value(job.json, b"swirl_direction")
        .is_some_and(|v| parse_str(v) == b"counterclockwise");
    let sign = if ccw { -1.0 } else { 1.0 };
    let ca = colour(job.json, "swirl_colour_a", [0.388, 0.212, 0.122]);
    let cb = colour(job.json, "swirl_colour_b", [0.753, 0.490, 0.247]);
    let vein = colour(job.json, "swirl_vein_colour", [0.918, 0.816, 0.627]);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    if distort {
        for y in top..bottom {
            for x in left..right {
                let (gx, gy) = (x as f32, y as f32);
                let (rx, ry) = (gx - cx, gy - cy);
                let r = sqrtf(rx * rx + ry * ry);
                let turn = sign * amount * 2.4 * (1.0 - smoothstep(radius * 0.25, radius, r));
                let (sn, cs) = (sinf(turn), cosf(turn));
                let (mut dx, mut dy) = (cx + rx * cs - ry * sn - gx, cy + rx * sn + ry * cs - gy);
                let d = sqrtf(dx * dx + dy * dy);
                if d > 50.0 {
                    dx *= 50.0 / d;
                    dy *= 50.0 / d;
                }
                let sampled = surface.sample(gx + dx, gy + dy);
                write(job, y * job.width + x, sampled);
            }
        }
    } else {
        for y in top..bottom {
            for x in left..right {
                let gx = ox + x as f32;
                let gy = oy + y as f32;
                let mut u = gx / scale;
                let mut v = gy / scale;
                // Successive smooth curl fields stretch a pigment sheet into nested folds.
                // Fine bands follow that same sheet instead of independent speckled noise.
                for octave in 0..3 {
                    let flow = noise(u * 0.42, v * 0.42, seed + octave * 17);
                    let turn = (flow - 0.5) * (2.4 + detail * 2.0);
                    let dx = sinf(v * 0.83 + turn) * 0.88;
                    let dy = cosf(u * 0.76 - turn) * 0.88;
                    u += dx;
                    v += dy;
                }
                let broad = fbm(u * 0.6, v * 0.6, seed + 7);
                let grain = noise(gx * 0.8, gy * 0.8, seed + 23);
                let phase = u * (10.0 + detail * 17.0) + broad * 8.0;
                let bands = 0.5 + 0.5 * sinf(phase);
                let thin = powf(0.5 + 0.5 * sinf(phase * 2.07 + broad * 3.0), 14.0);
                let ridge = powf(0.5 + 0.5 * sinf(phase * 0.31 + broad * 4.0), 28.0);
                let base = cap01(0.16 + broad * 0.48 + bands * 0.30 + (grain - 0.5) * 0.035);
                let c = mix3(
                    mix3(ca, cb, base),
                    vein,
                    cap01(ridge * 0.42 + thin * (0.10 + detail * 0.18)),
                );
                let src = px(job, (y * job.width + x) * job.channels);
                write(job, y * job.width + x, over(rgb(c, amount), src));
            }
        }
    }
    0
}
fn ripples(job: &mut Job) -> i32 {
    if !supported(job) {
        return 2;
    }
    let Some(s) = Surface::new(job) else {
        return 3;
    };
    let (ox, oy, cw, ch) = image_size(job);
    let (cx, cy) = centered(job.json, "ripples_center", cw, ch, ox, oy);
    let radius = num(job.json, "ripples_radius", 42.0).clamp(8.0, 1500.0);
    let amount = num(job.json, "ripples_amount", 56.0).clamp(0.0, 100.0) / 100.0;
    let freq = num(job.json, "ripples_frequency", 4.0).clamp(1.0, 8.0);
    let (left, top, right, bottom) = super::common::output_bounds(job);
    for y in top..bottom {
        for x in left..right {
            let (gx, gy) = (x as f32, y as f32);
            let (dx, dy) = (gx - cx, gy - cy);
            let r = sqrtf(dx * dx + dy * dy);
            if r >= radius || amount <= 0.0 {
                continue;
            }
            let env = (1.0 - smoothstep(radius * 0.48, radius, r)) / sqrtf(1.0 + r / radius * 2.0);
            let phase = r * freq * 0.26;
            let center_fade = smoothstep(0.0, 2.0, r);
            let wave = sinf(phase) * amount * 11.0 * env * center_fade;
            let (ux, uy) = if r > 1e-4 {
                (dx / r, dy / r)
            } else {
                (0.0, 0.0)
            };
            let mut sampled = s.sample(gx + ux * wave, gy + uy * wave);
            // Refraction follows the wave slope; glancing reflected light and focused light
            // beneath the surface keep water visible on low-detail source regions as well.
            let slope = cosf(phase) * amount * 0.84 * env * center_fade;
            let normalizer = 1.0 / sqrtf(1.0 + slope * slope);
            let (nx, ny, nz) = (
                -ux * slope * normalizer,
                -uy * slope * normalizer,
                normalizer,
            );
            let diffuse = (nx * -0.35 + ny * -0.45 + nz * 0.822).max(0.0);
            let shade = (diffuse - 0.822) * 0.7;
            let specular = powf((nx * -0.21 + ny * -0.28 + nz * 0.937).max(0.0), 55.0);
            let specular = (specular - powf(0.937, 55.0)).max(0.0) * amount * env * 0.45;
            let caustic = sinf(phase + 0.75) * amount * env * 0.055 * center_fade;
            for channel in &mut sampled[..3] {
                *channel = (*channel * (1.0 + shade + caustic) + specular).clamp(0.0, 1.0);
            }
            write(job, y * job.width + x, sampled);
        }
    }
    0
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_noise_floor_matches_libm_including_integer_edges_and_fallbacks() {
        for value in [
            f32::NEG_INFINITY,
            -f32::MAX,
            -8_388_609.0,
            -8_388_608.0,
            -8_388_607.5,
            -123_456.75,
            -1.0000001,
            -1.0,
            -0.99999994,
            -f32::MIN_POSITIVE,
            -0.0,
            0.0,
            f32::MIN_POSITIVE,
            0.99999994,
            1.0,
            1.0000001,
            123_456.75,
            8_388_607.5,
            8_388_608.0,
            8_388_609.0,
            f32::MAX,
            f32::INFINITY,
        ] {
            assert_eq!(
                noise_floor(value).to_bits(),
                floorf(value).to_bits(),
                "{value}"
            );
        }
        assert!(noise_floor(f32::NAN).is_nan());
        for i in -16_000..16_000 {
            let value = i as f32 * 0.03125 + 0.0123;
            assert_eq!(
                noise_floor(value).to_bits(),
                floorf(value).to_bits(),
                "{value}"
            );
        }
    }

    #[test]
    fn cached_noise_retains_exact_octaves_and_phase_across_negative_and_shifted_bounds() {
        for seed in [0, 109, 9999, u32::MAX - 2] {
            let broad = CachedFbm::new((-9.5, -7.5, 12.5, 11.5), seed);
            let band = CachedFbm::new((-2.1, -1.7, 5.3, 6.1), seed);
            let scalar = CachedNoise::new((-2.1, -1.7, 5.3, 6.1), seed);
            for iy in -40..48 {
                for ix in -48..52 {
                    let (x, y) = (ix as f32 * 0.2317, iy as f32 * 0.1733);
                    let expected = fbm(x, y, seed).to_bits();
                    assert_eq!(
                        broad.sample(x, y).to_bits(),
                        expected,
                        "broad ({x},{y}), seed{seed}"
                    );
                    assert_eq!(
                        band.sample(x, y).to_bits(),
                        expected,
                        "shifted/fallback ({x},{y}), seed{seed}"
                    );
                    assert_eq!(scalar.sample(x, y).to_bits(), noise(x, y, seed).to_bits());
                }
            }
        }
        let bounded = CachedNoise::new((-100000.0, -100000.0, 100000.0, 100000.0), 109);
        assert!(
            bounded.lattice.is_none(),
            "an oversized cache must use bounded-memory fallback"
        );
        assert_eq!(
            bounded.sample(0.25, -0.75).to_bits(),
            noise(0.25, -0.75, 109).to_bits()
        );
    }

    #[test]
    fn plume_source_bounds_include_thin_alpha_and_preserve_repeated_opaque_lower_edges() {
        for (source_row, alpha, expected) in [
            (7, 0.5, Some((7.0, 7.0))),
            (15, 1.0, Some((15.0, f32::INFINITY))),
            (7, 0.49, None),
        ] {
            let mut pixels = vec![0.0; 16 * 16 * 4];
            pixels[(source_row * 16 + 9) * 4 + 3] = alpha;
            let job = Job {
                px: &mut pixels,
                width: 16,
                height: 16,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json: b"{}",
            };
            let surface = Surface::new(&job).unwrap();
            assert_eq!(plume_source_rows(&surface), expected);
            assert_eq!(
                PlumeDistance::new(&surface, 1500.0, (-13.0, 37.0))
                    .unwrap()
                    .source_rows(),
                expected
            );
        }
    }

    #[test]
    fn large_plume_trace_finds_a_one_pixel_emitter_between_coarse_taps() {
        let mut pixels = vec![0.0; 16 * 1600 * 4];
        for x in 0..16 {
            pixels[(1103 * 16 + x) * 4 + 3] = 1.0;
        }
        let job = Job {
            px: &mut pixels,
            width: 16,
            height: 1600,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: b"{}",
        };
        let surface = Surface::new(&job).unwrap();
        let distance = PlumeDistance::new(&surface, 1500.0, (0.0, 0.0)).unwrap();
        let rise = plume_rise(
            &surface,
            &distance,
            (8.0, 100.0),
            1500.0,
            60.0,
            (0.0, 0.0),
            plume_source_rows(&surface).unwrap(),
        );
        assert!(
            rise.is_some(),
            "a one-pixel emitter must not fall between search taps"
        );
        assert!((rise.unwrap() - 1002.5 / 1500.0).abs() < 0.001);
    }

    #[test]
    fn large_atmosphere_controls_produce_visible_geometry_beyond_500_pixels() {
        const W: usize = 96;
        const H: usize = 1800;
        for (effect, control) in [
            ("backlight", "backlight_radius"),
            ("corona", "corona_radius"),
            ("electrify", "electrify_reach"),
            ("fire", "fire_height"),
            ("glow", "glow_radius"),
            ("smoke", "smoke_height"),
        ] {
            let mut pixels = vec![0.0; W * H * 4];
            for y in 1553..if effect == "electrify" { 1561 } else { 1554 } {
                for x in 0..W {
                    pixels[(y * W + x) * 4..][..4].copy_from_slice(&[0.2, 0.3, 0.4, 1.0]);
                }
            }
            render_fixture(
                effect,
                &std::format!("\"{control}\":1500"),
                &mut pixels,
                W,
                H,
            );
            let visible = (300..1000).any(|y| (0..W).any(|x| pixels[(y * W + x) * 4 + 3] > 0.01));
            assert!(
                visible,
                "{effect} must visibly reach more than 500 pixels from a thin emitter"
            );
            assert_eq!(
                pixels[3], 0.0,
                "{effect} must leave remote transparency clear"
            );
            assert!(pixels.iter().all(|sample| sample.is_finite()), "{effect}");
        }
    }

    #[test]
    fn large_star_and_distortion_radii_change_pixels_far_from_the_center() {
        const W: usize = 1800;
        const H: usize = 16;
        for (effect, parameters) in [
            ("star", "\"star_radius\":1500"),
            ("swirl", "\"swirl_style\":\"distort\",\"swirl_radius\":1500"),
            ("ripples", "\"ripples_radius\":1500"),
        ] {
            let mut pixels: Vec<f32> = (0..W * H)
                .flat_map(|i| [0.42, 0.42, (i / W) as f32 / H as f32, 1.0])
                .collect();
            let before = pixels.clone();
            render_fixture(effect, parameters, &mut pixels, W, H);
            let changed = (200..300).any(|x| {
                (0..3).any(|c| {
                    (pixels[(8 * W + x) * 4 + c] - before[(8 * W + x) * 4 + c]).abs() > 0.001
                })
            });
            assert!(
                changed,
                "{effect} must affect pixels over 500 pixels from its center"
            );
            assert!(pixels.iter().all(|sample| sample.is_finite()), "{effect}");
        }
    }

    #[test]
    fn publishes_nine_distinct_effects_with_namespaced_controls() {
        let names = [
            "backlight",
            "corona",
            "electrify",
            "fire",
            "glow",
            "smoke",
            "star",
            "swirl",
            "ripples",
        ];
        assert_eq!(EFFECTS.len(), names.len());
        for (effect, name) in EFFECTS.iter().zip(names) {
            assert_eq!(effect.name, name);
            assert!(!effect.params.is_empty());
            assert!(!effect.params.contains("\"radius\""));
        }
    }

    #[test]
    fn effects_render_on_transparent_subjects_and_keep_finite_samples() {
        const W: usize = 64;
        const H: usize = 64;
        for effect in EFFECTS {
            let mut pixels = vec![0.0_f32; W * H * 4];
            for y in 0..H {
                for x in 0..W {
                    let i = (y * W + x) * 4;
                    pixels[i] = x as f32 / W as f32;
                    pixels[i + 1] = y as f32 / H as f32;
                    pixels[i + 2] = 0.25;
                    let dx = x as f32 - 32.0;
                    let dy = y as f32 - 34.0;
                    pixels[i + 3] = if dx * dx + dy * dy < 14.0 * 14.0 {
                        1.0
                    } else {
                        0.0
                    };
                }
            }
            let json = b"{\"x\":0,\"y\":0,\"canvasWidth\":64,\"canvasHeight\":64}";
            let mut job = Job {
                px: &mut pixels,
                width: W,
                height: H,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json,
            };
            assert_eq!((effect.render)(&mut job), 0, "{}", effect.name);
            assert!(job.px.iter().all(|v| v.is_finite()), "{}", effect.name);
            assert!(job.px.iter().any(|v| *v > 0.0), "{}", effect.name);
        }
    }

    #[test]
    fn each_effects_primary_strength_control_changes_its_output() {
        const W: usize = 64;
        const H: usize = 64;
        let controls = [
            ("backlight", "backlight_strength"),
            ("corona", "corona_strength"),
            ("electrify", "electrify_strength"),
            ("fire", "fire_intensity"),
            ("glow", "glow_strength"),
            ("smoke", "smoke_density"),
            ("star", "star_strength"),
            ("swirl", "swirl_amount"),
            ("ripples", "ripples_amount"),
        ];
        for (effect, control) in controls {
            let input = || {
                let mut pixels = vec![0.0_f32; W * H * 4];
                for y in 0..H {
                    for x in 0..W {
                        let i = (y * W + x) * 4;
                        pixels[i] = x as f32 / W as f32;
                        pixels[i + 1] = y as f32 / H as f32;
                        pixels[i + 2] = 0.25;
                        let dx = x as f32 - 32.0;
                        let dy = y as f32 - 34.0;
                        pixels[i + 3] = if dx * dx + dy * dy < 14.0 * 14.0 {
                            1.0
                        } else {
                            0.0
                        };
                    }
                }
                pixels
            };
            let mut normal = input();
            let mut off = input();
            let normal_json = b"{\"x\":0,\"y\":0,\"canvasWidth\":64,\"canvasHeight\":64}";
            let disabled_json = std::format!(
                "{{\"x\":0,\"y\":0,\"canvasWidth\":64,\"canvasHeight\":64,\"{control}\":0}}"
            );
            let mut normal_job = Job {
                px: &mut normal,
                width: W,
                height: H,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json: normal_json,
            };
            let mut disabled_job = Job {
                px: &mut off,
                width: W,
                height: H,
                channels: 4,
                format: 8 | crate::HAS_ALPHA | (3 << 16),
                json: disabled_json.as_bytes(),
            };
            let renderer = EFFECTS.iter().find(|e| e.name == effect).unwrap().render;
            assert_eq!(renderer(&mut normal_job), 0, "{effect}");
            assert_eq!(renderer(&mut disabled_job), 0, "{effect} disabled");
            assert!(
                normal.iter().zip(&off).any(|(a, b)| (a - b).abs() > 1e-4),
                "{effect}"
            );
            assert!(normal.iter().all(|v| v.is_finite()), "{effect}");
        }
    }
    fn render_fixture(
        effect: &str,
        parameters: &str,
        pixels: &mut [f32],
        width: usize,
        height: usize,
    ) {
        let json =
            std::format!("{{\"canvasWidth\":{width},\"canvasHeight\":{height},{parameters}}}");
        let mut job = Job {
            px: pixels,
            width,
            height,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: json.as_bytes(),
        };
        let renderer = EFFECTS.iter().find(|e| e.name == effect).unwrap().render;
        assert_eq!(renderer(&mut job), 0);
    }

    #[test]
    fn water_surface_lights_a_flat_image_without_changing_alpha() {
        let mut pixels = vec![0.42; 96 * 96 * 4];
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 0.73;
        }
        render_fixture("ripples", "\"ripples_radius\":42", &mut pixels, 96, 96);
        let values: Vec<f32> = pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        let darkest = values.iter().copied().fold(1.0, f32::min);
        let brightest = values.iter().copied().fold(0.0, f32::max);
        assert!(
            darkest < 0.40 && brightest > 0.47,
            "water should reveal lit crests and shaded troughs: {darkest}..{brightest}"
        );
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| (p[3] - 0.73).abs() < 0.00001)
        );
        assert_eq!(&pixels[..4], &[0.42, 0.42, 0.42, 0.73]);
    }

    #[test]
    fn smoke_has_lit_volume_and_plumes_only_rise_from_the_subject() {
        const W: usize = 128;
        const H: usize = 128;
        for effect in ["smoke", "fire"] {
            let mut pixels = vec![0.0; W * H * 4];
            for y in 80..105 {
                for x in 22..106 {
                    let i = (y * W + x) * 4;
                    pixels[i..i + 4].copy_from_slice(&[0.2, 0.3, 0.4, 1.0]);
                }
            }
            render_fixture(
                effect,
                "\"smoke_height\":60,\"fire_height\":60",
                &mut pixels,
                W,
                H,
            );
            let mut light = Vec::new();
            for y in 0..H {
                for x in 0..W {
                    let i = (y * W + x) * 4;
                    if y < 80 && pixels[i + 3] > 0.1 {
                        light.push(pixels[i]);
                    }
                    if y >= 105 {
                        assert_eq!(
                            pixels[i + 3],
                            0.0,
                            "{effect} must rise rather than hang below the emitter"
                        );
                    }
                }
            }
            assert!(light.len() > 100, "{effect} emits a resolved plume");
            if effect == "smoke" {
                let min = light.iter().copied().fold(1.0, f32::min);
                let max = light.iter().copied().fold(0.0, f32::max);
                assert!(
                    max - min > 0.14,
                    "smoke must contain volume shading, not only varying opacity: {min}..{max}"
                );
            }
        }
    }

    #[test]
    fn disabled_atmosphere_controls_preserve_even_hidden_rgb() {
        let input: Vec<f32> = (0..64 * 64)
            .flat_map(|i| [0.2, 0.4, 0.6, if i % 13 < 5 { 1.0 } else { 0.0 }])
            .collect();
        for (effect, control) in [
            ("backlight", "backlight_strength"),
            ("corona", "corona_strength"),
            ("electrify", "electrify_strength"),
            ("fire", "fire_intensity"),
            ("glow", "glow_strength"),
            ("smoke", "smoke_density"),
            ("star", "star_strength"),
            ("swirl", "swirl_amount"),
            ("ripples", "ripples_amount"),
        ] {
            let mut pixels = input.clone();
            render_fixture(
                effect,
                &std::format!("\"{control}\":0"),
                &mut pixels,
                64,
                64,
            );
            assert_eq!(pixels, input, "{effect} at zero must be exact identity");
        }
    }

    #[test]
    fn emitted_light_never_darkens_opaque_backgrounds() {
        let mut pixels = vec![0.9, 0.8, 0.7, 1.0];
        let mut job = Job {
            px: &mut pixels,
            width: 1,
            height: 1,
            channels: 4,
            format: 8 | crate::HAS_ALPHA | (3 << 16),
            json: b"{}",
        };
        put_light(&mut job, 0, [0.02, 0.08, 0.8], 1.3);
        assert!(pixels[0] >= 0.9 && pixels[1] >= 0.8 && pixels[2] >= 0.7);
        assert_eq!(pixels[3], 1.0);
    }
    #[test]
    fn largest_atmosphere_effects_match_inside_the_64_pixel_host_halo() {
        const W: usize = 224;
        const H: usize = 192;
        const BW: usize = 160;
        const BH: usize = 160;
        const BX: usize = 24;
        const BY: usize = 16;
        let mut original = vec![0.0; W * H * 4];
        for y in 0..H {
            for x in 0..W {
                let i = (y * W + x) * 4;
                let r2 = (x as i32 - 112).pow(2) + (y as i32 - 108).pow(2);
                original[i..i + 4].copy_from_slice(&[
                    x as f32 / W as f32,
                    y as f32 / H as f32,
                    0.3,
                    if (22 * 22..55 * 55).contains(&r2) {
                        1.0
                    } else {
                        0.0
                    },
                ]);
            }
        }
        for effect in [
            "backlight",
            "corona",
            "electrify",
            "fire",
            "glow",
            "smoke",
            "ripples",
        ] {
            let settings = "\"backlight_radius\":60,\"corona_radius\":60,\"electrify_reach\":60,\"fire_height\":60,\"glow_radius\":60,\"smoke_height\":60,\"ripples_radius\":60";
            let whole_json =
                std::format!("{{\"canvasWidth\":{W},\"canvasHeight\":{H},{settings}}}");
            let band_json = std::format!(
                "{{\"x\":{BX},\"y\":{BY},\"canvasWidth\":{W},\"canvasHeight\":{H},\"overlap\":64,{settings}}}"
            );
            let mut whole = original.clone();
            let mut band = Vec::with_capacity(BW * BH * 4);
            for y in BY..BY + BH {
                band.extend_from_slice(&original[(y * W + BX) * 4..(y * W + BX + BW) * 4]);
            }
            let renderer = EFFECTS.iter().find(|e| e.name == effect).unwrap().render;
            for (pixels, width, height, json) in [
                (&mut whole, W, H, whole_json.as_bytes()),
                (&mut band, BW, BH, band_json.as_bytes()),
            ] {
                let mut job = Job {
                    px: pixels,
                    width,
                    height,
                    channels: 4,
                    format: 8 | crate::HAS_ALPHA | (3 << 16),
                    json,
                };
                assert_eq!(renderer(&mut job), 0);
            }
            let mut max_delta = 0.0_f32;
            for y in 64..BH - 64 {
                for x in 64..BW - 64 {
                    for c in 0..4 {
                        max_delta = max_delta.max(
                            (whole[((y + BY) * W + x + BX) * 4 + c] - band[(y * BW + x) * 4 + c])
                                .abs(),
                        );
                    }
                }
            }
            assert!(
                max_delta < 0.0001,
                "{effect} crosses a processing seam by {max_delta}"
            );
        }
    }
}

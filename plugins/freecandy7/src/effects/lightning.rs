//! Lightning: a branching bolt with a flash and a glow.
//!
//! The bolt geometry is built in canvas coordinates from the parameters alone, so every band of the
//! layer draws the same bolt, and every buffer that feeds a pixel (the glow grids included) is aligned
//! to canvas coordinates and summed in a fixed order, so bands join bit for bit. The bolt generator is
//! mirrored in `design/lightning-dialog.html` and `design/effect-dialog.html`.
//!
//! Pipeline:
//! 1. Geometry: each channel (trunk, branch, twig) is a polyline made by fractal midpoint
//!    displacement (Gaussian offsets, a little along-track jitter, per-level jittered decay) between
//!    its two end points. Forks are placed along a channel by arc length; each fork leaves at
//!    20–45° (times the spread setting) from the channel's heading and recurses up to four levels,
//!    thinner, dimmer and shorter each time. The trunk also gets a few thin parallel strands that
//!    split off and rejoin it. Width tapers along each channel; brightness wobbles with 1-D noise.
//! 2. Core: per segment, only the pixels in its small bounding box: a hard anti-aliased core plus a
//!    soft inner glow, both combined with `max` so joints do not double up.
//! 3. Glow: segment emission is splatted into two coarse grids aligned to canvas coordinates, then
//!    Gaussian-blurred (separable, fixed tap order) and bilinearly sampled. A medium and a wide layer
//!    together give a heavy-tailed, inverse-square-like halo at a cost independent of segment count.
//! 4. Flash: a bright, softly falling-off spot at the start point, squashed by the perspective.
//!    Everything is screen-blended onto the image.

use super::Effect;
use crate::{HAS_ALPHA, Job, alloc, colour, find_value, flag, num, parse_num, parse_str, point};
use libm::{ceilf, cosf, expf, floorf, sinf, sqrtf};

pub const EFFECT: Effect = Effect {
    name: "lightning",
    params: PARAMS,
    render,
};

/// This effect's entries for the manifest's `params` object (names, ranges and defaults).
const PARAMS: &str = r##"    "scaling": {"type":"choice","options":["adaptive","fixed"],"default":"adaptive","group":"Bolt","label":"Scaling","visibleWhen":{"effect":"lightning"}},
    "taper": {"type":"bool","default":true,"group":"Bolt","label":"Taper","visibleWhen":{"effect":"lightning"}},
    "thickness": {"type":"number","min":0,"max":500,"default":16,"group":"Bolt","label":"Thickness","visibleWhen":{"effect":"lightning"}},
    "jaggedness": {"type":"number","min":0,"max":100,"default":60,"group":"Bolt","label":"Jaggedness","visibleWhen":{"effect":"lightning"}},
    "branching": {"type":"number","min":0,"max":100,"default":60,"group":"Bolt","label":"Branching","visibleWhen":{"effect":"lightning"}},
    "spread": {"type":"number","min":0,"max":100,"default":30,"group":"Bolt","label":"Spread","visibleWhen":{"effect":"lightning"}},
    "seed": {"type":"int","min":1,"max":9999,"default":1,"group":"Bolt","label":"Random seed","visibleWhen":{"effect":"lightning"}},
    "start": {"type":"point","default":[12,12],"group":"Bolt","label":"Start point","visibleWhen":{"effect":"lightning"}},
    "end": {"type":"point","default":[88,62],"group":"Bolt","label":"End point","visibleWhen":{"effect":"lightning"}},
    "angle": {"type":"angle","default":0,"group":"Bolt","label":"Angle","visibleWhen":{"effect":"lightning"}},
    "flashSize": {"type":"number","min":0,"max":100,"default":57,"group":"Glow","label":"Flash size","visibleWhen":{"effect":"lightning"}},
    "flashPerspective": {"type":"number","min":0,"max":100,"default":20,"group":"Glow","label":"Perspective","visibleWhen":{"effect":"lightning"}},
    "glowWidth": {"type":"number","min":0,"max":100,"default":60,"group":"Glow","label":"Glow width","visibleWhen":{"effect":"lightning"}},
    "glowOpacity": {"type":"number","min":0,"max":100,"default":73,"group":"Glow","label":"Glow opacity","visibleWhen":{"effect":"lightning"}},
    "innerGlow": {"type":"color","default":"#ffffff","group":"Colour","label":"Inner glow","visibleWhen":{"effect":"lightning"}},
    "outerGlow": {"type":"color","default":"#bfefff","group":"Colour","label":"Outer glow","visibleWhen":{"effect":"lightning"}},
    "blend": {"type":"choice","options":["screen","normal"],"default":"screen","group":"Colour","label":"Blend mode","visibleWhen":{"effect":"lightning"}}"##;

/// The bolt core colour by default (the old `ice` tint's inner colour), `#ffffff`.
const INNER: [f32; 3] = [1.0, 1.0, 1.0];
/// The glow and halo colour by default (the old `ice` tint's outer colour), `#bfefff`.
const OUTER: [f32; 3] = [0.749, 0.937, 1.0];

/// Most bolt segments we keep; past this, further channels are dropped (deterministically).
const MAX_SEGS: usize = 24576;
/// Size of the polyline point stack (a channel and all its open ancestors live here at once).
const MAX_PTS: usize = 16384;
/// Deepest fork level (0 is the trunk).
const MAX_DEPTH: u32 = 4;
use core::f32::consts::PI;

struct Params {
    fixed: bool,
    taper: bool,
    thick: f32,
    jag: f32,
    branch: f32,
    spread: f32,
    seed: u32,
    start: (f32, f32),
    end: (f32, f32),
    /// Bolt angle in degrees (clockwise on screen, the host's `angle` convention): turns the end
    /// point around the start point, keeping their distance. `None` when the param is absent.
    angle: Option<f32>,
    flash: f32,
    persp: f32,
    gwidth: f32,
    gop: f32,
    inner: [f32; 3],
    outer: [f32; 3],
    /// `normal` paints the light over the image (so black light darkens); `screen` only adds light.
    normal: bool,
    /// The band's top-left corner and the canvas size, in canvas pixels.
    band: (f32, f32),
    canvas: (f32, f32),
}

impl Params {
    fn read(json: &[u8]) -> Params {
        let scaling = find_value(json, b"scaling").map_or(&b""[..], parse_str);
        Params {
            fixed: scaling == b"fixed",
            taper: flag(json, "taper", true),
            thick: num(json, "thickness", 16.0).clamp(0.0, 500.0),
            jag: (num(json, "jaggedness", 60.0) / 100.0).clamp(0.0, 1.0),
            branch: (num(json, "branching", 60.0) / 100.0).clamp(0.0, 1.0),
            spread: (num(json, "spread", 30.0) / 100.0).clamp(0.0, 1.0),
            seed: num(json, "seed", 1.0) as u32,
            start: point(json, "start", (12.0, 12.0)),
            end: point(json, "end", (88.0, 62.0)),
            angle: find_value(json, b"angle").map(parse_num),
            flash: num(json, "flashSize", 57.0).clamp(0.0, 100.0),
            persp: num(json, "flashPerspective", 20.0).clamp(0.0, 100.0),
            gwidth: num(json, "glowWidth", 60.0).clamp(0.0, 100.0),
            gop: (num(json, "glowOpacity", 73.0) / 100.0).clamp(0.0, 1.0),
            inner: colour(json, "innerGlow", INNER),
            outer: colour(json, "outerGlow", OUTER),
            normal: find_value(json, b"blend").map_or(&b""[..], parse_str) == b"normal",
            band: (num(json, "x", 0.0), num(json, "y", 0.0)),
            canvas: (
                num(json, "canvasWidth", 0.0),
                num(json, "canvasHeight", 0.0),
            ),
        }
    }
}

// ---- small maths ------------------------------------------------------------

/// A 32-bit integer hash (lowbias32) mapped to [0, 1).
fn hash01(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / 16_777_216.0
}

/// Smooth 1-D value noise in [0, 1].
fn noise1(s: f32, key: u32) -> f32 {
    let i = floorf(s);
    let f = s - i;
    let k = key.wrapping_add((i as i32) as u32);
    let a = hash01(k);
    let b = hash01(k.wrapping_add(1));
    let u = f * f * (3.0 - 2.0 * f);
    a + (b - a) * u
}

#[derive(Clone, Copy, Default)]
struct Pt {
    x: f32,
    y: f32,
}

/// A bolt segment: end points, full core width and brightness at each end.
#[derive(Clone, Copy, Default)]
struct Seg {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    w1: f32,
    w2: f32,
    b1: f32,
    b2: f32,
}

// ---- bolt generator (mirrored in design/lightning-dialog.html) ------------

struct Gen {
    segs: &'static mut [Seg],
    n: usize,
    pts: &'static mut [Pt],
    np: usize,
    /// mulberry32 state, the same generator the mock-up uses.
    state: u32,
    jag: f32,
    branch: f32,
    spread: f32,
    taper: bool,
    /// Shortest segment the subdivision produces, in pixels.
    min_len: f32,
    /// Pixel scale of the bolt (1 at "fixed").
    scale: f32,
}

impl Gen {
    fn rnd(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let a = self.state;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        (t ^ (t >> 14)) as f32 / 4_294_967_296.0
    }

    /// Roughly normal, mean 0, standard deviation 1 (sum of three uniforms).
    fn gauss(&mut self) -> f32 {
        (self.rnd() + self.rnd() + self.rnd() - 1.5) * 2.0
    }

    fn push_pt(&mut self, p: Pt) -> bool {
        if self.np < self.pts.len() {
            self.pts[self.np] = p;
            self.np += 1;
            true
        } else {
            false
        }
    }

    fn push_seg(&mut self, s: Seg) {
        if self.n < self.segs.len() {
            self.segs[self.n] = s;
            self.n += 1;
        }
    }

    /// Midpoint displacement from `a` to `b`: pushes the inner points and `b` (not `a`).
    /// The sideways offset's standard deviation is `rough` times the piece's length (so it halves
    /// with each level, with some jitter), or the large-scale `top` while that is bigger.
    fn displace(&mut self, a: Pt, b: Pt, top: f32, rough: f32, min_len: f32) {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = sqrtf(dx * dx + dy * dy);
        if len < min_len || self.np + 2 >= self.pts.len() {
            self.push_pt(b);
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let sd = (len * rough * (0.6 + 0.8 * self.rnd())).max(top);
        let off = (self.gauss() * sd).clamp(-0.3 * len, 0.3 * len);
        // A little along-track jitter so the kinks are not evenly spaced.
        let along = (self.rnd() - 0.5) * 0.2 * len;
        let mid = Pt {
            x: (a.x + b.x) * 0.5 - uy * off + ux * along,
            y: (a.y + b.y) * 0.5 + ux * off + uy * along,
        };
        let top = top * 0.45;
        self.displace(a, mid, top, rough, min_len);
        self.displace(mid, b, top, rough, min_len);
    }

    /// One channel from `a` to `b` at fork level `depth`, with core width `w0` and brightness `b0`
    /// at its root. Emits its segments, then its strands (trunk only) and its forks.
    fn channel(&mut self, a: Pt, b: Pt, w0: f32, b0: f32, depth: u32) {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let chord = sqrtf(dx * dx + dy * dy);
        if chord < 1.0 {
            return;
        }
        let start = self.np;
        if !self.push_pt(a) {
            return;
        }
        let rough = (0.08 + 0.2 * self.jag) * if depth == 0 { 1.0 } else { 1.15 };
        let top = chord * (0.03 + 0.045 * self.jag);
        let min_len = if depth == 0 {
            self.min_len
        } else {
            self.min_len * 0.7
        };
        self.displace(a, b, top, rough, min_len);
        let end = self.np;
        if end - start < 2 {
            self.np = start;
            return;
        }

        // Total arc length, for taper and fork placement.
        let mut total = 0.0;
        for k in start + 1..end {
            total += dist(self.pts[k - 1], self.pts[k]);
        }
        let key = (self.rnd() * 16_777_216.0) as u32;
        let taper_amt = if !self.taper {
            0.0
        } else if depth == 0 {
            0.72
        } else {
            0.8
        };
        let width = |u: f32| w0 * (1.0 - taper_amt * u);
        // Emit segments; brightness wobbles along the channel with two octaves of noise.
        let mut s = 0.0;
        let nscale = self.scale;
        let bri = |s: f32| {
            let n = 0.6 * noise1(s / (24.0 * nscale), key)
                + 0.4 * noise1(s / (6.0 * nscale), key ^ 0x5bd1);
            b0 * (0.55 + 0.45 * n)
        };
        let mut prev_w = width(0.0);
        let mut prev_b = bri(0.0);
        for k in start + 1..end {
            let (p, q) = (self.pts[k - 1], self.pts[k]);
            s += dist(p, q);
            let u = s / total;
            let (w, br) = (width(u), bri(s));
            self.push_seg(Seg {
                x1: p.x,
                y1: p.y,
                x2: q.x,
                y2: q.y,
                w1: prev_w,
                w2: w,
                b1: prev_b,
                b2: br,
            });
            prev_w = w;
            prev_b = br;
        }

        if depth == 0 {
            self.strands(start, end, w0);
        }

        // Forks: expected count per channel, spread along it with more toward the far end.
        if depth < MAX_DEPTH && self.branch > 0.0 {
            let expected = self.branch * [7.0, 7.0, 6.5, 5.5][depth as usize];
            let ang0 = (20.0 + 25.0 * self.spread) * (PI / 180.0);
            let (hx, hy) = (dx / chord, dy / chord);
            let mut s = 0.0;
            for k in start + 1..end - 1 {
                let seg = dist(self.pts[k - 1], self.pts[k]);
                s += seg;
                let u = s / total;
                if !(0.04..=0.93).contains(&u) {
                    continue;
                }
                let weight = 0.35 + 1.3 * u;
                if self.rnd() >= expected * weight * seg / total {
                    continue;
                }
                let p = self.pts[k];
                // Heading: mostly the channel's overall direction, partly the local tangent.
                let lo = if k >= start + 3 { k - 3 } else { start };
                let hi = if k + 3 < end { k + 3 } else { end - 1 };
                let (tx, ty) = (
                    self.pts[hi].x - self.pts[lo].x,
                    self.pts[hi].y - self.pts[lo].y,
                );
                let tl = sqrtf(tx * tx + ty * ty).max(1e-3);
                let (mut vx, mut vy) = (0.6 * hx + 0.4 * tx / tl, 0.6 * hy + 0.4 * ty / tl);
                let vl = sqrtf(vx * vx + vy * vy).max(1e-3);
                vx /= vl;
                vy /= vl;
                let side = if self.rnd() < 0.5 { -1.0 } else { 1.0 };
                let ang = side * ang0 * (0.6 + 0.8 * self.rnd());
                let (c, sn) = (cosf(ang), sinf(ang));
                let (bx, by) = (vx * c - vy * sn, vx * sn + vy * c);
                // Forks off the trunk are long; deeper forks are mostly short twigs, now and then longer.
                let r = self.rnd();
                let frac = match depth {
                    0 => 0.15 + 0.4 * r,
                    1 => 0.12 + 0.45 * r * r,
                    _ => 0.12 + 0.4 * r * r * r,
                };
                let bl = chord * frac * (1.0 - 0.5 * u);
                if bl < 3.0 * self.min_len {
                    continue;
                }
                let tip = Pt {
                    x: p.x + bx * bl,
                    y: p.y + by * bl,
                };
                let wb = (width(u) * (0.3 + 0.2 * self.rnd())).max(0.75);
                let bb = (bri(s) * (0.8 + 0.2 * self.rnd())).max(0.45);
                self.channel(p, tip, wb, bb, depth + 1);
            }
        }
        self.np = start;
    }

    /// Thin parallel strands that split off the trunk (points `start..end`) and rejoin it.
    fn strands(&mut self, start: usize, end: usize, w0: f32) {
        let n = end - start;
        if n < 16 {
            return;
        }
        let count = (self.branch * 2.2 + self.rnd()) as u32;
        for _ in 0..count {
            let k0 = start + (n as f32 * (0.15 + 0.55 * self.rnd())) as usize;
            let span = ((0.06 + 0.16 * self.rnd()) * n as f32) as usize + 6;
            let k1 = (k0 + span).min(end - 2);
            if k1 <= k0 + 4 {
                continue;
            }
            let amp =
                (2.0 + 4.0 * self.rnd()) * self.scale * if self.rnd() < 0.5 { -1.0 } else { 1.0 };
            let wb = (w0 * (0.12 + 0.1 * self.rnd())).max(0.75);
            let bb = 0.55 + 0.25 * self.rnd();
            let mut walk = 0.0;
            let mut prev = self.pts[k0];
            for k in k0 + 1..=k1 {
                let (pa, pb) = (self.pts[k - 1], self.pts[k + 1]);
                let (tx, ty) = (pb.x - pa.x, pb.y - pa.y);
                let tl = sqrtf(tx * tx + ty * ty).max(1e-3);
                let u = (k - k0) as f32 / (k1 - k0) as f32;
                walk += (self.rnd() - 0.5) * 1.6 * self.scale;
                let env = sinf(PI * u);
                let off = (amp + walk) * env;
                let p = self.pts[k];
                let q = Pt {
                    x: p.x - ty / tl * off,
                    y: p.y + tx / tl * off,
                };
                let fade = 0.35 + 0.65 * env;
                self.push_seg(Seg {
                    x1: prev.x,
                    y1: prev.y,
                    x2: q.x,
                    y2: q.y,
                    w1: wb,
                    w2: wb,
                    b1: bb * fade,
                    b2: bb * fade,
                });
                prev = q;
            }
        }
    }
}

fn dist(a: Pt, b: Pt) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    sqrtf(dx * dx + dy * dy)
}

/// The bolt's end `b` turned around its start `a` by `angle` degrees, clockwise on screen (canvas y
/// points down), keeping the start-to-end distance. Done in canvas pixels, in f32, from the two
/// points only, so every band computes the same end and bands still join. A missing angle, or one
/// that is a whole turn (0, 360, ...), returns `b` untouched: the default output is bit-identical
/// to builds without the param.
fn rotate_end(a: Pt, b: Pt, angle: Option<f32>) -> Pt {
    let Some(deg) = angle
        .map(|d| d % 360.0)
        .filter(|d| *d != 0.0 && d.is_finite())
    else {
        return b;
    };
    let (s, c) = (sinf(deg * PI / 180.0), cosf(deg * PI / 180.0));
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    Pt {
        x: a.x + dx * c - dy * s,
        y: a.y + dx * s + dy * c,
    }
}

/// Bolt geometry in canvas coordinates.
struct Bolt {
    segs: &'static [Seg],
    start: Pt,
    scale: f32,
}

fn build_bolt(p: &Params) -> Option<Bolt> {
    let (cw, ch) = p.canvas;
    let a = Pt {
        x: p.start.0 * cw,
        y: p.start.1 * ch,
    };
    let b = rotate_end(
        a,
        Pt {
            x: p.end.0 * cw,
            y: p.end.1 * ch,
        },
        p.angle,
    );
    let total = dist(a, b);
    let scale = if p.fixed {
        1.0
    } else {
        (total / 500.0).clamp(0.6, 1.6)
    };
    let mut g = Gen {
        segs: alloc::<Seg>(MAX_SEGS)?,
        n: 0,
        pts: alloc::<Pt>(MAX_PTS)?,
        np: 0,
        state: p.seed,
        jag: p.jag,
        branch: p.branch,
        spread: p.spread,
        taper: p.taper,
        // Fine detail scales with the bolt, but never coarser than a few pixels per kink.
        min_len: (total / 100.0).clamp(3.5, 16.0),
        scale,
    };
    let w0 = (0.8 + p.thick * 0.22) * scale;
    g.channel(a, b, w0, 1.0, 0);
    let n = g.n;
    Some(Bolt {
        segs: &g.segs[..n],
        start: a,
        scale,
    })
}

// ---- glow grid --------------------------------------------------------------

/// A coarse grid aligned to canvas coordinates (cell `i` covers `[i*cell, (i+1)*cell)`), holding
/// the blurred glow for a band plus the margin the blur needs.
struct Grid {
    cell: f32,
    /// First cell (canvas cell coordinates) and size of the stored region.
    gx0: i32,
    gy0: i32,
    gw: usize,
    gh: usize,
    v: &'static mut [f32],
    /// Multiplier from grid value to line-normalised glow (1 at a unit-density line).
    norm: f32,
}

impl Grid {
    /// Splats the bolt into a grid around the band and blurs it with a Gaussian of `sigma` pixels.
    /// `margin` extra cells are kept valid around the band (for sampling by a finer grid).
    fn build(segs: &[Seg], band: (f32, f32, f32, f32), sigma: f32, margin: f32) -> Option<Grid> {
        let cell = floorf(sigma / 3.0).max(1.0);
        let k = (ceilf(3.0 * sigma / cell) as i32).min(31);
        let (bx, by, bw, bh) = band;
        let m = ceilf(margin / cell) as i32;
        let gx0 = floorf(bx / cell) as i32 - 1 - k - m;
        let gy0 = floorf(by / cell) as i32 - 1 - k - m;
        let gx1 = ceilf((bx + bw) / cell) as i32 + 1 + k + m;
        let gy1 = ceilf((by + bh) / cell) as i32 + 1 + k + m;
        let (gw, gh) = ((gx1 - gx0) as usize, (gy1 - gy0) as usize);
        let src = alloc::<f32>(gw * gh)?;
        let tmp = alloc::<f32>(gw * gh)?;

        // Splat: sample each segment about once per cell, depositing bilinearly. Deposits into a
        // cell arrive in segment order, which is the same for every band.
        // (A sample up to a cell outside still reaches the edge cells bilinearly.)
        let (px0, py0) = ((gx0 - 1) as f32 * cell, (gy0 - 1) as f32 * cell);
        let (px1, py1) = ((gx1 + 1) as f32 * cell, (gy1 + 1) as f32 * cell);
        for s in segs {
            if s.x1.max(s.x2) < px0
                || s.x1.min(s.x2) > px1
                || s.y1.max(s.y2) < py0
                || s.y1.min(s.y2) > py1
            {
                continue;
            }
            let len = sqrtf((s.x2 - s.x1) * (s.x2 - s.x1) + (s.y2 - s.y1) * (s.y2 - s.y1));
            let steps = ceilf(len / cell).max(1.0);
            let ds = len / steps;
            let mut i = 0.0;
            while i < steps {
                let t = (i + 0.5) / steps;
                let x = s.x1 + (s.x2 - s.x1) * t;
                let y = s.y1 + (s.y2 - s.y1) * t;
                let w = s.w1 + (s.w2 - s.w1) * t;
                let b = s.b1 + (s.b2 - s.b1) * t;
                let e = b * (0.08 + 0.5 * w).min(6.0) * ds;
                // Weights from absolute cell coordinates, so every band rounds the same way.
                let fx = x / cell - 0.5;
                let fy = y / cell - 0.5;
                let (ix, iy) = (floorf(fx), floorf(fy));
                let (ax, ay) = (fx - ix, fy - iy);
                let (ix, iy) = (ix as i32 - gx0, iy as i32 - gy0);
                // Every in-grid cell must get its share even when its neighbour is off the grid,
                // or a band's edge cells (which the blur reads) would differ from the next band's.
                let wts = [
                    (1.0 - ax) * (1.0 - ay),
                    ax * (1.0 - ay),
                    (1.0 - ax) * ay,
                    ax * ay,
                ];
                for (c, wt) in wts.iter().enumerate() {
                    let (cx, cy) = (ix + (c & 1) as i32, iy + (c >> 1) as i32);
                    if cx >= 0 && cy >= 0 && (cx as usize) < gw && (cy as usize) < gh {
                        src[cy as usize * gw + cx as usize] += e * wt;
                    }
                }
                i += 1.0;
            }
        }

        // Normalised Gaussian taps, in cells.
        let mut taps = [0.0f32; 63];
        let mut sum = 0.0;
        for i in -k..=k {
            let d = i as f32 * cell / sigma;
            let w = expf(-0.5 * d * d);
            taps[(i + k) as usize] = w;
            sum += w;
        }
        let ku = k as usize;
        let taps = &mut taps[..2 * ku + 1];
        for t in taps.iter_mut() {
            *t /= sum;
        }
        // Horizontal pass (all rows, inner columns), then vertical (inner rows) back into src.
        // Rows that received nothing stay zero, so skip them.
        for y in 0..gh {
            let row = &src[y * gw..(y + 1) * gw];
            if row.iter().all(|&v| v == 0.0) {
                continue;
            }
            let out = &mut tmp[y * gw..(y + 1) * gw];
            for x in ku..gw - ku {
                let mut acc = 0.0;
                for (&v, &w) in row[x - ku..=x + ku].iter().zip(taps.iter()) {
                    acc += v * w;
                }
                out[x] = acc;
            }
        }
        for y in ku..gh - ku {
            for x in ku..gw - ku {
                let mut acc = 0.0;
                for (t, &w) in taps.iter().enumerate() {
                    acc += tmp[(y + t - ku) * gw + x] * w;
                }
                src[y * gw + x] = acc;
            }
        }
        let norm = 2.506_628 * sigma / (cell * cell);
        Some(Grid {
            cell,
            gx0,
            gy0,
            gw,
            gh,
            v: src,
            norm,
        })
    }

    /// Bilinear sample at canvas coordinates, line-normalised.
    fn sample(&self, x: f32, y: f32) -> f32 {
        let fx = x / self.cell - 0.5;
        let fy = y / self.cell - 0.5;
        let (ix, iy) = (floorf(fx), floorf(fy));
        let (ax, ay) = (fx - ix, fy - iy);
        let ix = ((ix as i32 - self.gx0).max(0) as usize).min(self.gw - 2);
        let iy = ((iy as i32 - self.gy0).max(0) as usize).min(self.gh - 2);
        let i = iy * self.gw + ix;
        let top = self.v[i] + (self.v[i + 1] - self.v[i]) * ax;
        let bot = self.v[i + self.gw] + (self.v[i + self.gw + 1] - self.v[i + self.gw]) * ax;
        (top + (bot - top) * ay) * self.norm
    }

    /// Mixes `wide` into this grid and tone-maps the sum into final glow strength, per cell,
    /// so each pixel needs one bilinear lookup.
    fn combine(&mut self, wide: &Grid, gain: f32) {
        let half = 0.5 * self.cell;
        for y in 0..self.gh {
            let cy = (self.gy0 + y as i32) as f32 * self.cell + half;
            for x in 0..self.gw {
                let cx = (self.gx0 + x as i32) as f32 * self.cell + half;
                let i = y * self.gw + x;
                let raw = 0.55 * self.v[i] * self.norm + 0.45 * wide.sample(cx, cy);
                self.v[i] = if raw > 1e-6 {
                    gain * (1.0 - expf(-0.55 * raw))
                } else {
                    0.0
                };
            }
        }
        self.norm = 1.0;
    }
}

// ---- rendering -------------------------------------------------------------

/// Draws the core (hard, anti-aliased) and the soft inner glow of every segment into `core` and
/// `soft` (band-sized, combined with `max`), scanning only each segment's capsule row by row.
fn draw_cores(
    segs: &[Seg],
    band: (f32, f32),
    w: usize,
    h: usize,
    core: &mut [f32],
    soft: &mut [f32],
) {
    let (bx, by) = band;
    for s in segs {
        let wmax = s.w1.max(s.w2);
        let sig_max = 0.55 + 0.4 * wmax;
        let reach = 0.5 * wmax + 0.5 + 2.83 * sig_max;
        let ylo = floorf(s.y1.min(s.y2) - reach - by).max(0.0) as usize;
        let yhi = (ceilf(s.y1.max(s.y2) + reach - by).max(0.0) as usize).min(h);
        let xlo = s.x1.min(s.x2) - reach - bx;
        let xhi = s.x1.max(s.x2) + reach - bx;
        if ylo >= yhi || xhi < 0.0 || xlo > w as f32 {
            continue;
        }
        let (sx, sy) = (s.x2 - s.x1, s.y2 - s.y1);
        let len = sqrtf(sx * sx + sy * sy);
        let (ux, uy) = if len > 1e-4 {
            (sx / len, sy / len)
        } else {
            (1.0, 0.0)
        };
        let inv_len = if len > 1e-4 { 1.0 / len } else { 0.0 };
        let r2 = reach * reach;
        let (dw, db) = (s.w2 - s.w1, s.b2 - s.b1);
        for j in ylo..yhi {
            let ry = by + j as f32 + 0.5 - s.y1;
            // Columns (as qx = pixel x - x1) where the pixel is inside the capsule's bounding
            // parallelogram: -reach <= along <= len + reach and |perp| <= reach.
            let (mut lo, mut hi) = (xlo + bx - s.x1, xhi + bx - s.x1);
            if ux.abs() > 1e-4 {
                let qa = (-reach - ry * uy) / ux;
                let qb = (len + reach - ry * uy) / ux;
                lo = lo.max(qa.min(qb));
                hi = hi.min(qa.max(qb));
            }
            if uy.abs() > 1e-4 {
                let qa = (ry * ux - reach) / uy;
                let qb = (ry * ux + reach) / uy;
                lo = lo.max(qa.min(qb));
                hi = hi.min(qa.max(qb));
            }
            let i0 = ceilf(lo + s.x1 - bx - 0.5).max(0.0);
            let i1 = (floorf(hi + s.x1 - bx - 0.5) + 1.0).min(w as f32);
            if i0 >= i1 {
                continue;
            }
            let row = j * w;
            for i in i0 as usize..i1 as usize {
                let qx = bx + i as f32 + 0.5 - s.x1;
                let along = qx * ux + ry * uy;
                let perp = ry * ux - qx * uy;
                let d2 = if along < 0.0 {
                    along * along + perp * perp
                } else if along > len {
                    (along - len) * (along - len) + perp * perp
                } else {
                    perp * perp
                };
                if d2 >= r2 {
                    continue;
                }
                let t = (along * inv_len).clamp(0.0, 1.0);
                let wd = s.w1 + dw * t;
                let b = s.b1 + db * t;
                let idx = row + i;
                let sig = 0.55 + 0.4 * wd;
                let x = d2 / (2.0 * sig * sig);
                if x < 4.0 {
                    // (1 - x/4)^4 ~ exp(-x), reaching zero smoothly.
                    let q = 1.0 - 0.25 * x;
                    let q2 = q * q;
                    let g = b * q2 * q2 * (0.3 + 0.25 * wd.min(2.0));
                    if g > soft[idx] {
                        soft[idx] = g;
                    }
                }
                let r = 0.5 * wd;
                let re = r.max(0.5);
                let e = re + 0.5;
                if d2 < e * e {
                    // Anti-aliased coverage of a line of radius r (at least half a pixel, dimmer if thinner).
                    let cov = (e - sqrtf(d2)).min(1.0) * (2.0 * r).min(1.0);
                    let c = b * cov;
                    if c > core[idx] {
                        core[idx] = c;
                    }
                }
            }
        }
    }
}

/// Adds the lightning to the pixels of this band, in place.
fn render(job: &mut Job) -> i32 {
    let p = Params::read(job.json);
    if p.canvas.0 <= 0.0 || p.canvas.1 <= 0.0 {
        return 2;
    }
    let n = job.channels;
    let px = &mut job.px[..];
    let alpha = job.format & HAS_ALPHA != 0;
    let colour = (if alpha { n - 1 } else { n }).min(4);
    // Grey + alpha or RGB + alpha: the light also goes into alpha, so it shows on transparent pixels.
    let paint_alpha = alpha && (n == 2 || n == 4);

    let Some(bolt) = build_bolt(&p) else {
        return 3;
    };
    let (w, h) = (job.width, job.height);
    let (bx, by) = (p.band.0, p.band.1);
    let (Some(core), Some(soft)) = (alloc::<f32>(w * h), alloc::<f32>(w * h)) else {
        return 3;
    };
    draw_cores(bolt.segs, (bx, by), w, h, core, soft);

    // Glow: a medium and a wide Gaussian layer give a heavy-tailed halo; the wide one is folded
    // into the medium grid so each pixel does a single lookup.
    let band = (bx, by, w as f32, h as f32);
    let sc = bolt.scale;
    let glow = if p.gop > 0.0 {
        let sm = (2.5 + p.gwidth * 0.1) * sc;
        let sw = (8.0 + p.gwidth * 0.6) * sc;
        let (Some(mut gm), Some(gw)) = (
            Grid::build(bolt.segs, band, sm, 0.0),
            Grid::build(bolt.segs, band, sw, 4.0 * sm),
        ) else {
            return 3;
        };
        gm.combine(&gw, p.gop);
        Some(gm)
    } else {
        None
    };

    // Flash at the start point, squashed vertically by the perspective setting.
    let a = bolt.start;
    let flash_r = (30.0 + p.flash * 2.6) * sc;
    let squash = 0.55 + 0.45 * (1.0 - p.persp / 100.0);
    let flash_k = if p.flash > 0.0 {
        0.25 + p.flash / 160.0
    } else {
        0.0
    };
    let inv_r2 = 1.0 / (flash_r * flash_r);
    let inv_r02 = 1.0 / (0.22 * flash_r * 0.22 * flash_r);

    // Per-channel colours: core, soft/flash (half way to the outer colour) and glow.
    let grey = |c: [f32; 3]| (c[0] + c[1] + c[2]) / 3.0;
    let mut cin = [1.0f32; 4];
    let mut cout = [1.0f32; 4];
    for ch in 0..colour {
        (cin[ch], cout[ch]) = if colour >= 3 {
            (p.inner[ch.min(2)], p.outer[ch.min(2)])
        } else {
            (grey(p.inner), grey(p.outer))
        };
    }
    let cmid = [
        0.5 * (cin[0] + cout[0]),
        0.5 * (cin[1] + cout[1]),
        0.5 * (cin[2] + cout[2]),
        0.5 * (cin[3] + cout[3]),
    ];

    for j in 0..h {
        let py = by + j as f32 + 0.5;
        let fdy = (py - a.y) / squash;
        let fdy2 = fdy * fdy;
        for i in 0..w {
            let pidx = (j * w + i) * n;
            if alpha && !paint_alpha && px[pidx + n - 1] <= 0.0 {
                continue;
            }
            let pxx = bx + i as f32 + 0.5;
            let idx = j * w + i;
            let mut flash = 0.0;
            if flash_k > 0.0 {
                let fdx = pxx - a.x;
                let d2 = fdx * fdx + fdy2;
                let e = 1.0 - d2 * inv_r2;
                if e > 0.0 {
                    flash = flash_k * e * e / (1.0 + d2 * inv_r02);
                }
            }
            let g = match &glow {
                Some(gm) => gm.sample(pxx, py),
                None => 0.0,
            };
            let c = core[idx];
            let sf = soft[idx];
            if c <= 0.0 && sf <= 0.0 && g <= 1e-5 && flash <= 0.0 {
                continue;
            }
            let sm = sf + flash;
            if p.normal {
                paint_normal(
                    &mut px[pidx..pidx + n],
                    colour,
                    paint_alpha,
                    c,
                    sm,
                    g,
                    &cin,
                    &cmid,
                    &cout,
                );
                continue;
            }
            if paint_alpha {
                paint_with_alpha(&mut px[pidx..pidx + n], c, sm, g, &cin, &cmid, &cout);
                continue;
            }
            for ch in 0..colour {
                let add = (c * cin[ch] + sm * cmid[ch] + g * cout[ch]).min(1.0);
                let base = px[pidx + ch];
                px[pidx + ch] = 1.0 - (1.0 - base) * (1.0 - add);
            }
        }
    }
    0
}

/// Paints the light over one pixel: `out = base·(1−k) + colour·k`, where `k` is the light's strength
/// and `colour` its colour. Black light therefore darkens the image. With alpha, the light also
/// raises alpha to `k` (the same rule as [`paint_with_alpha`]), so it shows on a transparent layer.
#[allow(clippy::too_many_arguments)]
fn paint_normal(
    pix: &mut [f32],
    colour: usize,
    has_alpha: bool,
    c: f32,
    sm: f32,
    g: f32,
    cin: &[f32; 4],
    cmid: &[f32; 4],
    cout: &[f32; 4],
) {
    let t = c + sm + g;
    if t <= 0.0 {
        return;
    }
    let k = t.min(1.0);
    let a_in = if has_alpha { pix[colour].max(0.0) } else { 1.0 };
    let a_out = if has_alpha { a_in.max(k) } else { 1.0 };
    for ch in 0..colour {
        let mix = ((c * cin[ch] + sm * cmid[ch] + g * cout[ch]) / t).min(1.0);
        let over = pix[ch] * (1.0 - k) + mix * k;
        pix[ch] = if a_out > a_in {
            (a_in * over + (a_out - a_in) * mix) / a_out
        } else {
            over
        };
    }
    if has_alpha {
        pix[colour] = a_out;
    }
}

/// Adds the light to one pixel whose last channel is straight alpha. The light is a layer of its own
/// colour (the tint, normalised to its brightest channel) with alpha `I`, its intensity: alpha
/// becomes `max(alpha, I)`, where the pixel was already that opaque the colour is the usual screen
/// blend, and the coverage the light adds is filled with its own colour (so a transparent pixel
/// becomes the tint, not black).
fn paint_with_alpha(
    pix: &mut [f32],
    c: f32,
    sm: f32,
    g: f32,
    cin: &[f32; 4],
    cmid: &[f32; 4],
    cout: &[f32; 4],
) {
    let colour = pix.len() - 1;
    let mut add = [0.0f32; 3];
    let mut i = 0.0f32;
    for ch in 0..colour {
        add[ch] = (c * cin[ch] + sm * cmid[ch] + g * cout[ch]).min(1.0);
        i = i.max(add[ch]);
    }
    if i <= 0.0 {
        return;
    }
    let a_in = pix[colour].max(0.0);
    let a_out = a_in.max(i);
    for ch in 0..colour {
        let screen = 1.0 - (1.0 - pix[ch]) * (1.0 - add[ch]);
        pix[ch] = if a_out > a_in {
            (a_in * screen + (a_out - a_in) * (add[ch] / i)) / a_out
        } else {
            screen
        };
    }
    pix[colour] = a_out;
}

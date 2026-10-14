//! FreeCandy7: a PhotoCraft filter plug-in (plug-in ABI v1, see photocraft `docs/plugins.md`).
//!
//! One module, one menu item: the first parameter, `effect`, picks which effect draws, and every
//! effect's own parameters follow it. The effects live in [`effects`]; this file holds the ABI
//! exports, the manifest builder, parameter parsing and the dispatch, none of which change when an
//! effect is added.
//!
//! `no_std`, no allocator, no imports. Each effect's output must be a deterministic function of its
//! parameters and absolute canvas coordinates, so that bands join seamlessly.
#![cfg_attr(target_arch = "wasm32", no_std)]

mod effects;

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

/// Everything in the manifest except the `params` object, which is built from [`effects::EFFECTS`].
const MANIFEST_HEAD: &str = r#"{
  "id": "com.stack-tech.freecandy7",
  "name": "FreeCandy7",
  "version": "0.15.0",
  "kind": "filter",
  "author": "FreeCandy7 contributors",
  "description": "Procedural texture, material, lighting and distortion effects. Select an effect to show its settings.",
  "params": {
"#;
const MANIFEST_TAIL: &str = r#"
  },
  "presetSelector": "effect",
  "presetPreserve": ["animal_fur_pattern", "animal_fur_scale", "animal_fur_scaling", "animal_fur_natural_colors", "animal_fur_light", "animal_fur_dark", "animal_fur_seed"],
  "selectionShape": {"parameter":"effect","values":["bevel","chrome","glass","extrude","shadow","motion_trail","drip","snow","icicles","backlight","corona","electrify","fire","glow","smoke","neon_tubes","holographic_foil","cracked_paint","paper_cut"]},
  "guide": {"from": "start", "to": "end"},
  "overlap": 64,
  "clampContext": true,
  "clampContextWhen": {"parameter":"effect","values":["drip","icicles","snow","shadow","extrude","backlight","corona","electrify","fire","glow","smoke","holographic_foil","paper_cut"]},
  "area": "canvas",
  "presets": "#;
const BUILTIN_PRESETS: &str = include_str!("../presets/builtins.json");
/// Room for the manifest (the host's limit is 64 KiB).
const MANIFEST_CAP: usize = 65536;
static mut MANIFEST: [u8; MANIFEST_CAP] = [0; MANIFEST_CAP];

/// Bit 8 of the `format` argument: the last channel is alpha.
pub(crate) const HAS_ALPHA: u32 = 1 << 8;

/// Error code for an `effect` value no effect answers to.
const UNKNOWN_EFFECT: i32 = 4;

/// One band to filter, as the host passed it.
pub struct Job<'a> {
    /// `width × height × channels` interleaved samples.
    pub px: &'a mut [f32],
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    /// `depth | alpha << 8 | mode << 16`.
    pub format: u32,
    /// The validated parameter JSON, including `_image`.
    pub json: &'a [u8],
}

#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> u32 {
    1
}

/// Appends `s` to `buf` at `*at`; false if it does not fit.
fn put(buf: &mut [u8], at: &mut usize, s: &str) -> bool {
    let end = *at + s.len();
    if end > buf.len() {
        return false;
    }
    buf[*at..end].copy_from_slice(s.as_bytes());
    *at = end;
    true
}

/// `len << 32 | ptr` of the UTF-8 manifest JSON: the head, the `effect` choice listing every
/// effect (the first is the default), each effect's params in turn, and the tail.
#[unsafe(no_mangle)]
pub extern "C" fn pc_manifest() -> u64 {
    // SAFETY: wasm32 here is single-threaded and this is the only place that touches MANIFEST.
    #[allow(clippy::deref_addrof)] // `&raw mut` avoids a reference to a `static mut`.
    let buf: &mut [u8; MANIFEST_CAP] = unsafe { &mut *(&raw mut MANIFEST) };
    let mut at = 0;
    let mut ok = put(buf, &mut at, MANIFEST_HEAD)
        && put(
            buf,
            &mut at,
            r#"    "effect": {"label": "Effect", "type": "choice", "options": ["#,
        );
    for (i, e) in effects::EFFECTS.iter().enumerate() {
        ok = ok
            && put(buf, &mut at, if i == 0 { "\"" } else { ", \"" })
            && put(buf, &mut at, e.name)
            && put(buf, &mut at, "\"");
    }
    ok = ok
        && put(buf, &mut at, r#"], "default": ""#)
        && put(buf, &mut at, effects::EFFECTS[0].name)
        && put(buf, &mut at, "\"}");
    for e in effects::EFFECTS {
        if !e.params.is_empty() {
            ok = ok && put(buf, &mut at, ",\n") && put(buf, &mut at, e.params);
        }
    }
    ok = ok
        && put(buf, &mut at, MANIFEST_TAIL)
        && put(buf, &mut at, BUILTIN_PRESETS)
        && put(buf, &mut at, "}");
    // A truncated manifest would be invalid JSON, so report none at all.
    let len = if ok { at } else { 0 };
    ((len as u64) << 32) | buf.as_ptr() as u64
}

/// A block of `size` bytes from fresh pages. Each run gets a new instance, so nothing is freed.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn pc_alloc(size: u32) -> u32 {
    let pages = (size as usize).div_ceil(65536);
    let old = core::arch::wasm32::memory_grow(0, pages);
    if old == usize::MAX {
        0
    } else {
        (old * 65536) as u32
    }
}

/// `count` zeroed values of `T` from fresh pages (never freed; each run gets a fresh instance).
/// `T` must be plain-old-data that is valid when all-zero.
#[cfg(target_arch = "wasm32")]
pub(crate) fn alloc<T: Default + Clone>(count: usize) -> Option<&'static mut [T]> {
    let bytes = count.checked_mul(core::mem::size_of::<T>())?;
    let pages = bytes.div_ceil(65536).max(1);
    let old = core::arch::wasm32::memory_grow(0, pages);
    if old == usize::MAX {
        return None;
    }
    // SAFETY: memory_grow just gave us `pages` fresh zeroed pages starting at `old * 65536`, which
    // is 64 KiB aligned; every `T` used here is plain-old-data valid when zeroed.
    Some(unsafe { core::slice::from_raw_parts_mut((old * 65536) as *mut T, count) })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn alloc<T: Default + Clone>(count: usize) -> Option<&'static mut [T]> {
    let mut values = std::vec::Vec::new();
    values.try_reserve_exact(count).ok()?;
    values.resize(count, T::default());
    Some(std::boxed::Box::leak(values.into_boxed_slice()))
}

/// Manifest text for native validation and tooling. The WASM ABI uses the same builder.
#[cfg(not(target_arch = "wasm32"))]
pub fn manifest_json() -> std::string::String {
    let mut result = std::string::String::from(MANIFEST_HEAD);
    result.push_str("\"effect\":{\"label\":\"Effect\",\"type\":\"choice\",\"options\":[");
    for (i, e) in effects::EFFECTS.iter().enumerate() {
        if i > 0 {
            result.push(',');
        }
        result.push('"');
        result.push_str(e.name);
        result.push('"');
    }
    result.push_str("],\"default\":\"lightning\"}");
    for e in effects::EFFECTS {
        if !e.params.is_empty() {
            result.push(',');
            result.push_str(e.params);
        }
    }
    result.push_str(MANIFEST_TAIL);
    result.push_str(BUILTIN_PRESETS);
    result.push('}');
    result
}

/// Validated native renderer entry point, also used by the WASM ABI.
pub fn filter(job: &mut Job) -> i32 {
    if job.width == 0
        || job.height == 0
        || job.channels == 0
        || job
            .width
            .checked_mul(job.height)
            .and_then(|n| n.checked_mul(job.channels))
            != Some(job.px.len())
    {
        return 1;
    }
    let effect = match find_value(job.json, b"effect") {
        None => &effects::EFFECTS[0],
        Some(v) => match effects::find(parse_str(v)) {
            Some(e) => e,
            None => return UNKNOWN_EFFECT,
        },
    };
    (effect.render)(job)
}

/// Required source context for these settings. Keep the established 64px context
/// for ordinary settings; enlarged effects request only their actual dependency.
pub fn required_overlap(json: &[u8]) -> u32 {
    let effect = find_value(json, b"effect").map_or(b"lightning".as_slice(), parse_str);
    let reach = match effect {
        b"drip" => {
            num(json, "drip_length", 18.0).clamp(2.0, 150.0)
                * num(json, "drip_size", 100.0).clamp(25.0, 1000.0)
                / 100.0
                + 2.0
        }
        b"icicles" => {
            num(json, "icicles_length", 32.0).clamp(2.0, 150.0)
                * num(json, "icicles_size", 100.0).clamp(25.0, 1000.0)
                / 100.0
                + 2.0
        }
        b"neon_tubes" => {
            num(json, "neon_tubes_width", 8.0).clamp(2.0, 500.0) * 0.5
                + num(json, "neon_tubes_glow", 28.0).clamp(0.0, 1250.0)
                + 4.0
        }
        b"holographic_foil" => num(json, "holographic_foil_bevel", 18.0).clamp(0.0, 500.0) + 4.0,
        b"paper_cut" => num(json, "paper_cut_depth", 36.0).clamp(0.0, 500.0) + 8.0,
        b"fire" => num(json, "fire_height", 42.0).clamp(8.0, 1500.0) + 4.0,
        b"smoke" => num(json, "smoke_height", 48.0).clamp(8.0, 1500.0) + 4.0,
        b"backlight" => num(json, "backlight_radius", 34.0).clamp(4.0, 1500.0) + 4.0,
        b"corona" => num(json, "corona_radius", 38.0).clamp(8.0, 1500.0) + 4.0,
        b"glow" => num(json, "glow_radius", 27.0).clamp(2.0, 1500.0) + 4.0,
        b"electrify" => num(json, "electrify_reach", 38.0).clamp(4.0, 1500.0) + 6.0,
        b"shadow" => {
            num(json, "shadow_distance", 15.0).clamp(0.0, 1500.0)
                + libm::ceilf(num(json, "shadow_softness", 5.0).clamp(0.0, 12.0) * 1.5)
                + 2.0
        }
        b"extrude" => num(json, "extrude_length", 18.0).clamp(1.0, 1500.0) + 34.0,
        b"motion_trail" => num(json, "motion_trail_length", 22.0).clamp(2.0, 1500.0) + 34.0,
        b"bevel" => num(json, "bevel_size", 8.0).clamp(1.0, 500.0) * 2.0 + 4.0,
        b"glass" => {
            (num(json, "glass_refraction", 8.0).clamp(0.0, 500.0) * 0.44 + 12.0) * 2.0 + 4.0
        }
        b"snow" => {
            let depth = num(json, "snow_depth", 14.0).clamp(1.0, 500.0);
            if depth <= 24.0 {
                64.0
            } else {
                depth + (depth * 0.32 + 3.0).max(64.0) + 4.0
            }
        }
        b"water_drops" => num(json, "water_drops_scale", 32.0).clamp(14.0, 1500.0) * 0.12 + 2.0,
        _ => 64.0,
    };
    libm::ceilf(reach.max(64.0)) as u32
}

/// Optional ABI-v1 context negotiation; the host supplies resolved parameter JSON.
///
/// # Safety
/// `params` points to `params_len` bytes allocated by the host in module memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pc_overlap(params: *const u8, params_len: u32) -> i32 {
    if params_len > 65536 {
        return -1;
    }
    // SAFETY: the host-owned JSON allocation is valid for the duration of this call.
    let json = unsafe { core::slice::from_raw_parts(params, params_len as usize) };
    required_overlap(json) as i32
}

// ---- parameters -----------------------------------------------------------

/// Finds `"key": value` in the parameter JSON and returns the bytes starting at the value.
pub(crate) fn find_value<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    while i + key.len() + 3 <= json.len() {
        if json[i] == b'"'
            && &json[i + 1..i + 1 + key.len()] == key
            && json[i + 1 + key.len()] == b'"'
            && json[i + 2 + key.len()] == b':'
        {
            let mut j = i + 3 + key.len();
            while j < json.len() && json[j] == b' ' {
                j += 1;
            }
            return Some(&json[j..]);
        }
        i += 1;
    }
    None
}

/// Parses the number at the start of `v` (sign, integer and fraction; no exponent).
pub(crate) fn parse_num(v: &[u8]) -> f32 {
    let mut i = 0;
    let neg = v.first() == Some(&b'-');
    if neg {
        i += 1;
    }
    let mut r = 0.0f32;
    while i < v.len() && v[i].is_ascii_digit() {
        r = r * 10.0 + (v[i] - b'0') as f32;
        i += 1;
    }
    if i < v.len() && v[i] == b'.' {
        i += 1;
        let mut f = 0.1f32;
        while i < v.len() && v[i].is_ascii_digit() {
            r += (v[i] - b'0') as f32 * f;
            f *= 0.1;
            i += 1;
        }
    }
    if neg { -r } else { r }
}

/// The string literal at the start of `v`, without its quotes.
pub(crate) fn parse_str(v: &[u8]) -> &[u8] {
    if v.first() != Some(&b'"') {
        return &[];
    }
    let end = v[1..]
        .iter()
        .position(|&c| c == b'"')
        .map_or(v.len(), |p| p + 1);
    &v[1..end]
}

pub(crate) fn num(json: &[u8], key: &str, default: f32) -> f32 {
    find_value(json, key.as_bytes()).map_or(default, parse_num)
}

/// A `point` param, `"key": [x, y]` in percent of the canvas, as fractions `(x / 100, y / 100)`.
pub(crate) fn point(json: &[u8], key: &str, default: (f32, f32)) -> (f32, f32) {
    let (x, y) = find_value(json, key.as_bytes())
        .and_then(|v| v.strip_prefix(b"["))
        .and_then(|v| {
            let comma = v.iter().position(|&c| c == b',')?;
            let y = &v[comma + 1..];
            let skip = y.iter().position(|&c| c != b' ').unwrap_or(0);
            Some((parse_num(v), parse_num(&y[skip..])))
        })
        .unwrap_or(default);
    (x / 100.0, y / 100.0)
}

/// A `color` param, `"key": "#rrggbb"`, as linear 0..1 channels rounded to three decimals (so
/// `#bfefff` is exactly `[0.749, 0.937, 1.0]`). `default` when the value is missing or malformed.
pub(crate) fn colour(json: &[u8], key: &str, default: [f32; 3]) -> [f32; 3] {
    let Some(v) = find_value(json, key.as_bytes()) else {
        return default;
    };
    let s = parse_str(v);
    if s.len() != 7 || s[0] != b'#' {
        return default;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    };
    let mut out = [0.0f32; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let (Some(hi), Some(lo)) = (digit(s[1 + 2 * i]), digit(s[2 + 2 * i])) else {
            return default;
        };
        let byte = u32::from(hi) * 16 + u32::from(lo);
        // Thousandths, rounded: an exact integer over 1000 is the nearest f32 to that decimal.
        *o = ((byte * 1000 + 127) / 255) as f32 / 1000.0;
    }
    out
}

pub(crate) fn flag(json: &[u8], key: &str, default: bool) -> bool {
    find_value(json, key.as_bytes()).map_or(default, |v| v.starts_with(b"true"))
}

// ---- dispatch ----------------------------------------------------------------

/// Filters this band in place with the effect named by the `effect` parameter (the first effect
/// when it is absent). An `effect` no effect answers to returns error code 4.
///
/// # Safety
/// The host guarantees `buf..buf + len` is a block it got from `pc_alloc`, holding `len / 4` samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pc_filter(
    buf: *mut f32,
    len: u32,
    width: u32,
    height: u32,
    channels: u32,
    format: u32,
    params: *const u8,
    params_len: u32,
) -> i32 {
    if channels == 0 || width == 0 || height == 0 {
        return 1;
    }
    // SAFETY: see the function's contract; the host passes `params_len` bytes of JSON.
    let json = unsafe { core::slice::from_raw_parts(params, params_len as usize) };
    // Old hosts can load this module but cannot safely render the enlarged ranges.
    // Reject missing source context instead of returning an image with clipped seams.
    if num(json, "overlap", 0.0) < required_overlap(json) as f32 {
        return 5;
    }
    // SAFETY: see the function's contract.
    let px = unsafe { core::slice::from_raw_parts_mut(buf, len as usize / 4) };
    let mut job = Job {
        px,
        width: width as usize,
        height: height as usize,
        channels: channels as usize,
        format,
        json,
    };
    filter(&mut job)
}

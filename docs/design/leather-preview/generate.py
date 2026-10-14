#!/usr/bin/env python3
"""Illustrative leather swatches for the proposed FreeCandy7 `leather` effect.

IMPORTANT: this is NOT plug-in output. FreeCandy7 has no `leather` effect yet.
This script re-implements the plug-in's deterministic primitives (hash / noise /
fbm / warped Voronoi, from `plugins/freecandy7/src/effects/common.rs`) in Python
and renders representative mock-ups of each proposed hide and finish so the
looks can be compared side by side.

Pure standard library (no numpy / PIL): PNGs are written with zlib + struct.
Run:  python3 docs/design/leather-preview/generate.py
"""
import math
import os
import struct
import time
import zlib

MASK = 0xFFFFFFFF
OUT = os.path.dirname(os.path.abspath(__file__))

# ---------------------------------------------------------------------------
# Primitives ported verbatim from src/effects/common.rs
# ---------------------------------------------------------------------------


def hashf(n):
    n &= MASK
    n ^= n >> 16
    n = (n * 0x7FEB352D) & MASK
    n ^= n >> 15
    n = (n * 0x846CA68B) & MASK
    n ^= n >> 16
    return (n >> 8) / 16777216.0


def mixf(a, b, t):
    return a + (b - a) * t


def smoothstep(a, b, x):
    if b == a:
        b = a + 1e-5
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3.0 - 2.0 * t)


def _lat(cx, cy, seed):
    return hashf(((cx & MASK) * 0x9E3779B9 & MASK)
                 ^ ((cy & MASK) * 0x85EBCA6B & MASK) ^ (seed & MASK))


def noise2(x, y, seed):
    ix = math.floor(x)
    iy = math.floor(y)
    tx = x - ix
    ty = y - iy
    a = int(ix)
    b = int(iy)
    u = tx * tx * (3.0 - 2.0 * tx)
    v = ty * ty * (3.0 - 2.0 * ty)
    return mixf(mixf(_lat(a, b, seed), _lat(a + 1, b, seed), u),
                mixf(_lat(a, b + 1, seed), _lat(a + 1, b + 1, seed), u), v)


def fbm(x, y, seed):
    return (noise2(x, y, seed) * 0.5
            + noise2(x * 2.03, y * 2.03, seed + 1) * 0.25
            + noise2(x * 4.09, y * 4.09, seed + 2) * 0.125
            + noise2(x * 8.17, y * 8.17, seed + 3) * 0.0625) / 0.9375


def voronoi(x, y, cscale, seed, jitter=0.9):
    """Nearest two jittered cell points: (d1, d2, nearest_cx, nearest_cy)."""
    gx = math.floor(x / cscale)
    gy = math.floor(y / cscale)
    best1 = 1e18
    best2 = 1e18
    bx = by = 0.0
    for oy in (-1, 0, 1):
        cy = int(gy) + oy
        for ox in (-1, 0, 1):
            cx = int(gx) + ox
            r1 = hashf(((cx & MASK) * 0x9E3779B1 & MASK)
                       ^ ((cy & MASK) * 0x85EBCA77 & MASK) ^ (seed & MASK))
            r2 = hashf(((cx & MASK) * 0xC2B2AE3D & MASK)
                       ^ ((cy & MASK) * 0x27D4EB2F & MASK) ^ ((seed + 7) & MASK))
            px = (cx + jitter * r1) * cscale
            py = (cy + jitter * r2) * cscale
            dx = x - px
            dy = y - py
            d = math.sqrt(dx * dx + dy * dy)
            if d < best1:
                best2 = best1
                best1 = d
                bx = px
                by = py
            elif d < best2:
                best2 = d
    return best1, best2, bx, by


# ---------------------------------------------------------------------------
# A tiny 5x7 bitmap font so each PNG is self-labelling
# ---------------------------------------------------------------------------
F = {
    "A": [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
    "B": [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
    "C": [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
    "D": [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
    "E": [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
    "F": [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
    "G": [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
    "H": [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
    "I": [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
    "J": [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
    "K": [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
    "L": [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
    "M": [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
    "N": [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
    "O": [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    "P": [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
    "Q": [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
    "R": [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
    "S": [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
    "T": [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
    "U": [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    "V": [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
    "W": [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
    "X": [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
    "Y": [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
    "Z": [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
    "0": [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
    "1": [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
    "2": [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
    "3": [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
    "4": [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
    "5": [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
    "6": [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
    "7": [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
    "8": [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    "9": [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
    " ": [0, 0, 0, 0, 0, 0, 0],
    "-": [0, 0, 0, 0x1F, 0, 0, 0],
    "_": [0, 0, 0, 0, 0, 0, 0x1F],
    ".": [0, 0, 0, 0, 0, 0x0C, 0x0C],
    "/": [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
    "+": [0, 0x04, 0x04, 0x1F, 0x04, 0x04, 0],
}


def draw_text(buf, w, x, y, text, scale, rgb, spacing=1):
    for ch in text.upper():
        rows = F.get(ch, F[" "])
        for ry, bits in enumerate(rows):
            for rx in range(5):
                if bits & (1 << (4 - rx)):
                    for sy in range(scale):
                        for sx in range(scale):
                            px = x + rx * scale + sx
                            py = y + ry * scale + sy
                            if 0 <= px < w:
                                o = (py * w + px) * 3
                                buf[o] = rgb[0]
                                buf[o + 1] = rgb[1]
                                buf[o + 2] = rgb[2]
        x += (5 + spacing) * scale
    return x


def write_png(path, w, h, rgb):
    raw = bytearray()
    stride = w * 3
    for y in range(h):
        raw.append(0)
        raw += rgb[y * stride:(y + 1) * stride]

    def chunk(typ, data):
        return (struct.pack(">I", len(data)) + typ + data
                + struct.pack(">I", zlib.crc32(typ + data) & MASK))

    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
           + chunk(b"IEND", b""))
    with open(path, "wb") as fh:
        fh.write(png)


# ---------------------------------------------------------------------------
# Leather surface model
# ---------------------------------------------------------------------------
SIZE = 240          # swatch edge, pixels
LABEL = 24          # caption bar height, pixels

DEF = dict(cells=None, jitter=0.9, pebble=0.0, pores=None, dome=None, ridge=None,
           grain=(0.0, 0.1), wrinkle=(0.0, 0.02), emboss=None, nap=None,
           relief=5.0, base=(0.5, 0.4, 0.3), spec=0.3, shin=24.0, kd=1.0,
           amb=0.35, seed=7, ridge_x=0.5, matte=False, file="", label="")


def cfg(**kw):
    c = dict(DEF)
    c.update(kw)
    return c


def height(x, y, v):
    """Procedural surface height at a pixel; the source of the relief shading."""
    h = 0.0
    seed = v["seed"]
    cs = v["cells"]
    if cs:
        d1, d2, bx, by = voronoi(x, y, cs, seed, v["jitter"])
        # Cells are domed, borders are valleys: (d2-d1) is small on a seam.
        # The exponent rounds the tops so cells read as pebbles, not cobbles.
        seam = smoothstep(0.0, 0.62, (d2 - d1) / cs)
        h += v["pebble"] * seam ** 1.5
        if v["pores"]:
            depth, r = v["pores"]
            for ox, oy in ((-0.20, -0.06), (0.17, -0.13), (0.02, 0.21)):
                dx = x - (bx + ox * cs)
                dy = y - (by + oy * cs)
                h -= depth * math.exp(-(dx * dx + dy * dy) / (2.0 * r * r))
    if v["dome"]:
        amp, rad = v["dome"]
        d1, _d2, _bx, _by = voronoi(x, y, cs or 40.0, seed, 0.85)
        dom = smoothstep(rad, 0.0, d1)
        h += amp * dom * dom
    if v["ridge"]:
        amp, sig = v["ridge"]
        rx = v["ridge_x"] * SIZE + 5.0 * math.sin(y * 0.02)
        h += amp * math.exp(-((x - rx) ** 2) / (2.0 * sig * sig))
    ga, gf = v["grain"]
    if ga:
        h += ga * (fbm(x * gf, y * gf, seed + 11) - 0.5)
    wa, wf = v["wrinkle"]
    if wa:
        h += wa * (fbm(x * wf, y * wf, seed + 31) - 0.5)
    if v["emboss"]:
        amp, k = v["emboss"]
        u = math.sin(k * (x + y) * 0.7071)
        w = math.sin(k * (x - y) * 0.7071)
        h += amp * smoothstep(0.55, 0.95, min(abs(u), abs(w)))
    if v["nap"]:
        amp, nf = v["nap"]
        n = fbm(x * nf, y * nf, seed + 91)
        s = noise2(x * nf * 0.7, y * nf * 1.15, seed + 93)   # only mild nap direction
        h += amp * (0.75 * (n - 0.5) + 0.35 * (s - 0.5))
    return h


def _n3(x, y, z):
    inv = 1.0 / math.sqrt(x * x + y * y + z * z)
    return x * inv, y * inv, z * inv


# Natural hides, each at its own characteristic grain (neutral brown/tan range).
HIDES = [
    ("HIDE 1  COWHIDE", "hide_01_cowhide.png",
     cfg(cells=26, pebble=1.00, grain=(0.28, 0.09), wrinkle=(0.35, 0.012),
         base=(0.52, 0.34, 0.19), spec=0.30, shin=22, relief=6.0, seed=7)),
    ("HIDE 2  LAMB NAPPA", "hide_02_lamb_nappa.png",
     cfg(cells=16, jitter=0.8, pebble=0.45, grain=(0.22, 0.16), wrinkle=(0.16, 0.018),
         base=(0.68, 0.55, 0.42), spec=0.42, shin=30, relief=4.0, amb=0.40, seed=13)),
    ("HIDE 3  GOAT", "hide_03_goat_morocco.png",
     cfg(cells=14, pebble=0.50, grain=(0.34, 0.12), wrinkle=(0.10, 0.03),
         base=(0.30, 0.20, 0.13), spec=0.18, shin=16, relief=4.5, seed=21)),
    ("HIDE 4  PIGSKIN", "hide_04_pigskin.png",
     cfg(cells=22, pebble=0.85, pores=(0.75, 2.6), grain=(0.30, 0.10), wrinkle=(0.20, 0.02),
         base=(0.62, 0.52, 0.47), spec=0.26, shin=20, relief=7.0, amb=0.38, seed=29)),
    ("HIDE 5  OSTRICH", "hide_05_ostrich.png",
     cfg(cells=26, dome=(1.30, 14.0), grain=(0.16, 0.14), wrinkle=(0.12, 0.02),
         base=(0.35, 0.27, 0.20), spec=0.24, shin=18, relief=8.0, seed=35)),
    ("HIDE 6  STINGRAY", "hide_06_stingray_shagreen.png",
     cfg(cells=13, pebble=0.75, ridge=(0.90, 13.0), grain=(0.28, 0.12),
         base=(0.23, 0.34, 0.33), spec=0.34, shin=26, relief=7.0, seed=41)),
    ("HIDE 7  CORDOVAN", "hide_07_cordovan.png",
     cfg(cells=11, pebble=0.50, grain=(0.30, 0.16), wrinkle=(0.08, 0.02),
         base=(0.38, 0.13, 0.12), spec=0.75, shin=60, relief=3.5, amb=0.30, seed=47)),
    ("HIDE 8  BUFFALO", "hide_08_buffalo.png",
     cfg(cells=34, pebble=1.15, grain=(0.34, 0.07), wrinkle=(0.55, 0.008),
         base=(0.28, 0.20, 0.14), spec=0.22, shin=16, relief=9.0, amb=0.32, seed=53)),
    ("HIDE 9  DEERSKIN", "hide_09_deerskin.png",
     cfg(cells=15, pebble=0.40, grain=(0.26, 0.20), wrinkle=(0.14, 0.02),
         base=(0.66, 0.53, 0.38), spec=0.30, shin=22, relief=4.0, amb=0.40, seed=59)),
    ("HIDE 10  KANGAROO", "hide_10_kangaroo.png",
     cfg(cells=12, pebble=0.55, grain=(0.32, 0.15), wrinkle=(0.12, 0.025),
         base=(0.44, 0.30, 0.17), spec=0.36, shin=28, relief=5.0, amb=0.38, seed=67)),
]

# Finishes, all applied over one common cowhide-like substrate for comparison.
FINISHES = [
    ("FINISH 1  NAPPA", "finish_01_nappa.png",
     cfg(cells=16, jitter=0.85, pebble=0.50, grain=(0.22, 0.18), wrinkle=(0.12, 0.02),
         base=(0.72, 0.60, 0.47), spec=0.45, shin=34, relief=3.5, amb=0.40, seed=71)),
    ("FINISH 2  PEBBLED", "finish_02_pebbled.png",
     cfg(cells=26, pebble=1.20, grain=(0.28, 0.09), wrinkle=(0.30, 0.012),
         base=(0.40, 0.27, 0.16), spec=0.34, shin=24, relief=7.5, seed=73)),
    ("FINISH 3  SUEDE", "finish_03_suede.png",
     cfg(nap=(0.55, 0.35), grain=(0.10, 0.12), base=(0.55, 0.47, 0.40),
         spec=0.02, shin=6, relief=5.0, amb=0.52, matte=True, seed=79)),
    ("FINISH 4  NUBUCK", "finish_04_nubuck.png",
     cfg(nap=(0.40, 0.50), grain=(0.08, 0.12), base=(0.62, 0.54, 0.45),
         spec=0.03, shin=6, relief=4.0, amb=0.54, matte=True, seed=83)),
    ("FINISH 5  SAFFIANO", "finish_05_saffiano.png",
     cfg(emboss=(1.0, 2.0 * math.pi / 13.0), grain=(0.10, 0.10),
         base=(0.66, 0.51, 0.30), spec=0.38, shin=28, relief=7.0, seed=89)),
    ("FINISH 6  PATENT", "finish_06_patent.png",
     cfg(cells=12, pebble=0.15, grain=(0.05, 0.10), base=(0.07, 0.07, 0.08),
         spec=1.00, shin=45, relief=2.0, amb=0.22, seed=97)),
    ("FINISH 7  PULL-UP", "finish_07_pullup_oiled.png",
     cfg(cells=26, pebble=0.80, grain=(0.30, 0.10), wrinkle=(0.90, 0.006),
         base=(0.58, 0.35, 0.12), spec=0.50, shin=40, relief=6.5, seed=101)),
    ("FINISH 8  WORN", "finish_08_distressed.png",
     cfg(cells=26, pebble=1.00, grain=(0.40, 0.06), wrinkle=(0.60, 0.02),
         base=(0.45, 0.36, 0.26), spec=0.10, shin=10, relief=7.0, amb=0.42, seed=107)),
]


def render(v):
    """Two passes: build the height field, then light it as a bumpy surface."""
    n = SIZE
    H = [0.0] * (n * n)
    for y in range(n):
        row = y * n
        for x in range(n):
            H[row + x] = height(x, y, v)
    hmin = min(H)
    rng = (max(H) - hmin) or 1.0
    L = _n3(-0.55, -0.62, 0.56)          # key light, upper-left
    L2 = _n3(0.55, 0.28, 0.78)           # fill light
    hx, hy, hz = _n3(L[0], L[1], L[2] + 1.0)   # half vector, view = +z
    base, spec, shin = v["base"], v["spec"], v["shin"]
    kd, amb, relief, seed = v["kd"], v["amb"], v["relief"], v["seed"]
    out = bytearray(n * n * 3)
    for y in range(n):
        row = y * n
        ym = row - n if y > 0 else row
        yp = row + n if y < n - 1 else row
        for x in range(n):
            xm = x - 1 if x > 0 else x
            xp = x + 1 if x < n - 1 else x
            h = H[row + x]
            dzdx = (H[row + xp] - H[row + xm]) * relief
            dzdy = (H[yp + x] - H[ym + x]) * relief
            inv = 1.0 / math.sqrt(dzdx * dzdx + dzdy * dzdy + 1.0)
            nx, ny, nz = -dzdx * inv, -dzdy * inv, inv
            d1 = max(0.0, nx * L[0] + ny * L[1] + nz * L[2])
            d2 = max(0.0, nx * L2[0] + ny * L2[1] + nz * L2[2])
            sp = (max(0.0, nx * hx + ny * hy + nz * hz) ** shin) * spec
            ao = 0.55 + 0.45 * ((h - hmin) / rng)
            tint = 1.0 + 0.06 * (fbm(x * 0.03, y * 0.03, seed + 5) - 0.5)
            lit = (amb + kd * d1 + 0.26 * d2) * tint * ao
            o = (row + x) * 3
            out[o] = min(255, int(max(0.0, base[0] * lit + sp) * 255))
            out[o + 1] = min(255, int(max(0.0, base[1] * lit + sp) * 255))
            out[o + 2] = min(255, int(max(0.0, base[2] * lit + sp) * 255))
    return out


def labelled(img, text):
    """Image with a caption bar under it, so each PNG explains itself."""
    w, total = SIZE, SIZE + LABEL
    buf = bytearray(w * total * 3)
    buf[0:w * SIZE * 3] = img
    for y in range(SIZE, total):
        o = y * w * 3
        for x in range(w):
            buf[o + x * 3] = 22
            buf[o + x * 3 + 1] = 22
            buf[o + x * 3 + 2] = 26
    draw_text(buf, w, 8, SIZE + 7, text, 2, (236, 236, 240))
    return buf


def main():
    t0 = time.time()
    tiles = []
    for group in (HIDES, FINISHES):
        for label, name, v in group:
            img = render(v)
            mean = sum(img) / len(img) / 255.0
            sd = (sum((b / 255.0 - mean) ** 2 for b in img) / len(img)) ** 0.5
            buf = labelled(img, label)
            write_png(os.path.join(OUT, name), SIZE, SIZE + LABEL, buf)
            tiles.append(buf)
            print(f"{name:30s} mean={mean:.3f} sd={sd:.3f}  {time.time() - t0:5.1f}s")

    # Contact sheet: 6 columns x 3 rows of the same tiles.
    cols, rows = 6, 3
    tw, th = SIZE, SIZE + LABEL
    sw, sh = cols * tw, rows * th
    sheet = bytearray(sw * sh * 3)
    for i, tile in enumerate(tiles):
        cx, cy = (i % cols) * tw, (i // cols) * th
        for y in range(th):
            so = ((cy + y) * sw + cx) * 3
            sheet[so:so + tw * 3] = tile[y * tw * 3:(y + 1) * tw * 3]
    write_png(os.path.join(OUT, "contact-sheet.png"), sw, sh, sheet)
    print(f"contact-sheet.png {sw}x{sh}  total {time.time() - t0:.1f}s")


if __name__ == "__main__":
    main()

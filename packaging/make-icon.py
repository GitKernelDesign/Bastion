#!/usr/bin/env python3
"""Génère l'icône Bastion : bouclier sobre sur fond arrondi sombre.

Produit :
  assets/icon.png      (512×512, PNG)
  assets/icon64.rgba   (64×64, RGBA brut pour l'icône de fenêtre egui)

Aucune dépendance externe : PNG encodé à la main (zlib + CRC).
"""

import struct
import zlib
import math
import os

SIZE = 512

# Palette (cf. docs/ERGONOMIE.md)
BG_TOP = (0x26, 0x2B, 0x38)
BG_BOTTOM = (0x14, 0x16, 0x1B)
SHIELD_OUTER = (0x4C, 0x8D, 0xFF)   # accent
SHIELD_INNER = (0x1B, 0x21, 0x2C)
KEYHOLE = (0xE8, 0xEA, 0xED)

OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets")


def lerp(a, b, t):
    return (a[i] + (b[i] - a[i]) * t for i in range(3))


def in_rounded_rect(x, y, r):
    """Carré arrondi plein cadre (r = rayon des coins)."""
    dx = abs(x - SIZE / 2)
    dy = abs(y - SIZE / 2)
    h = SIZE / 2
    if dx <= h - r or dy <= h - r:
        return dx <= h and dy <= h
    qx = dx - (h - r)
    qy = dy - (h - r)
    return qx * qx + qy * qy <= r * r


def shield_points(w, head_y, tip_y):
    """Contour du bouclier, en 5/8 supérieur rectangulaire + pointe basse."""
    cx = SIZE / 2
    left = cx - w
    right = cx + w
    shoulder = head_y + 26
    pts = []

    # Côté gauche (du sommet vers la pointe)
    pts.append((left, head_y + 18))
    pts.append((left, shoulder))
    # Flanc courbe vers la pointe centrale
    steps = 24
    for i in range(steps + 1):
        t = i / steps
        yy = shoulder + (tip_y - shoulder) * t
        ease = t * t * (3 - 2 * t)
        xx = left + (cx - left) * ease * 1.04
        pts.append((min(xx, cx), yy))
    pts.append((cx, tip_y))

    # Miroir droit (du bas vers le sommet)
    for (x, y) in reversed(pts[1:-1]):
        pts.append((cx + (cx - x), y))
    pts.append((right, head_y + 18))
    return pts


def point_in_poly(x, y, pts):
    inside = False
    n = len(pts)
    j = n - 1
    for i in range(n):
        xi, yi = pts[i]
        xj, yj = pts[j]
        if ((yi > y) != (yj > y)) and (x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi):
            inside = not inside
        j = i
    return inside


def render():
    cx = SIZE / 2
    w = SIZE * 0.315
    head_y = SIZE * 0.27
    tip_y = SIZE * 0.78
    outer = shield_points(w, head_y, tip_y)
    inner = shield_points(
        w * 0.85, head_y + SIZE * 0.045, tip_y - SIZE * 0.028
    )
    key_r = SIZE * 0.062
    key_cy = SIZE * 0.465
    key_slot_h = SIZE * 0.105
    key_slot_w = SIZE * 0.026
    corner_r = SIZE * 0.19

    buf = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            if not in_rounded_rect(x + 0.5, y + 0.5, corner_r):
                buf += b"\x00\x00\x00\x00"
                continue
            col = lerp(BG_TOP, BG_BOTTOM, y / SIZE)
            col = tuple(int(c) for c in col)
            if point_in_poly(x + 0.5, y + 0.5, outer):
                if point_in_poly(x + 0.5, y + 0.5, inner):
                    col = SHIELD_INNER
                    px, py = x + 0.5, y + 0.5
                    in_circle = (px - cx) ** 2 + (py - key_cy) ** 2 <= key_r ** 2
                    in_stem = (
                        abs(px - cx) <= key_slot_w
                        and key_cy <= py <= key_cy + key_slot_h
                    )
                    if in_circle or in_stem:
                        col = KEYHOLE
                else:
                    col = SHIELD_OUTER
            buf += bytes((col[0], col[1], col[2], 255))
    return bytes(buf)


def write_png(path, size, rgba):
    raw = b""
    stride = size * 4
    for y in range(size):
        raw += b"\x00" + rgba[y * stride : (y + 1) * stride]

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        c += struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        return c

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def downsample(rgba, src, dst):
    out = bytearray()
    ratio = src / dst
    for y in range(dst):
        for x in range(dst):
            r = g = b = a = 0
            n = 0
            for sy in range(int(y * ratio), int((y + 1) * ratio)):
                for sx in range(int(x * ratio), int((x + 1) * ratio)):
                    i = (sy * src + sx) * 4
                    r += rgba[i]
                    g += rgba[i + 1]
                    b += rgba[i + 2]
                    a += rgba[i + 3]
                    n += 1
            out += bytes((r // n, g // n, b // n, a // n))
    return bytes(out)


def main():
    print("Rendu de l'icône %d×%d…" % (SIZE, SIZE))
    rgba = render()
    os.makedirs(OUT_DIR, exist_ok=True)
    png_path = os.path.join(OUT_DIR, "icon.png")
    write_png(png_path, SIZE, rgba)
    print("→ %s" % png_path)

    icon64 = downsample(rgba, SIZE, 64)
    rgba_path = os.path.join(OUT_DIR, "icon64.rgba")
    with open(rgba_path, "wb") as f:
        f.write(icon64)
    print("→ %s" % rgba_path)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate assets/icon.png without external dependencies."""

import math
import struct
import zlib
from pathlib import Path

SIZE = 1024
SS = 2  # supersampling factor per axis


def rounded_rect(x, y, left, top, right, bottom, radius):
    cx = min(max(x, left + radius), right - radius)
    cy = min(max(y, top + radius), bottom - radius)
    dx = x - cx
    dy = y - cy
    return dx * dx + dy * dy <= radius * radius


def segment_distance(px, py, ax, ay, bx, by):
    vx = bx - ax
    vy = by - ay
    wx = px - ax
    wy = py - ay
    t = (wx * vx + wy * vy) / (vx * vx + vy * vy)
    t = max(0.0, min(1.0, t))
    dx = px - (ax + t * vx)
    dy = py - (ay + t * vy)
    return math.hypot(dx, dy)


def sample(x, y):
    # Transparent outside the macOS-style rounded square.
    if not rounded_rect(x, y, 40, 40, SIZE - 40, SIZE - 40, 210):
        return (0, 0, 0, 0)
    # Subtle vertical gradient.
    t = (y - 40) / (SIZE - 80)
    r = int(20 + 6 * (1.0 - t))
    g = int(23 + 7 * (1.0 - t))
    b = int(28 + 10 * (1.0 - t))
    # Inset teal frame.
    if rounded_rect(x, y, 150, 150, SIZE - 150, SIZE - 150, 120):
        r, g, b = 27, 45, 48
    # Stylised N drawn with three strokes.
    stroke = 66.0
    d = 1e9
    d = min(d, segment_distance(x, y, 350, 710, 350, 330))
    d = min(d, segment_distance(x, y, 674, 710, 674, 330))
    d = min(d, segment_distance(x, y, 350, 330, 674, 710))
    if d <= stroke:
        # Antialias the outer edge.
        edge = max(0.0, min(1.0, stroke - d + 0.5))
        teal = (70, 217, 191)
        r = int(r * (1 - edge) + teal[0] * edge)
        g = int(g * (1 - edge) + teal[1] * edge)
        b = int(b * (1 - edge) + teal[2] * edge)
    return (r, g, b, 255)


def main():
    rows = []
    for y in range(SIZE):
        row = bytearray()
        for x in range(SIZE):
            acc = [0, 0, 0, 0]
            for sy in range(SS):
                for sx in range(SS):
                    px = sample(x + (sx + 0.5) / SS, y + (sy + 0.5) / SS)
                    for i in range(4):
                        acc[i] += px[i]
            count = SS * SS
            row.extend(int(round(value / count)) for value in acc)
        rows.append(bytes(row))

    raw = b"".join(b"\x00" + row for row in rows)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )

    out = Path(__file__).resolve().parent.parent / "assets" / "icon.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(png)
    print(f"icon: {out}")


if __name__ == "__main__":
    main()

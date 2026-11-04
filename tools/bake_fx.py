#!/usr/bin/env python3
"""Bake the jar's effect textures into run tables the mod can draw without shipping a PNG.

A `.hnmod` holds only sources, so the rune circle and the lapis circle are stored as rows of runs:
(row, first column, length, alpha 1..15). The result is spliced into scene.rs between the FX-DATA
markers. Usage: bake_fx.py <jar-extract-dir>   (the dir with assets/cinematic/textures/fx/...)
"""
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SCENE = ROOT / 'mixins' / 'cinematic_interactions' / 'scene.rs'


def runs(im, size, levels=15):
    im = im.convert('RGBA').split()[3]
    if im.size != (size, size):
        im = im.resize((size, size), Image.BOX)
    px = im.load()
    out = []
    for y in range(size):
        x = 0
        while x < size:
            a = round(px[x, y] / 255 * levels)
            if a == 0:
                x += 1
                continue
            x1 = x
            while x1 + 1 < size and round(px[x1 + 1, y] / 255 * levels) == a:
                x1 += 1
            out.append((y, x, x1 - x + 1, a))
            x = x1 + 1
    return out


def rust(name, data, size):
    body = ', '.join(f'({y},{x},{n},{a})' for y, x, n, a in data)
    return f'pub const {name}_SIZE: usize = {size};\npub static {name}: [(u8, u8, u8, u8); {len(data)}] = [{body}];\n'


def main():
    base = Path(sys.argv[1]) / 'assets' / 'cinematic' / 'textures' / 'fx'
    magic = runs(Image.open(base / 'magic_circle.png'), 96)
    pixel = runs(Image.open(base / 'enchant_circle_pixel.png'), 64)
    text = '// FX-DATA-BEGIN (made by tools/bake_fx.py from the jar\'s magic_circle.png and enchant_circle_pixel.png)\n'
    text += rust('MAGIC_CIRCLE', magic, 96) + rust('PIXEL_CIRCLE', pixel, 64) + '// FX-DATA-END\n'
    src = SCENE.read_text()
    if '// FX-DATA-BEGIN' in src:
        a = src.index('// FX-DATA-BEGIN')
        b = src.index('// FX-DATA-END\n') + len('// FX-DATA-END\n')
        src = src[:a] + text + src[b:]
    else:
        src = src.rstrip('\n') + '\n\n' + text
    SCENE.write_text(src)
    print(len(magic), len(pixel))


main()

"""Compares a Tracyo screenshot with a Curvyo reference at 1x (ux-notes.md section 12).

Usage: python3 compare.py px-00-bar-closed.png tracyo-00-bar-closed.png [diff.png]

Both images are aligned on the top edge and the left edge of the Trace bar (the first long run of
--toolbar-bg #DCDCE0), cropped to the common size, and compared pixel by pixel with a 1 px
neighbourhood and a tolerance of 24 per channel, so a glyph that is drawn by another rasteriser or
lies half a pixel off is not a difference, but a box, gap, border or colour that is wrong is.
Prints the share of differing pixels and the longest straight run of them, and writes the
differing pixels in red over a faded copy of the reference. Pass (section 12): at most 2 % of the
pixels differ (glyph edges) and no row or column has a run of 12 or more differing pixels (a box,
border, gap or fill that is off). Needs Pillow only.
"""

import sys

from PIL import Image, ImageChops, ImageFilter

BAR = (0xDC, 0xDC, 0xE0)
TOLERANCE = 24
PASS_SHARE = 0.02
PASS_RUN = 12


def bar_origin(image):
    """The (x, y) of the bar's top edge row and left edge column."""
    width, height = image.size
    px = image.load()
    for y in range(height):
        run = 0
        for x in range(width):
            run = run + 1 if px[x, y] == BAR else 0
            if run >= 200:
                top = y
                break
        else:
            continue
        break
    else:
        raise SystemExit("no Trace bar (#DCDCE0 run of 200 px) found")
    probe = top + 18
    for x in range(width):
        if all(px[x, y] == BAR for y in range(probe, probe + 8)):
            return x, top
    raise SystemExit("no left edge of the bar found")


def mismatch(a, b):
    """A mask (mode L, 255 = differs) of pixels of `a` with no match within 1 px in `b`."""
    low = b.filter(ImageFilter.MinFilter(3))
    high = b.filter(ImageFilter.MaxFilter(3))
    under = ImageChops.subtract(low, a)
    over = ImageChops.subtract(a, high)
    worst = ImageChops.lighter(under, over)
    bands = worst.split()
    peak = ImageChops.lighter(ImageChops.lighter(bands[0], bands[1]), bands[2])
    return peak.point(lambda v: 255 if v > TOLERANCE else 0)


def longest_run(mask):
    """The longest horizontal or vertical run of differing pixels."""
    width, height = mask.size
    px = mask.load()
    best = 0
    for outer, inner, at in ((height, width, lambda o, i: px[i, o]), (width, height, lambda o, i: px[o, i])):
        for o in range(outer):
            run = 0
            for i in range(inner):
                run = run + 1 if at(o, i) else 0
                best = max(best, run)
    return best


def main():
    ref = Image.open(sys.argv[1]).convert("RGB")
    got = Image.open(sys.argv[2]).convert("RGB")
    (rx, ry), (gx, gy) = bar_origin(ref), bar_origin(got)
    left, top = min(rx, gx), min(ry, gy)
    width = min(ref.width - rx, got.width - gx) + left
    height = min(ref.height - ry, got.height - gy) + top
    ref = ref.crop((rx - left, ry - top, rx - left + width, ry - top + height))
    got = got.crop((gx - left, gy - top, gx - left + width, gy - top + height))
    differs = ImageChops.lighter(mismatch(ref, got), mismatch(got, ref))
    count = differs.histogram()[255]
    share = count / (width * height)
    run = longest_run(differs)
    passed = share <= PASS_SHARE and run < PASS_RUN
    print(f"{width} x {height} px compared, {count} differ ({share:.2%}), longest run {run} px: {'PASS' if passed else 'FAIL'}")
    if len(sys.argv) > 3:
        faded = Image.blend(ref, Image.new("RGB", ref.size, "white"), 0.6)
        faded.paste(Image.new("RGB", ref.size, (230, 0, 0)), mask=differs)
        faded.save(sys.argv[3])
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()

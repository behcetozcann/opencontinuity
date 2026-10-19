"""Tepsi simgelerini 'satırlı kâğıt' biçiminde üretir (44 px tasarım ızgarası, 16x süper örnekleme)."""
import sys
from PIL import Image, ImageDraw

OUT = sys.argv[1]
SS = 16  # süper örnekleme
GRID = 44
BACK = (3, 3, 25, 28)
FRONT = (16.5, 14.5, 41, 41)
RADIUS = 5.5
STROKE = 3.4
GAP = 3.2
LINE = 2.8
LINES = [(22, 36), (22, 36), (22, 31)]  # (x0, x1) her satır; y aşağıda
LINE_Y = [22.5, 27.5, 32.5]


def box(rect, grow=0.0):
    x0, y0, x1, y1 = rect
    return [(x0 - grow) * SS, (y0 - grow) * SS, (x1 + grow) * SS, (y1 + grow) * SS]


def paper(draw, rect, filled):
    draw.rounded_rectangle(box(rect), radius=RADIUS * SS, fill=255)
    if not filled:
        draw.rounded_rectangle(box(rect, -STROKE), radius=(RADIUS - STROKE / 2) * SS, fill=0)


def bar(draw, x0, y0, x1, y1, value):
    draw.rounded_rectangle([x0 * SS, y0 * SS, x1 * SS, y1 * SS], radius=min(x1 - x0, y1 - y0) / 2 * SS, fill=value)


def mask(look):
    image = Image.new("L", (GRID * SS, GRID * SS), 0)
    draw = ImageDraw.Draw(image)
    filled = look == "connected"
    paper(draw, BACK, filled)
    draw.rounded_rectangle(box(FRONT, GAP), radius=(RADIUS + GAP) * SS, fill=0)
    paper(draw, FRONT, filled)
    if look == "paused":
        for x in (25, 31.5):
            bar(draw, x, 22, x + 2.8, 34, 255)
    else:
        for (x0, x1), y in zip(LINES, LINE_Y):
            bar(draw, x0, y - LINE / 2, x1, y + LINE / 2, 0 if filled else 255)
    return image


COLORS = {"connected": (63, 139, 255), "paused": (244, 159, 39), "waiting": (160, 160, 160)}

for look in ("connected", "waiting", "paused"):
    shape = mask(look)
    for platform, size, color in (("mac", 44, (0, 0, 0)), ("win", 32, COLORS[look])):
        alpha = shape.resize((size, size), Image.LANCZOS)
        icon = Image.new("RGBA", (size, size), color + (0,))
        icon.putalpha(alpha)
        icon.save(f"{OUT}/{platform}-{look}.png")

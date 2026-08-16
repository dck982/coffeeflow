from nurb import *
from math import cos, radians

# Rectangular electronics bay. A front deck carries four magnet towers for the
# wedge; the rear bay stays open for Atom / PSU / relays later.
# Towers use the same 8x3 / puit_diametre wells as the Wago boxes.


@part
def screen_base(
    inner_width=150.0,
    inner_depth=180.0,
    inner_height=45.0,
    wall=2.5,
    floor=2.5,
    tower_height=8.0,
    draft=False,
):
    """Open box under the 60° screen wedge.

    inner_width: clear width inside the box
    inner_depth: clear depth inside the box (front to back)
    inner_height: clear height under the lid line
    wall: side wall thickness
    floor: bottom thickness
    tower_height: how tall the magnet towers stand above the front deck
    """
    puit = measured("puit_diametre")
    aimant_h = measured("aimant_hauteur")
    fond = measured("puit_fond")

    outer_w = inner_width + 2 * wall
    outer_d = inner_depth + 2 * wall
    outer_h = floor + inner_height

    body = Pos(0, 0, outer_h / 2) * Box(outer_w, outer_d, outer_h)
    cavity = Pos(0, 0, floor + inner_height / 2 + 0.5) * Box(
        inner_width, inner_depth, inner_height + 1
    )
    body = body - cavity

    module_w = measured("module_width")
    module_h = measured("module_height")
    frame_w = module_w + 2 * 15.0
    frame_h = module_h + 2 * 10.0
    face_margin = 6.0
    wedge_wall = 2.5
    face_w = frame_w + 2 * face_margin
    face_h = frame_h + 2 * face_margin
    wedge_depth = face_h * cos(radians(60.0))
    wedge_width = face_w + 2 * wedge_wall

    tip_y = -outer_d / 2 + wall + 8.0
    deck = Pos(0, tip_y + wedge_depth / 2, outer_h - wall / 2) * Box(
        wedge_width - 1, wedge_depth + 4, wall
    )
    body = body + deck

    inset = 12.0
    tower_r = puit / 2 + 2.5

    for x in (-(wedge_width / 2 - inset), wedge_width / 2 - inset):
        for y in (tip_y + inset, tip_y + wedge_depth - inset):
            # Overlap the deck so the tower is one solid with the box.
            tower = Pos(x, y, outer_h + tower_height / 2 - 0.5) * Cylinder(
                tower_r, tower_height + 1.0
            )
            # Well opens upward; fond of plastic under the magnet.
            pocket = Pos(
                x, y, outer_h + tower_height - fond - aimant_h / 2
            ) * Cylinder(puit / 2, aimant_h + 0.05)
            body = body + tower - pocket

    if draft:
        return body

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    return polish(body, keep, 1.0)

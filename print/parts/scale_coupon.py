from nurb import *

from system import AMIN, CMIN, scale_frame, scale_layout

CHAMFER = 1.0
PLAY_PER_END = 0.25


@part
def scale_coupon(strip_width=14.0, strip_thickness=2.0, post_length=6.0, draft=False):
    """Fit gauge for the base plate: a strip that drops between the side walls,
    sighting the plate's centre hole and the ring-2 hole at 0 degrees, with a post
    whose top must come flush with the side walls. Turned 45 degrees about Z so
    the 192.5 mm strip fits the bed.

    strip_width: width of the strip along Y
    strip_thickness: thickness of the strip on the plate floor
    post_length: length of the post along X at the strip's left end
    """
    layout = scale_layout()
    axis_y = layout.axis[1]
    x0, x1 = PLAY_PER_END, layout.plate_width - PLAY_PER_END
    wall_height = measured("scale_tole_paroi_hauteur")

    strip = Pos(x0, axis_y - strip_width / 2.0, 0) * Box(x1 - x0, strip_width, strip_thickness, align=AMIN)
    post = Pos(x0, axis_y - strip_width / 2.0, 0) * Box(post_length, strip_width, wall_height, align=AMIN)
    body = strip + post

    for hole_x, hole_y in (layout.holes[0], layout.holes[7]):
        body -= Pos(hole_x, hole_y, -1.0) * Cylinder(layout.hole_diameter / 2.0, strip_thickness + 2.0, align=CMIN)

    if not draft:
        ends = body.edges().filter_by(Axis.Z).filter_by(
            lambda e: abs(e.center().X - x0) < 0.01 or abs(e.center().X - x1) < 0.01
        )
        body = polish(body, ends, CHAMFER)

    body = scale_frame(body)
    centre = Vector(layout.centre_x, -axis_y, 0)
    return Pos(centre) * Rot(0, 0, 45) * Pos(-centre) * body

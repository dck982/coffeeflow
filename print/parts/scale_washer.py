from nurb import *

from system import CMIN, scale_layout

CHAMFER = 1.0


@part
def scale_washer(flange_diameter=14.0, flange_height=1.5, draft=False):
    """Stepped bushing for one of the scale base's three M3 screws: a wide flange
    that spans the plate's Ø8 hole from below and a collar that enters the hole.
    Prints upright on its flange, standalone at the origin.

    flange_diameter: width of the flange the screw head bears on
    flange_height: thickness of the flange, which also sets the screw's reach
    """
    base = scale_layout().base
    flange = Cylinder(flange_diameter / 2.0, flange_height, align=CMIN)
    if not draft:
        top_rim = flange.edges().filter_by(lambda e: e.bounding_box().min.Z > 0.01)
        flange = polish(flange, top_rim, CHAMFER)
    collar = Pos(0, 0, flange_height) * Cylinder(base.washer_collar_diameter / 2.0, base.washer_collar_depth, align=CMIN)
    through = Cylinder(measured("vis_passage") / 2.0, flange_height + base.washer_collar_depth, align=CMIN)
    return flange + collar - through

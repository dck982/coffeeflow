from nurb import *

from system import CMIN, scale_layout


@part
def scale_dowel(entry_chamfer=0.3, draft=False):
    """Dowel pressed into the bottom of the scale tower; its proud end drops
    into the adapter's hex key hole and sets where the tower stands. Prints
    standing, the tower end on the bed and the cone tip up, standalone at the
    origin. Hollow: the pin that pushes out the tower's strong magnet above it
    goes through.

    entry_chamfer: chamfer on the end pushed into the tower, which also takes the bed's squash
    """
    dowel = scale_layout().dowel
    length = dowel.in_tower + dowel.proud
    pin = Cylinder(dowel.diameter / 2.0, length, align=CMIN)
    if not draft:
        pin = polish(pin, pin.edges().filter_by(lambda e: e.bounding_box().min.Z > length - 0.01), dowel.tip)
        pin = polish(pin, pin.edges().filter_by(lambda e: e.bounding_box().max.Z < 0.01), entry_chamfer)
    return pin - Pos(0, 0, -1.0) * Cylinder(dowel.bore / 2.0, length + 2.0, align=CMIN)

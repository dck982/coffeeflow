from nurb import *

from system import CMIN, scale_layout


@part
def scale_dowel(entry_chamfer=0.3, draft=False):
    """Dowel pressed into the bottom of the scale tower; its proud end drops
    into the adapter's hex key hole and sets where the tower stands. Prints
    standing, the tower end on the bed and the cone tip up, standalone at the
    origin.

    entry_chamfer: chamfer on the end pushed into the tower, which also takes the bed's squash
    """
    dowel = scale_layout().dowel
    length = dowel.in_tower + dowel.proud
    pin = Cylinder(dowel.diameter / 2.0, length, align=CMIN)
    if draft:
        return pin
    pin = polish(pin, pin.edges().filter_by(lambda e: e.bounding_box().min.Z > length - 0.01), dowel.tip)
    return polish(pin, pin.edges().filter_by(lambda e: e.bounding_box().max.Z < 0.01), entry_chamfer)

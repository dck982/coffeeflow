from nurb import *
from system import offset_in

# Edge.Cuts of pcb/combined/combined.kicad_pcb, in KiCad millimetres (Y down).
_BOARD_OUTLINE_KICAD = [
    (80.5, 47.0), (80.5, 98.8), (55.9, 98.8), (55.9, 134.6),
    (106.0, 134.6), (106.0, 82.0), (180.7, 82.0), (180.7, 59.0),
    (156.5, 59.0), (156.5, 47.0),
]


def _board_outline():
    """Board outline with the south-west corner at the origin, Y north, CCW."""
    west = min(x for x, _ in _BOARD_OUTLINE_KICAD)
    south = max(y for _, y in _BOARD_OUTLINE_KICAD)
    return [(x - west, south - y) for x, y in _BOARD_OUTLINE_KICAD]


@part
def combined(wall_around=1.68, plate_thickness=1.6):
    """Temporary flat plate: the combined board outline plus the enclosure wall,
    printed to check that the footprint fits in the machine.

    wall_around: how far the plate reaches past the board edge on every side
    plate_thickness: thickness of the plate
    """
    outline = offset_in(_board_outline(), -wall_around)
    return extrude(Polygon(*outline, align=None), amount=plate_thickness)

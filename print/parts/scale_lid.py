from nurb import *

from parts.scale_base import CHAMFER, OVERSHOOT, block
from system import scale_flip, scale_frame, scale_layout


def _outline(layout, margin):
    """The PCB box's outer footprint grown by `margin` all round: (x0, x1, y0, y1)."""
    x = layout.box
    return (
        x.inner_x0 - x.wall - margin, x.inner_x1 + x.wall + margin,
        x.inner_y0 - x.wall - margin, x.inner_y1 + x.wall + margin,
    )


def _wire_slots(layout, gap, slot_height):
    """Slot through each X-end skirt, from the skirt's bottom edge, in line
    with the box's wall notches."""
    x = layout.box
    outer_x0, outer_x1, _, _ = _outline(layout, gap + x.lid_skirt)
    inner_x0, inner_x1, _, _ = _outline(layout, gap)
    half = x.notch_width / 2.0
    z0, z1 = x.lid_skirt_bottom - OVERSHOOT, x.notch_bottom + slot_height
    west = block(outer_x0 - OVERSHOOT, inner_x0 + OVERSHOOT, x.notch_y - half, x.notch_y + half, z0, z1)
    east = block(inner_x1 - OVERSHOOT, outer_x1 + OVERSHOOT, x.notch_y - half, x.notch_y + half, z0, z1)
    return west + east


@part
def scale_lid(fit_gap=None, slot_height=4.0, draft=False):
    """Cap over the scale's PCB box: a plate with a skirt that slips over the
    box walls and holds by a snug fit. Prints upside down on its plate.

    fit_gap: gap between the box walls and the skirt, per side (default: layout lid_play)
    slot_height: how far the wire slots rise above the box's wall notch bottom
    """
    layout = scale_layout()
    x = layout.box
    gap = x.lid_play if fit_gap is None else fit_gap

    x0, x1, y0, y1 = _outline(layout, gap + x.lid_skirt)
    plate_bottom = x.wall_top
    cap = block(x0, x1, y0, y1, x.lid_skirt_bottom, plate_bottom + x.lid_plate)
    if not draft:
        cap = polish(cap, cap.edges().filter_by(Axis.Z), CHAMFER)

    inner_x0, inner_x1, inner_y0, inner_y1 = _outline(layout, gap)
    hollow = block(inner_x0, inner_x1, inner_y0, inner_y1, x.lid_skirt_bottom - OVERSHOOT, plate_bottom)
    body = cap - hollow - _wire_slots(layout, gap, slot_height)
    return scale_flip(scale_frame(body))

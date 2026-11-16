from nurb import *

from system import AMIN, CMIN, scale_flip, scale_frame, scale_layout

APRON_THICKNESS = 2.5
# Air between the tray's front wall and the apron's inner face.
APRON_GAP = 0.5

CUTOUT_CORNER_CHAMFER = 5.0
# The pull hole sits this far in from the plate's right edge and from the
# tray front wall's inner face.
PULL_HOLE_INSET = 10.0

DRAIN_HOLE = 8.0
DRAIN_WEB = 3.5
# Solid plate kept along the rails, the rear edge and the front wall, and around the other openings.
RAIL_BORDER = 6.0
OPENING_BORDER = 4.0


def _block(x0, x1, y0, y1, z0, z1):
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=AMIN)


def _outline(layout):
    """Plate extent in the machine frame: (x0, x1, y0, apron inner face).

    The rear edge stands `play` in front of the tray's sloping rear wall,
    measured where the plate's underside meets the slope's inner face; that
    face rises 45° toward the rear and reaches the rim at the tray's rear end."""
    tray, grille = layout.tray, layout.grille_outer
    slope_y = tray.y_rear + (tray.rim_top - tray.rail_top)
    return (tray.x0 + tray.wall + grille.play, tray.x1 - tray.wall - grille.play,
            slope_y + grille.play, tray.y_front + APRON_GAP)


def _pull_hole_box(layout):
    """(x0, x1, y0, y1) of the square pull hole near the front-right corner."""
    x1 = _outline(layout)[1] - PULL_HOLE_INSET
    y1 = layout.tray.y_front - layout.tray.wall - PULL_HOLE_INSET
    size = layout.grille_outer.pull_hole
    return x1 - size, x1, y1 - size, y1


def _plate_with_apron(layout):
    grille = layout.grille_outer
    x0, x1, y0, apron_y = _outline(layout)
    underside = grille.top - grille.thickness
    plate = _block(x0, x1, y0, apron_y + APRON_THICKNESS, underside, grille.top)
    apron = _block(x0, x1, apron_y, apron_y + APRON_THICKNESS, grille.apron_bottom, grille.top)
    return plate + apron


def _openings(layout):
    """Cut-out for the cup grille, purge opening and pull hole, as cutters."""
    grille = layout.grille_outer
    underside = grille.top - grille.thickness
    cutout = Pos(*layout.axis, underside) * Box(grille.cutout, grille.cutout, grille.thickness, align=CMIN)
    cutout = chamfer(cutout.edges().filter_by(Axis.Z), CUTOUT_CORNER_CHAMFER)
    purge = Pos(layout.centre_x, grille.purge_y, underside) * Cylinder(
        grille.purge_diameter / 2.0, grille.thickness, align=CMIN)
    px0, px1, py0, py1 = _pull_hole_box(layout)
    pull = _block(px0, px1, py0, py1, underside, grille.top)
    return cutout + purge + pull


def _drain_hole_centres(layout):
    """Centres of the whole drain holes: a square grid symmetric about the tray's
    centre line, rows lined up on the tower axis, each hole kept clear of the
    rails, the rear edge, the front wall and the other openings."""
    grille, tray = layout.grille_outer, layout.tray
    x0, x1, y0, _ = _outline(layout)
    half = DRAIN_HOLE / 2.0
    pitch = DRAIN_HOLE + DRAIN_WEB
    x_min, x_max = x0 + RAIL_BORDER + half, x1 - RAIL_BORDER - half
    y_min = y0 + RAIL_BORDER + half
    y_max = tray.y_front - tray.wall - RAIL_BORDER - half
    axis_x, axis_y = layout.axis
    cutout_reach = grille.cutout / 2.0 + OPENING_BORDER + half
    purge_reach = grille.purge_diameter / 2.0 + OPENING_BORDER
    px0, px1, py0, py1 = _pull_hole_box(layout)
    pull_reach = OPENING_BORDER + half

    def clear(x, y):
        if abs(x - axis_x) < cutout_reach and abs(y - axis_y) < cutout_reach:
            return False
        if px0 - pull_reach < x < px1 + pull_reach and py0 - pull_reach < y < py1 + pull_reach:
            return False
        # Nearest point of the hole's square to the purge opening's centre.
        near_x = min(max(layout.centre_x, x - half), x + half)
        near_y = min(max(grille.purge_y, y - half), y + half)
        return (near_x - layout.centre_x) ** 2 + (near_y - grille.purge_y) ** 2 >= purge_reach ** 2

    columns = [layout.centre_x + k * pitch for k in range(-20, 21)
               if x_min <= layout.centre_x + k * pitch <= x_max]
    rows = [axis_y + k * pitch for k in range(-20, 21) if y_min <= axis_y + k * pitch <= y_max]
    return [(x, y) for x in columns for y in rows if clear(x, y)]


def _drain_holes(layout, centres):
    grille = layout.grille_outer
    underside = grille.top - grille.thickness
    hole = Box(DRAIN_HOLE, DRAIN_HOLE, grille.thickness, align=CMIN)
    return Compound([Pos(x, y, underside) * hole for x, y in centres])


def _polish_edges(body, layout):
    """Convex straight edges of the outline and apron, but none in the top face
    (the bed once flipped) nor in the underside that rests on the rails."""
    grille = layout.grille_outer
    x0, x1, y0, apron_y = _outline(layout)
    underside = grille.top - grille.thickness
    concave = concave_edges(body)

    def keep(edge):
        box = edge.bounding_box()
        flat_at = lambda z: abs(box.min.Z - z) < 1e-3 and abs(box.max.Z - z) < 1e-3
        on_outline = (abs(box.min.X - x0) < 1e-3 or abs(box.max.X - x1) < 1e-3
                      or abs(box.min.Y - y0) < 1e-3 or box.min.Y > apron_y - 1e-3)
        return (edge.geom_type == GeomType.LINE and on_outline
                and not flat_at(grille.top) and not flat_at(underside)
                and not any(edge.is_same(other) for other in concave))

    return [edge for edge in body.edges() if keep(edge)]


@part
def scale_grille_outer(edge_chamfer=1.0, draft=False):
    """
    Outer grille: a plate on the tray's rails with a cut-out for the cup grille and a front apron.

    edge_chamfer: size of the chamfer on the outline and apron edges
    """
    layout = scale_layout()
    centres = _drain_hole_centres(layout)
    body = _plate_with_apron(layout) - _openings(layout) - _drain_holes(layout, centres)
    if not draft:
        body = polish(body, _polish_edges(body, layout), edge_chamfer)
    return scale_flip(scale_frame(body))

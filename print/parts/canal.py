from nurb import *
from system import CMIN, AMIN

_EPS = 1e-6
# Overlap into the U wall so the boolean fuses; coplanar contact alone stays separate.
_PAD_OVERLAP = 0.4
_NOTCH_MARGIN = 0.5


def _spans_overlap(left_a, right_a, left_b, right_b):
    return left_a < right_b - _EPS and left_b < right_a - _EPS


def _require_non_negative(value, name):
    if value < 0.0:
        reject(f"{name} cannot be negative", param=name)


def _require_zero_or_one(value, name):
    if value not in (0, 1):
        reject(f"{name} is 0 or 1, not {value}", param=name)


def _add_puit(body, x, y_wall, y_outward, half, depth, retour, outer_z, puit_r, well_h):
    """Fuse a trapezoid magnet pad onto the channel wall and cut the well.

    y_outward is +1 (north, +Y) or -1 (south, -Y). The well shares X and Z
    with a pad on the opposite wall; only Y flips.
    """
    pad_west_x = x - half - retour
    pad_east_x = x + half + retour
    y_inner = y_wall - y_outward * _PAD_OVERLAP
    y_outer = y_wall + y_outward * depth
    inner_west = (pad_west_x, y_inner)
    inner_east = (pad_east_x, y_inner)
    outer_east = (x + half, y_outer)
    outer_west = (x - half, y_outer)
    # CCW from +Z so extrude(amount>0) rises with the U.
    if y_outward >= 0:
        pts = [inner_west, inner_east, outer_east, outer_west]
    else:
        pts = [inner_west, outer_west, outer_east, inner_east]
    pad = extrude(Polygon(*pts, align=None), outer_z)
    y_well = y_wall + y_outward * (depth / 2.0)
    well = Pos(x, y_well, 0) * Cylinder(puit_r, well_h, align=CMIN)
    return body + pad - well


def _cut_south_notch(body, x0, width, wall, outer_z):
    notch = Pos(x0, -_NOTCH_MARGIN, -_NOTCH_MARGIN) * Box(
        width,
        wall + 2.0 * _NOTCH_MARGIN,
        outer_z + 2.0 * _NOTCH_MARGIN,
        align=AMIN,
    )
    return body - notch


def _cut_cable_ties(body, x0, width, wall, largeur_interne, z0, height):
    cutter = Box(width, wall, height, align=AMIN)
    body = body - Pos(x0, 0, z0) * cutter
    return body - Pos(x0, wall + largeur_interne, z0) * cutter


def _span(start, width):
    if width <= 0:
        return None
    return (start, start + width)


def _reject_overlap(span_a, param_a, span_b, param_b, why):
    if span_a is None or span_b is None:
        return
    if _spans_overlap(*span_a, *span_b):
        reject(
            f"{param_a} overlaps {param_b}: {why}",
            param=param_a,
        )


@part
def canal(
    segment_longueur=20.0,
    largeur_interne=7.0,
    hauteur_interne=7.0,
    epaisseur_paroi=1.68,
    retour_biais=4.0,
    ouverture_ouest=0.0,
    ouverture_est=0.0,
    cable_tie_ouest=0.0,
    cable_tie_est=0.0,
    cable_tie_height=1,
    cable_tie_z=1,
    puit_sud=0,
    draft=False,
):
    """U-shaped wire raceway: a channel segment, one magnet well, then the same segment again. Printed open-up; flip onto the chassis so the 0.6 mm magnet face sits on the metal.

    segment_longueur: length of straight channel on each side of the magnet well
    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
    retour_biais: length of the 45-ish taper that blends the pad back into the channel wall
    ouverture_ouest: width of the cable-exit notch cut into the south wall, at the west (low X) end of the channel
    ouverture_est: width of the cable-exit notch cut into the south wall, at the east (high X) end of the channel
    cable_tie_ouest: width of two cable tie openings on the west side of the center pad
    cable_tie_est: width of two cable tie openings on the east side of the center pad
    cable_tie_height: height of cable tie openings
    cable_tie_z: z position of cable tie openings above the floor
    puit_sud: 1 adds a second magnet well on the south (−Y) face, same X and Z as the north well
    """
    wall = epaisseur_paroi

    if largeur_interne < 4.0:
        reject(
            f"largeur_interne {largeur_interne} is under 4 mm: raise it",
            param="largeur_interne",
        )
    if hauteur_interne < 4.0:
        reject(
            f"hauteur_interne {hauteur_interne} is under 4 mm: raise it",
            param="hauteur_interne",
        )
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    _require_non_negative(segment_longueur, "segment_longueur")
    _require_non_negative(retour_biais, "retour_biais")
    _require_non_negative(ouverture_ouest, "ouverture_ouest")
    _require_non_negative(ouverture_est, "ouverture_est")
    _require_non_negative(cable_tie_ouest, "cable_tie_ouest")
    _require_non_negative(cable_tie_est, "cable_tie_est")
    _require_non_negative(cable_tie_height, "cable_tie_height")
    _require_non_negative(cable_tie_z, "cable_tie_z")
    _require_zero_or_one(puit_sud, "puit_sud")

    aimant_h = measured("aimant_hauteur")
    puit_d = measured("aimant_diametre") + measured("aimant_puit_press_fit")
    puit_r = puit_d / 2.0
    puit_fond = measured("aimant_puit_fond")
    marge_puit = measured("aimant_puit_mur")
    half = puit_r + marge_puit
    depth = puit_d + 2.0 * marge_puit
    retour = retour_biais

    pad_clear = half + retour
    outer_x = 2.0 * segment_longueur + 2.0 * pad_clear
    u_y = wall + largeur_interne + wall
    outer_z = wall + hauteur_interne
    x = outer_x / 2.0
    well_h = outer_z - puit_fond

    need_z = puit_fond + aimant_h + 0.4
    if outer_z < need_z:
        reject(
            f"hauteur_interne {hauteur_interne} with epaisseur_paroi {wall} "
            f"leaves {outer_z:.2f} mm total height, under {need_z:.2f} mm "
            f"for the magnet and 0.6 mm face: raise hauteur_interne",
            param="hauteur_interne",
        )

    if ouverture_ouest > outer_x + _EPS:
        reject(
            f"ouverture_ouest {ouverture_ouest} is longer than the channel: lower it",
            param="ouverture_ouest",
        )
    if ouverture_est > outer_x + _EPS:
        reject(
            f"ouverture_est {ouverture_est} is longer than the channel: lower it",
            param="ouverture_est",
        )

    ouverture_ouest_span = _span(0.0, ouverture_ouest)
    ouverture_est_span = _span(outer_x - ouverture_est, ouverture_est)
    cable_tie_ouest_span = _span(segment_longueur - cable_tie_ouest, cable_tie_ouest)
    cable_tie_est_span = _span(outer_x - segment_longueur, cable_tie_est)
    pad_span = (segment_longueur, outer_x - segment_longueur)

    if cable_tie_ouest > segment_longueur + _EPS:
        reject(
            f"cable_tie_ouest {cable_tie_ouest} is longer than the "
            f"{segment_longueur:.2f} mm straight run and would run off the west end: "
            f"lower it",
            param="cable_tie_ouest",
        )
    if cable_tie_est > segment_longueur + _EPS:
        reject(
            f"cable_tie_est {cable_tie_est} is longer than the "
            f"{segment_longueur:.2f} mm straight run and would run off the east end: "
            f"lower it",
            param="cable_tie_est",
        )

    _reject_overlap(
        ouverture_ouest_span,
        "ouverture_ouest",
        cable_tie_ouest_span,
        "cable_tie_ouest",
        "they share the west straight run; lower one of them",
    )
    _reject_overlap(
        ouverture_est_span,
        "ouverture_est",
        cable_tie_est_span,
        "cable_tie_est",
        "they share the east straight run; lower one of them",
    )
    _reject_overlap(
        ouverture_ouest_span,
        "ouverture_ouest",
        cable_tie_est_span,
        "cable_tie_est",
        "they meet on the south wall; lower one of them",
    )
    _reject_overlap(
        ouverture_est_span,
        "ouverture_est",
        cable_tie_ouest_span,
        "cable_tie_ouest",
        "they meet on the south wall; lower one of them",
    )
    if puit_sud:
        pad_why = "the south magnet pad starts there; lower the opening or set puit_sud to 0"
        _reject_overlap(
            ouverture_ouest_span, "ouverture_ouest", pad_span, "puit_sud", pad_why
        )
        _reject_overlap(
            ouverture_est_span, "ouverture_est", pad_span, "puit_sud", pad_why
        )

    if cable_tie_ouest > 0 or cable_tie_est > 0:
        tie_top = wall + cable_tie_z + cable_tie_height
        if tie_top > outer_z + _EPS:
            reject(
                f"cable_tie_z {cable_tie_z} plus cable_tie_height {cable_tie_height} "
                f"reach {tie_top:.2f} mm and the wall is only {outer_z:.2f} mm: "
                f"lower cable_tie_z or cable_tie_height",
                param="cable_tie_z",
            )

    floor = Box(outer_x, u_y, wall, align=AMIN)
    left = Box(outer_x, wall, outer_z, align=AMIN)
    right = Pos(0, wall + largeur_interne, 0) * Box(
        outer_x, wall, outer_z, align=AMIN
    )
    body = floor + left + right

    puit_kw = dict(
        x=x,
        half=half,
        depth=depth,
        retour=retour,
        outer_z=outer_z,
        puit_r=puit_r,
        well_h=well_h,
    )
    body = _add_puit(body, y_wall=u_y, y_outward=1, **puit_kw)
    if puit_sud:
        body = _add_puit(body, y_wall=0.0, y_outward=-1, **puit_kw)

    if ouverture_ouest > 0:
        body = _cut_south_notch(
            body, -_NOTCH_MARGIN, ouverture_ouest + _NOTCH_MARGIN, wall, outer_z
        )
    if ouverture_est > 0:
        body = _cut_south_notch(
            body,
            outer_x - ouverture_est + _NOTCH_MARGIN,
            ouverture_est + _NOTCH_MARGIN,
            wall,
            outer_z,
        )

    tie_z0 = wall + cable_tie_z
    if cable_tie_ouest > 0:
        body = _cut_cable_ties(
            body,
            segment_longueur - cable_tie_ouest,
            cable_tie_ouest,
            wall,
            largeur_interne,
            tie_z0,
            cable_tie_height,
        )
    if cable_tie_est > 0:
        body = _cut_cable_ties(
            body,
            outer_x - segment_longueur,
            cable_tie_est,
            wall,
            largeur_interne,
            tie_z0,
            cable_tie_height,
        )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    bb = body.bounding_box()
    outer_y = bb.max.Y

    def keep(edge):
        ebb = edge.bounding_box()
        if ebb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        on_x = mx < 0.4 or mx > outer_x - 0.4
        on_y = my < 0.4 or my > outer_y - 0.4
        return on_x and on_y

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

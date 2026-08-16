from nurb import *


@part
def canal_test(
    largeur_interne=10.0,
    hauteur_interne=10.0,
    epaisseur_paroi=1.6,
    puit_diametre=8.2,
    marge_puit=2.0,
    retour_biais=5.0,
    marge_extremite=10.0,
    draft=False,
):
    """Fit coupon for one `canal` magnet well: the well's pad plus a plain run of channel on each side. Prints without the wall opposite the well, since that wall carries nothing being tested.

    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
    puit_diametre: well inside diameter (looser than the box 8.15, so the disc reaches the face)
    marge_puit: solid plastic around the well
    retour_biais: length of the 45-ish taper that blends the pad back into the channel wall
    marge_extremite: length of plain channel on each side of the well's pad
    """
    wall = epaisseur_paroi
    puit_fond = measured("puit_fond")
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    puit_r = puit_diametre / 2.0

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
    if puit_diametre < aimant_d + 0.2:
        reject(
            f"puit_diametre {puit_diametre} is too tight for an {aimant_d} mm magnet "
            f"in a deep well: raise it above {aimant_d + 0.2}",
            param="puit_diametre",
        )
    if marge_puit < 1.2:
        reject(
            f"marge_puit {marge_puit} is under 1.2 mm: raise it",
            param="marge_puit",
        )
    if retour_biais < 0.0:
        reject("retour_biais cannot be negative", param="retour_biais")
    if marge_extremite < 0.0:
        reject("marge_extremite cannot be negative", param="marge_extremite")

    half = puit_r + marge_puit
    depth = puit_diametre + 2.0 * marge_puit
    retour = retour_biais
    pad_clear = half + retour

    outer_x = 2.0 * marge_extremite + 2.0 * pad_clear
    u_y = wall + largeur_interne + wall
    outer_z = wall + hauteur_interne

    need_z = puit_fond + aimant_h + 0.4
    if outer_z < need_z:
        reject(
            f"hauteur_interne {hauteur_interne} with epaisseur_paroi {wall} "
            f"leaves {outer_z:.2f} mm total height, under {need_z:.2f} mm "
            f"for the magnet and 0.6 mm face: raise hauteur_interne",
            param="hauteur_interne",
        )

    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    # Floor plus only the wall that carries the well; the opposite wall is
    # not part of what this coupon tests, so it is left out to print faster.
    floor = Box(outer_x, u_y, wall, align=amin)
    near_well_wall = Pos(0, wall + largeur_interne, 0) * Box(
        outer_x, wall, outer_z, align=amin
    )
    body = floor + near_well_wall

    x = outer_x / 2.0
    y_wall = u_y
    y_well = y_wall + depth / 2.0
    well_h = outer_z - puit_fond
    overlap = 0.4

    pts = [
        (x - half - retour, y_wall - overlap),
        (x + half + retour, y_wall - overlap),
        (x + half, y_wall + depth),
        (x - half, y_wall + depth),
    ]
    pad = extrude(Polygon(*pts, align=None), outer_z)
    body = body + pad
    body = body - Pos(x, y_well, 0) * Cylinder(puit_r, well_h, align=cmin)

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

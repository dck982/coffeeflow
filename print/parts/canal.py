from nurb import *


@part
def canal(
    segment_longueur=20.0,
    largeur_interne=7.0,
    hauteur_interne=7.0,
    epaisseur_paroi=1.6,
    puit_diametre=8.2,
    marge_puit=1.6,
    retour_biais=4.0,
    ouverture_ouest=0.0,
    ouverture_est=0.0,
    draft=False,
):
    """U-shaped wire raceway: a channel segment, one magnet well, then the same segment again. Printed open-up; flip onto the chassis so the 0.6 mm magnet face sits on the metal.

    segment_longueur: length of straight channel on each side of the magnet well
    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
    puit_diametre: well inside diameter (looser than the box 8.15, so the disc reaches the face)
    marge_puit: solid plastic around the well
    retour_biais: length of the 45-ish taper that blends the pad back into the channel wall
    ouverture_ouest: width of the cable-exit notch cut into the wall opposite the magnet well, at the west (low X) end of the channel
    ouverture_est: width of the cable-exit notch cut into the wall opposite the magnet well, at the east (high X) end of the channel
    """
    wall = epaisseur_paroi
    puit_fond = measured("puit_fond")
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    puit_d = puit_diametre
    puit_r = puit_d / 2.0

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
    if segment_longueur < 0.0:
        reject("segment_longueur cannot be negative", param="segment_longueur")
    if puit_d < aimant_d + 0.2:
        reject(
            f"puit_diametre {puit_d} is too tight for an {aimant_d} mm magnet "
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

    # Local pad around the well, not a full-length rail.
    half = puit_r + marge_puit
    depth = puit_d + 2.0 * marge_puit
    retour = retour_biais

    # Straight segment on each side, then the pad's angled return, on both ends.
    pad_clear = half + retour
    outer_x = 2.0 * segment_longueur + 2.0 * pad_clear
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

    # U on the bed, open +Z. Ends stay open for the wires.
    floor = Box(outer_x, u_y, wall, align=amin)
    left = Box(outer_x, wall, outer_z, align=amin)
    right = Pos(0, wall + largeur_interne, 0) * Box(
        outer_x, wall, outer_z, align=amin
    )
    body = floor + left + right

    y_wall = u_y
    y_well = y_wall + depth / 2.0
    well_h = outer_z - puit_fond
    # Overlap into the U wall so the boolean fuses; coplanar contact alone stays separate.
    overlap = 0.4

    x = outer_x / 2.0
    # Trapezoid: pad around the well, angled returns back to the channel wall.
    # Winding is CCW from +Z so extrude(amount>0) rises with the U.
    pts = [
        (x - half - retour, y_wall - overlap),
        (x + half + retour, y_wall - overlap),
        (x + half, y_wall + depth),
        (x - half, y_wall + depth),
    ]
    pad = extrude(Polygon(*pts, align=None), outer_z)
    body = body + pad
    body = body - Pos(x, y_well, 0) * Cylinder(puit_r, well_h, align=cmin)

    # Cable-exit notches through the wall opposite the well, at either end.
    margin = 0.5
    if ouverture_ouest > 0:
        if ouverture_ouest > outer_x + 1e-6:
            reject(
                f"ouverture_ouest {ouverture_ouest} is longer than the channel: lower it",
                param="ouverture_ouest",
            )
        notch = Pos(-margin, -margin, -margin) * Box(
            ouverture_ouest + margin, wall + 2.0 * margin, outer_z + 2.0 * margin, align=amin
        )
        body = body - notch
    if ouverture_est > 0:
        if ouverture_est > outer_x + 1e-6:
            reject(
                f"ouverture_est {ouverture_est} is longer than the channel: lower it",
                param="ouverture_est",
            )
        notch = Pos(outer_x - ouverture_est + margin, -margin, -margin) * Box(
            ouverture_est + margin, wall + 2.0 * margin, outer_z + 2.0 * margin, align=amin
        )
        body = body - notch

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

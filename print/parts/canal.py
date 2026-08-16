from nurb import *


@part
def canal(
    longueur=100.0,
    largeur_interne=10.0,
    hauteur_interne=10.0,
    epaisseur_paroi=1.6,
    aimants=2,
    puit_diametre=8.4,
    marge_puit=2.0,
    retour_biais=5.0,
    draft=False,
):
    """U-shaped wire raceway. Printed open-up; flip onto the chassis so the 0.6 mm magnet faces sit on the metal.

    longueur: length of the channel along the wires
    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
    aimants: how many 8x3 mm magnet wells along the side
    puit_diametre: well inside diameter (looser than the box 8.15, so the disc reaches the face)
    marge_puit: solid plastic around each well
    retour_biais: length of the 45-ish taper that blends each pad back into the channel wall
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
            f"epaisseur_paroi {wall} is under 1.2 mm for a 100 mm run: raise it",
            param="epaisseur_paroi",
        )
    if aimants < 1:
        reject("aimants must be at least 1", param="aimants")
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

    # Local pad around each well, not a full-length rail.
    half = puit_r + marge_puit
    depth = puit_d + 2.0 * marge_puit
    pad_len = 2.0 * half
    retour = retour_biais

    outer_x = longueur
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

    # Keep pads clear of the open ends.
    inset = half + retour + 1.0
    if longueur < 2.0 * inset + pad_len:
        reject(
            f"longueur {longueur} is too short for {aimants} magnet(s): "
            f"raise it above {2.0 * inset + pad_len:.0f}",
            param="longueur",
        )

    y_wall = u_y
    y_well = y_wall + depth / 2.0
    well_h = outer_z - puit_fond
    # Overlap into the U wall so the boolean fuses; coplanar contact alone stays separate.
    overlap = 0.4

    if aimants == 1:
        xs = [outer_x / 2.0]
    else:
        span = outer_x - 2.0 * inset
        xs = [inset + span * i / (aimants - 1) for i in range(aimants)]

    for x in xs:
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

from nurb import *
from system import CMIN, AMIN

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
    draft=False,
):
    """U-shaped wire raceway: a channel segment, one magnet well, then the same segment again. Printed open-up; flip onto the chassis so the 0.6 mm magnet face sits on the metal.

    segment_longueur: length of straight channel on each side of the magnet well
    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
    retour_biais: length of the 45-ish taper that blends the pad back into the channel wall
    ouverture_ouest: width of the cable-exit notch cut into the wall opposite the magnet well, at the west (low X) end of the channel
    ouverture_est: width of the cable-exit notch cut into the wall opposite the magnet well, at the east (high X) end of the channel
    cable_tie_ouest: width of two cable tie openings on the west side of the center pad
    cable_tie_est: width of two cable tie openings on the east side of the center pad
    cable_tie_height: height of cable tie openings
    cable_tie_z: z position of cable tie openings
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
    if segment_longueur < 0.0:
        reject("segment_longueur cannot be negative", param="segment_longueur")
    if retour_biais < 0.0:
        reject("retour_biais cannot be negative", param="retour_biais")

    # Local pad around the well, not a full-length rail.
    aimant_h = measured("aimant_hauteur")
    puit_d = (measured("aimant_diametre") + measured("aimant_puit_press_fit"))
    puit_r = puit_d / 2.0
    puit_fond = measured("aimant_puit_fond")
    marge_puit = measured("aimant_puit_mur")
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

    # U on the bed, open +Z. Ends stay open for the wires.
    floor = Box(outer_x, u_y, wall, align=AMIN)
    left = Box(outer_x, wall, outer_z, align=AMIN)
    right = Pos(0, wall + largeur_interne, 0) * Box(
        outer_x, wall, outer_z, align=AMIN
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
    pad_west_x = x - half - retour 
    pad_east_x = x + half + retour
    pts = [
        (pad_west_x, y_wall - overlap),
        (pad_east_x, y_wall - overlap),
        (x + half, y_wall + depth),
        (x - half, y_wall + depth),
    ]
    pad = extrude(Polygon(*pts, align=None), outer_z)
    body = body + pad
    body = body - Pos(x, y_well, 0) * Cylinder(puit_r, well_h, align=CMIN)

    # Cable-exit notches through the wall opposite the well, at either end.
    margin = 0.5
    if ouverture_ouest > 0:
        if ouverture_ouest > outer_x + 1e-6:
            reject(
                f"ouverture_ouest {ouverture_ouest} is longer than the channel: lower it",
                param="ouverture_ouest",
            )
        notch = Pos(-margin, -margin, -margin) * Box(
            ouverture_ouest + margin, wall + 2.0 * margin, outer_z + 2.0 * margin, align=AMIN
        )
        body = body - notch
    if ouverture_est > 0:
        if ouverture_est > outer_x + 1e-6:
            reject(
                f"ouverture_est {ouverture_est} is longer than the channel: lower it",
                param="ouverture_est",
            )
        notch = Pos(outer_x - ouverture_est + margin, -margin, -margin) * Box(
            ouverture_est + margin, wall + 2.0 * margin, outer_z + 2.0 * margin, align=AMIN
        )
        body = body - notch

    if cable_tie_ouest > 0:
        ctie_cutter = Box(cable_tie_ouest, wall, cable_tie_height, align=AMIN)
        ctie_x = pad_west_x-cable_tie_ouest
        body = body - Pos(ctie_x, 0, wall + cable_tie_z) * ctie_cutter
        body = body - Pos(ctie_x, wall + largeur_interne, wall + cable_tie_z) * ctie_cutter

    if cable_tie_est > 0:
        ctie_cutter = Box(cable_tie_est, wall, cable_tie_height, align=AMIN)
        ctie_x = pad_east_x
        body = body - Pos(ctie_x, 0, wall + cable_tie_z) * ctie_cutter
        body = body - Pos(ctie_x, wall + largeur_interne, wall + cable_tie_z) * ctie_cutter


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

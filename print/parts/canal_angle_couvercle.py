from nurb import *


@part
def canal_angle_couvercle(
    cote_boite=40.0,
    portee_angle=20.5,
    epaisseur_paroi=1.6,
    epaisseur_couvercle=3.6,
    marge_puit=2.0,
    jeu_couvercle=0.3,
    draft=False,
):
    """Couvercle du canal_angle: plaque 40×40, un aimant opposé au pilier du boîtier.

    Imprimé face extérieure au lit; jupe N/E/S vers +Z (pas de jupe côté pente).
    Pose: rotation 180° autour de X (l'ouest reste à l'ouest).
    Épaisseur 3.6 mm = aimant 3 mm + 0.6 mm de peau, peau côté boîtier :
    le puits s'ouvre sur la face extérieure (au lit), l'aimant se pose par
    dehors et se colle. Plafond du puits = pont Ø8.15 sur 8.15 mm.

    cote_boite: côté hors-tout, doit matcher le boîtier
    portee_angle: portée du boîtier (slider en phase avec le boîtier)
    epaisseur_paroi: mur du boîtier, place la jupe
    epaisseur_couvercle: épaisseur de la plaque (3.6 = aimant 3 mm + peau 0.6)
    marge_puit: plastique autour du puits (aligné sur le pilier du boîtier)
    jeu_couvercle: jeu jupe / paroi intérieure
    """
    wall = epaisseur_paroi
    puit_fond = measured("puit_fond")
    aimant_h = measured("aimant_hauteur")
    puit_d = measured("puit_diametre")
    puit_d_profond = measured("puit_diametre_canal")
    puit_r = puit_d / 2.0
    puit_r_profond = puit_d_profond / 2.0
    jupe_h = measured("jupe_hauteur")
    jupe_th = measured("jupe_epaisseur")

    if cote_boite < 20.0:
        reject(
            f"cote_boite {cote_boite} is under 20 mm: raise it",
            param="cote_boite",
        )
    if epaisseur_couvercle < puit_fond + 1.0:
        reject(
            f"epaisseur_couvercle {epaisseur_couvercle} is under "
            f"{puit_fond + 1.0} mm: raise it",
            param="epaisseur_couvercle",
        )
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )

    outer = cote_boite
    lid_th = epaisseur_couvercle
    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    body = Box(outer, outer, lid_th, align=amin)

    # Skirt on N / E / S only — no jupe on the west (ramp) side.
    ox = wall + jeu_couvercle
    oy = wall + jeu_couvercle
    sx = outer - 2.0 * ox
    sy = outer - 2.0 * oy
    body = body + Pos(ox + sx - jupe_th, oy, lid_th) * Box(
        jupe_th, sy, jupe_h, align=amin
    )
    body = body + Pos(ox, oy, lid_th) * Box(sx, jupe_th, jupe_h, align=amin)
    body = body + Pos(ox, oy + sy - jupe_th, lid_th) * Box(
        sx, jupe_th, jupe_h, align=amin
    )

    # Same placement as the box pillar, remapped after 180° X seating:
    # west as far as the ramp join allows, Y-centred.
    pillar_r_outer = puit_r_profond + marge_puit
    pillar_x_box = portee_angle + pillar_r_outer + 0.4
    lid_x = pillar_x_box - portee_angle
    lid_y = outer / 2.0
    well_h = lid_th - puit_fond
    if well_h < aimant_h:
        reject(
            f"epaisseur_couvercle {epaisseur_couvercle} leaves a {well_h:.2f} mm "
            f"well under the {puit_fond} mm skin, less than the {aimant_h} mm "
            f"magnet: raise it",
            param="epaisseur_couvercle",
        )

    # The well opens at the BED, which is the outside face, so the puit_fond
    # skin ends up between this magnet and the one in the box. Its ceiling is
    # a Ø8.15 bridge — 8.15mm of unsupported span, well inside what a 0.4 mm
    # nozzle bridges, and it is a closed pocket roof, not a hole rim on air.
    # The magnet goes in from the outside and wants a drop of glue.
    body = body - Pos(lid_x, lid_y, -0.2) * Cylinder(
        puit_r, well_h + 0.2, align=cmin
    )

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        ebb = edge.bounding_box()
        if ebb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        on_x = mx < 0.4 or mx > outer - 0.4
        on_y = my < 0.4 or my > outer - 0.4
        return on_x and on_y

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

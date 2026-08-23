from nurb import *

from system import outer_corners


@part
def support_2x5w(
    longueur_interne=30.0,
    largeur_interne=16.5,
    hauteur=12.0,
    epaisseur_paroi=1.6,
    mur_nord=3.0,
    draft=False,
):
    """Support ouvert pour deux Wago 221-415, aimants dans le mur nord.

    Bac ouvert en +Z : murs est/ouest/sud à epaisseur_paroi, mur nord plus
    épais avec deux puits horizontaux. L'aimant s'insère depuis la cavité ;
    la peau puit_fond (0.6 mm) est sur la face nord extérieure. Hauteur
    10.4 mm (pas 10) : un puit Ø8.15 centré laisse sinon moins de 1 mm
    au-dessus et en dessous.

    longueur_interne: espace libre est-ouest (X), hors murs
    largeur_interne: espace libre nord-sud (Y), hors murs
    hauteur: hauteur hors-tout depuis le lit (10.4 pour les marges des puits)
    epaisseur_paroi: fond et murs E/O/S
    mur_nord: épaisseur du mur nord (Y)
    """
    wall = epaisseur_paroi
    floor = epaisseur_paroi
    puit_d = measured("puit_diametre")
    puit_r = puit_d / 2.0
    puit_fond = measured("puit_fond")
    aimant_d = measured("aimant_diametre")

    if longueur_interne < 10.0:
        reject(
            f"longueur_interne {longueur_interne} is under 10 mm: raise it",
            param="longueur_interne",
        )
    if largeur_interne < 8.0:
        reject(
            f"largeur_interne {largeur_interne} is under 8 mm: raise it",
            param="largeur_interne",
        )
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if mur_nord < puit_fond + 1.0:
        reject(
            f"mur_nord {mur_nord} leaves under 1 mm of well behind the "
            f"{puit_fond} mm outer skin: raise mur_nord",
            param="mur_nord",
        )

    outer_x = longueur_interne + 2.0 * wall
    outer_y = largeur_interne + wall + mur_nord
    well_depth = mur_nord - puit_fond
    # 1.0 mm each side of the round opening (printer min_wall on A1 Mini).
    marge_verticale = 1.0
    if hauteur < puit_d + 2.0 * marge_verticale:
        reject(
            f"hauteur {hauteur} leaves under {marge_verticale} mm above/below a "
            f"Ø{puit_d} well: raise it above {puit_d + 2.0 * marge_verticale}",
            param="hauteur",
        )
    if outer_x < puit_d * 2.0 + 1.0:
        reject(
            f"longueur_interne {longueur_interne} is too short for two "
            f"Ø{puit_d} wells at the ends: raise it",
            param="longueur_interne",
        )
    if puit_d < aimant_d + 0.1:
        reject(
            f"puit_diametre {puit_d} is too tight for an {aimant_d} mm magnet",
            param="mur_nord",
        )

    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    body = Box(outer_x, outer_y, hauteur, align=amin)
    cavity = Pos(wall, wall, floor) * Box(
        longueur_interne, largeur_interne, hauteur, align=amin
    )
    body = body - cavity

    # Horizontal wells open from the cavity (+Y into the north wall). The
    # puit_fond skin stays on the outer north face — same idea as the floor
    # wells in boitier_*, just sideways. Round opening, no overhang.
    # With mur_nord=3 the pocket is 2.4 mm: an 8x3 disc seats against the
    # skin and stands ~0.6 mm proud into the cavity.
    cavity_north = wall + largeur_interne
    marge_bout = 2.0
    well_z = hauteur / 2.0
    well_xs = [puit_r + marge_bout, outer_x - puit_r - marge_bout]
    cutter_h = well_depth + 0.1
    for x in well_xs:
        body = body - (
            Plane(origin=(x, cavity_north - 0.1, well_z), z_dir=(0, 1, 0))
            * Cylinder(puit_r, cutter_h, align=cmin)
        )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, outer_x, outer_y, bed), 1.0)

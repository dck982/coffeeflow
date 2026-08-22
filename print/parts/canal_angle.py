import math

from nurb import *


@part
def canal_angle(
    cote_boite=40.0,
    portee_angle=20.5,
    epaisseur_paroi=1.6,
    trou_diametre=16.4,
    trou_depuis_est=15.0,
    trou_depuis_nord=20.0,
    marge_puit=2.0,
    draft=False,
):
    """Boîtier d'angle : fond plat + rampe 1.6 mm à 45°, un puits aimant pour le couvercle.

    cote_boite: côté hors-tout du carré fermé (sans la rampe)
    portee_angle: course X et montée Z de la rampe à 45° (= hauteur intérieure)
    epaisseur_paroi: épaisseur des murs, du fond plat et de la rampe
    trou_diametre: trou traversant dans le fond (Ø16 tôle + jeu d'impression)
    trou_depuis_est: centre du trou depuis le bord est extérieur
    trou_depuis_nord: centre du trou depuis le bord nord extérieur
    marge_puit: plastique autour du pilier d'aimant couvercle
    """
    wall = epaisseur_paroi
    puit_fond = measured("puit_fond")
    puit_d_profond = measured("puit_diametre_canal")
    puit_r_profond = puit_d_profond / 2.0

    if cote_boite < 20.0:
        reject(
            f"cote_boite {cote_boite} is under 20 mm: raise it",
            param="cote_boite",
        )
    if portee_angle < 8.0:
        reject(
            f"portee_angle {portee_angle} is under 8 mm: raise it",
            param="portee_angle",
        )
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if marge_puit < 1.2:
        reject(
            f"marge_puit {marge_puit} is under 1.2 mm: raise it",
            param="marge_puit",
        )
    if trou_diametre < 2.0:
        reject(
            f"trou_diametre {trou_diametre} is under 2 mm: raise it",
            param="trou_diametre",
        )

    # Origin at the west tip of the ramp. Box occupies x = portee .. outer_x.
    outer_x = portee_angle + cote_boite
    outer_y = cote_boite
    hauteur = wall + portee_angle
    # The ramp is offset PERPENDICULAR to its own 45° plane, so `wall` is the
    # thickness that actually prints. Offsetting it vertically by `wall` (what
    # this used to do) gives a 1.13 mm plate for a 1.6 mm parameter and fires
    # min_wall 0.44 on the 64mm2 west tip face. The perpendicular offset is
    # wall * sqrt(2) in Z, so the underside reaches the bed 0.66 mm west of the
    # floor edge; the ramp foot is clipped on the bed there and fuses into the
    # floor, which keeps `hauteur` (and the lid) exactly where it was.
    montee_rampe = wall * math.sqrt(2.0)
    pied_rampe = hauteur - montee_rampe
    trou_r = trou_diametre / 2.0
    hole_x = outer_x - trou_depuis_est
    hole_y = outer_y - trou_depuis_nord

    if trou_depuis_est - trou_r < 0.4:
        reject(
            f"trou_depuis_est {trou_depuis_est} puts the Ø{trou_diametre} "
            f"hole through the east face: move it west",
            param="trou_depuis_est",
        )
    if hole_x - trou_r < portee_angle + 0.4:
        reject(
            f"trou overlaps the ramp join: raise trou_depuis_est or lower "
            f"trou_diametre",
            param="trou_depuis_est",
        )
    if hole_y - trou_r < wall + 0.4 or hole_y + trou_r > outer_y - wall - 0.4:
        reject(
            f"trou_depuis_nord {trou_depuis_nord} puts the hole into a wall: "
            f"centre it",
            param="trou_depuis_nord",
        )

    # PA M16 locknut SW22, flats east-west: apothem 11 mm must clear the inner wall.
    ecrou_apotheme = measured("ecrou_m16_plats") / 2.0
    east_inner = trou_depuis_est - wall
    if east_inner < ecrou_apotheme + 0.5:
        reject(
            f"east inner run {east_inner:.2f} mm cannot take the "
            f"{2.0 * ecrou_apotheme:.0f} mm nut (need 0.5 mm free): "
            f"raise cote_boite or trou_depuis_est",
            param="trou_depuis_est",
        )

    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    # Flat floor under the square only (on the bed).
    body = Pos(portee_angle, 0, 0) * Box(cote_boite, outer_y, wall, align=amin)

    # Ramp: `wall` thick measured square to the 45° plane, nothing underneath.
    # Top from (portee, wall) to (0, hauteur); underside from (pied, 0) to
    # (0, pied); the foot between pied and portee sits on the bed inside the
    # floor.
    ramp_pts = [
        (portee_angle, 0.0),
        (portee_angle, wall),
        (0.0, hauteur),
        (0.0, pied_rampe),
        (pied_rampe, 0.0),
    ]
    body = body + extrude(Plane.XZ * Polygon(*ramp_pts, align=None), -outer_y)

    # N/S walls follow the ramp: flat rim at hauteur, bottom edge on the 45°
    # underside west of the join (no tall rectangle under the overhang).
    wall_pts = [
        (outer_x, 0.0),
        (outer_x, hauteur),
        (0.0, hauteur),
        (0.0, pied_rampe),
        (pied_rampe, 0.0),
    ]
    wall_profile = Plane.XZ * Polygon(*wall_pts, align=None)
    body = body + extrude(wall_profile, -wall)
    body = body + Pos(0, outer_y - wall, 0) * extrude(wall_profile, -wall)

    # East wall on the box only.
    body = body + Pos(outer_x - wall, 0, 0) * Box(wall, outer_y, hauteur, align=amin)

    body = body - Pos(hole_x, hole_y, -0.5) * Cylinder(
        trou_r, wall + 1.0, align=cmin
    )

    # One lid-magnet pillar: Y-centred, as far west as possible without
    # invading the ramp (outer cylinder flush with the join + a hair).
    pillar_r_outer = puit_r_profond + marge_puit
    pillar_x = portee_angle + pillar_r_outer + 0.4
    pillar_y = outer_y / 2.0
    edge = hole_x - pillar_x - trou_r - puit_r_profond
    if edge < 1.6:
        reject(
            f"lid-magnet pillar clears the hole by only {edge:.2f} mm: "
            f"move the hole east or lower marge_puit",
            param="trou_depuis_est",
        )
    nut_west = hole_x - ecrou_apotheme
    pillar_east = pillar_x + pillar_r_outer
    if nut_west < pillar_east + 0.5:
        reject(
            f"SW22 nut (flat west) clears the lid pillar by only "
            f"{nut_west - pillar_east:.2f} mm: move the hole east",
            param="trou_depuis_est",
        )
    cap_z = hauteur - puit_fond
    body = body + Pos(pillar_x, pillar_y, wall) * Cylinder(
        pillar_r_outer, hauteur - wall, align=cmin
    )
    body = body - Pos(pillar_x, pillar_y, -0.5) * Cylinder(
        puit_r_profond, cap_z + 0.5, align=cmin
    )

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        ebb = edge.bounding_box()
        if ebb.min.Z < bed - 0.05:
            return False
        # West tip is only epaisseur_paroi tall — a 1 mm chamfer would knife it.
        if ebb.max.X < 0.5:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        on_x = mx < 0.4 or mx > outer_x - 0.4
        on_y = my < 0.4 or my > outer_y - 0.4
        return on_x and on_y

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

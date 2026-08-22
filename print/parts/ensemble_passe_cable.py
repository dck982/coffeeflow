from nurb import *


@assembly
def ensemble_passe_cable(
    cote_boite=40.0,
    portee_angle=20.5,
    epaisseur_paroi=1.6,
    trou_diametre=16.4,
    trou_depuis_est=15.0,
    trou_depuis_nord=20.0,
):
    """canal_angle, passe-câble assis dans le trou, écrou serré sur le fond, tôle 1 mm.

    cote_boite: côté hors-tout du carré, passé au boîtier
    portee_angle: rampe du boîtier
    epaisseur_paroi: murs et fond du boîtier
    trou_diametre: trou du fond
    trou_depuis_est: centre du trou depuis l'est
    trou_depuis_nord: centre du trou depuis le nord
    """
    ep_tole = measured("tole_epaisseur")
    ep_bride = measured("passe_cable_bride_epaisseur")
    ep_ecrou = measured("ecrou_m16_epaisseur")
    hole_x = portee_angle + cote_boite - trou_depuis_est
    hole_y = cote_boite - trou_depuis_nord

    box = use(
        "canal_angle",
        cote_boite=float(cote_boite),
        portee_angle=float(portee_angle),
        epaisseur_paroi=float(epaisseur_paroi),
        trou_diametre=float(trou_diametre),
        trou_depuis_est=float(trou_depuis_est),
        trou_depuis_nord=float(trou_depuis_nord),
    )
    # 90° about Z is the thread phase that mates at this seating height, and
    # it is measured, not guessed: at 0° the flanks interpenetrate by 12.5mm3,
    # at 90° by 0.0. It also turns the two cable bores north-south.
    gland = (
        Pos(hole_x, hole_y, -(ep_bride + ep_tole))
        * Rot(0, 0, 90)
        * use("passe_cable")
    )
    # The nut is turned over to use it: its lead-in cone is on the face that
    # prints upward, and the barrel enters from the floor side. No Z rotation
    # here — canal_angle sizes the east run on the SW22 apothem, so the flats
    # stay east-west; the thread phase is taken on the gland instead.
    nut = (
        Pos(hole_x, hole_y, float(epaisseur_paroi) + ep_ecrou)
        * Rot(180, 0, 0)
        * use("ecrou_passe_cable")
    )

    sheet = Pos(hole_x, hole_y, 0) * (
        Box(70.0, 70.0, ep_tole, align=(Align.CENTER, Align.CENTER, Align.MAX))
        - Cylinder(
            8.0,
            ep_tole + 1.0,
            align=(Align.CENTER, Align.CENTER, Align.MAX),
        )
    )
    return box, gland, nut, obstacle(sheet, "tôle 1 mm Ø16")

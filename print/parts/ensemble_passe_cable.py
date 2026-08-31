from nurb import *


@assembly
def ensemble_passe_cable():
    """passe-câble assis dans la tôle Ø16, écrou serré de l'autre côté."""
    ep_tole = measured("tole_epaisseur")
    ep_bride = measured("passe_cable_bride_epaisseur")
    ep_ecrou = measured("ecrou_m16_epaisseur")

    # 90° about Z is the thread phase that mates at this seating height, and
    # it is measured, not guessed: at 0° the flanks interpenetrate by 12.5mm3,
    # at 90° by 0.0. It also turns the two cable bores north-south.
    gland = (
        Pos(0, 0, -(ep_bride + ep_tole))
        * Rot(0, 0, 90)
        * use("passe_cable")
    )
    # The nut is turned over to use it: its lead-in cone is on the face that
    # prints upward, and the barrel enters from the sheet side. No Z rotation
    # here — the thread phase is taken on the gland instead.
    nut = (
        Pos(0, 0, ep_ecrou)
        * Rot(180, 0, 0)
        * use("ecrou_passe_cable")
    )

    sheet = Box(
        70.0, 70.0, ep_tole, align=(Align.CENTER, Align.CENTER, Align.MAX)
    ) - Cylinder(
        8.0,
        ep_tole + 1.0,
        align=(Align.CENTER, Align.CENTER, Align.MAX),
    )
    return gland, nut, obstacle(sheet, "tôle 1 mm Ø16")

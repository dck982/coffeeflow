from nurb import *

from system import add_well, AMIN, CMIN

from parts.boitier_dc import bb_bottom, magnet_spacer_pins


@part
def magnet_spacer_dc(
    hauteur=4.0,
    wall=1.68,
    floor=1.6,
    draft=False,
):
    """Cale d'aimants sous le sud de boitier_dc.

    Repère identique à boitier_dc : pas de translation XY dans l'assemblage.

    hauteur: hauteur du spacer, le jeu sous boitier_dc
    wall: épaisseur des parois, doit correspondre à boitier_dc
    floor: épaisseur du fond de boitier_dc, hauteur des plots
    """

    if hauteur < measured("aimant_hauteur"):
        reject(
            f"hauteur {hauteur} is less than the 3mm magnet height: "
            f"raise it",
            param="hauteur",
        )

    bb = bb_bottom()
    size_y = measured("aimant_diametre") * 2
    pins = magnet_spacer_pins(wall)
    py = pins[0][1]
    y0 = py - size_y / 2.0

    body = Pos(bb.min.X, y0, 0) * Box(bb.size.X, size_y, hauteur, align=AMIN)

    wxd = bb.size.X / 3.5
    for wx in (bb.min.X + wxd, bb.max.X - wxd):
        body = add_well(body, body, wx, py, z0=0, h=hauteur + 1)

    pin_r = measured("magnet_spacer_pin_d") / 2
    for px, pin_y, pz in pins:
        body = body + (
            Pos(px, pin_y, hauteur) * Cylinder(pin_r, floor + pz, align=CMIN)
        )

    return body

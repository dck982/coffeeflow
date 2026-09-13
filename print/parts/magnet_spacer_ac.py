from nurb import *

from system import add_well, AMIN, CMIN

from parts.boitier_ac import magnet_spacer_bb, magnet_spacer_pins


@part
def magnet_spacer_ac(
    hauteur=4.0,
    wall=1.68,
    floor=1.6,
    draft=False,
):
    """Cale d'aimant sous le sud de boitier_ac.

    Repère identique à boitier_ac : pas de translation XY dans l'assemblage.
    Déborde au sud, recouvre le fond, un puits au centre, deux plots.

    hauteur: hauteur du spacer, le jeu sous boitier_ac
    wall: épaisseur des parois, doit correspondre à boitier_ac
    floor: épaisseur du fond de boitier_ac, hauteur des plots
    """

    if hauteur < measured("aimant_hauteur"):
        reject(
            f"hauteur {hauteur} is less than the 3mm magnet height: "
            f"raise it",
            param="hauteur",
        )

    sp = magnet_spacer_bb()
    body = Pos(sp.min.X, sp.min.Y, 0) * Box(
        sp.size.X, sp.size.Y, hauteur, align=AMIN
    )

    cx = 0.5 * (sp.min.X + sp.max.X)
    cy = 0.5 * (sp.min.Y + sp.max.Y)
    body = add_well(body, body, cx, cy, z0=0, h=hauteur + 1)

    pin_r = measured("magnet_spacer_pin_d") / 2
    for px, py, pz in magnet_spacer_pins(wall):
        body = body + (
            Pos(px, py, hauteur) * Cylinder(pin_r, floor + pz, align=CMIN)
        )

    return body

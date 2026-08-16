from nurb import *


@part
def canal_test(
    longueur=10.0,
    largeur_interne=10.0,
    hauteur_interne=10.0,
    epaisseur_paroi=1.6,
    draft=False,
):
    """Bare U coupon: a short offcut of `canal` with no magnet wells, for a fit/clearance test.

    longueur: length of the channel along the wires
    largeur_interne: clear width inside the U
    hauteur_interne: clear height inside the U
    epaisseur_paroi: wall and floor thickness
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

    outer_x = longueur
    u_y = wall + largeur_interne + wall
    outer_z = wall + hauteur_interne

    amin = (Align.MIN, Align.MIN, Align.MIN)

    # U on the bed, open +Z. Ends stay open for the wires.
    floor = Box(outer_x, u_y, wall, align=amin)
    left = Box(outer_x, wall, outer_z, align=amin)
    right = Pos(0, wall + largeur_interne, 0) * Box(
        outer_x, wall, outer_z, align=amin
    )
    body = floor + left + right

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

    return polish(body, body.edges().filter_by(keep), 1.0)

from nurb import *

from system import AMIN, CMIN


def _block(x0,y0,z0,dx,dy,dz):
    return Pos(x0,y0,z0)*Box(dx,dy,dz,align=AMIN)

@part
def boitier_pid(
    wall=1.68, 
    floor=1.6,
    largeur=15.0,
    hauteur=5.0,
    marge=10.0,
    draft=False):
    """Boîtier PID réduit à un simple pont pour tenir le cache pid

    wall: épaisseur des murs
    floor: épaisseur du plancher
    largeur: largeur du pont
    hauteur: hauteur du pont (hauteur du heat insert)
    marge: de combien dépasse le pont en haut et en bas de l'ouverture
    """

    body = _block(0,0,0,largeur,measured("boitier_pid_ouverture_hauteur")+marge,floor)
    bb = body.bounding_box()
    body += _block(0,0,0,bb.max.X,wall,floor+hauteur)
    body += _block(0,bb.max.Y-wall,0,bb.max.X,wall,floor+hauteur)
    # M3 hole in the center
    body -= Pos(bb.center().X,bb.center().Y,0)*Cylinder(1.5,floor,align=CMIN)

    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    keep = body.edges().filter_by(Axis.Z).filter_by(
        lambda e: (e.bounding_box().min.Y < 0.05) or (bb.max.Y-e.bounding_box().max.Y < 0.05)
    )
    return polish(body, keep, 1.0) if keep else body

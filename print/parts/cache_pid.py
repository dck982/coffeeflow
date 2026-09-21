from nurb import *

from system import AMIN

def _block(x0,y0,z0,dx,dy,dz):
    return Pos(x0,y0,z0)*Box(dx,dy,dz,align=AMIN)

@part
def cache_pid(wall=1.68, draft=False):
    """
    Cache pour l'ouverture PID de la face avant

    wall: épaisseur des murs
    floor: épaisseur du plancher
    """
    
    body = _block(0,0,0,measured("cache_pid_largeur"),measured("cache_pid_longueur"),measured("cache_pid_epaisseur"))
    
    goujon_z = measured("cache_pid_goujon_z")
    aimant_h = measured("aimant_petit_hauteur")+measured("aimant_puit_press_fit")
    aimant_skin = measured("aimant_puit_fond")

    # Two holes that can be connected to the holes in boitier_pid with a "goujon_pid"
    opening_dx = measured("boitier_pid_ouverture_largeur")
    opening_dy = measured("boitier_pid_ouverture_hauteur")
    goujon_w = measured("boitier_pid_goujon_w")
    goujon_x0 = measured("boitier_pid_goujon_x")
    goujon_y = body.bounding_box().center().Y - goujon_w/2
    goujon_coords = (
        (body.bounding_box().center().X-opening_dx/2+goujon_x0,goujon_y),
        (body.bounding_box().center().X+opening_dx/2-goujon_x0-goujon_w,goujon_y)
    )
    for (gx, gy) in goujon_coords:
        body -= _block(gx,gy,0,goujon_w,goujon_w,goujon_z)

    # Two slots for the magnets
    aimant_w = measured("aimant_petit_diametre")+measured("aimant_puit_press_fit")
    aimant_cy = body.bounding_box().center().Y - aimant_w/2
    aimant_cx = (
        body.bounding_box().center().X-opening_dx/2-aimant_w-0.5,
        body.bounding_box().center().X+opening_dx/2+aimant_w+0.5,
    )
    body -= _block(
        aimant_cx[0],aimant_cy,aimant_skin,
        goujon_coords[0][0]-aimant_cx[0],aimant_w,goujon_z-aimant_skin)
    body -= _block(
        goujon_coords[1][0]+goujon_w,aimant_cy,aimant_skin,
        aimant_cx[1]-(goujon_coords[1][0]+goujon_w),aimant_w,goujon_z-aimant_skin)
    
    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e:  body.bounding_box().max.Z - e.bounding_box().min.Z < 0.05)
    return polish(body, keep, 1.0)

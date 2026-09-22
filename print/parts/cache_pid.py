from nurb import *

from system import AMIN

def _block(x0,y0,z0,dx,dy,dz):
    return Pos(x0,y0,z0)*Box(dx,dy,dz,align=AMIN)

@part
def cache_pid(wall=1.68, plaque=0, draft=False):
    """
    Cache pour l'ouverture PID de la face avant

    wall: épaisseur des murs
    plaque: quand 1, mode plaque
    """
    
    cache_dims = (measured("cache_pid_largeur"),measured("cache_pid_longueur"),measured("cache_pid_epaisseur"))
    plaque_dims = (measured("boitier_pid_ouverture_largeur"),measured("boitier_pid_ouverture_hauteur"),measured("plaque_pid_epaisseur"))
    body = _block(0,0,0,*cache_dims) if plaque==0 else _block(0,0,0,*plaque_dims) 
    
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
        (body.bounding_box().center().X+goujon_x0-goujon_w,goujon_y),
        (body.bounding_box().center().X-goujon_x0,goujon_y)
    )
    for (gx, gy) in goujon_coords:
        body -= _block(gx,gy,0,goujon_w,goujon_w,goujon_z)

    if plaque==0:
        # Two slots for the magnets
        aimant_w = measured("aimant_petit_diametre")+measured("aimant_puit_press_fit")
        aimant_cy = body.bounding_box().center().Y - aimant_w/2
        aimant_cx = (
            body.bounding_box().center().X-opening_dx/2-aimant_w-0.5,
            body.bounding_box().center().X+opening_dx/2+aimant_w+0.5,
        )
        body -= _block(
            aimant_cx[0],aimant_cy,aimant_skin,
            goujon_coords[1][0]-aimant_cx[0],aimant_w,goujon_z-aimant_skin)
        body -= _block(
            goujon_coords[0][0]+goujon_w,aimant_cy,aimant_skin,
            aimant_cx[1]-(goujon_coords[0][0]+goujon_w),aimant_w,goujon_z-aimant_skin)
    
    if draft or plaque>0:
        return body

    def keep(e):
        bb = body.bounding_box()
        if bb.max.Z - e.bounding_box().min.Z > 0.05:
            return False
        if abs(bb.max.X-e.bounding_box().max.X) < 0.05:
            return True
        if abs(bb.min.X-e.bounding_box().min.X) < 0.05:
            return True
        return False
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    keep = body.edges().filter_by(keep)
    return polish(body, keep, 1.0)

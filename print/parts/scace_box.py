from nurb import *
from system import AMIN

def _block(x0,y0,z0,dx,dy,dz):
    return Pos(x0,y0,z0)*Box(dx,dy,dz,align=AMIN)

@part
def scace_box(height=25.0, wall=1.68, floor=1.6, draft=False):
    w1 = measured("wago_415_largeur") # wago 5 and ADS
    w2 = 17.0 # R1K and LDO
    w3 = measured("wago_423_largeur") # wago 3

    lw = measured("wago_epaisseur")
    lb = 2*lw  # 2 wago bottom
    lt1 = 10 # ADS
    lt2 = 9 # LDO

    width = w1+w2+w3+4*wall
    length = lb+lt1+3*wall

    body = Pos(0,0,0) * Box(width,length,floor,align=AMIN)

    # outer walls
    body += _block(0,0,0,width,wall,height)
    body += _block(0,0,0,wall,length,height)
    body += _block(0,length-wall,0,width,wall,height)
    body += _block(width-wall,0,0,wall,length,height)

    # inner walls
    body += _block(wall+w1,0,0,wall,length,height)
    body += _block(wall+w1+wall+w2,0,0,wall,length,height)
    body += _block(wall,wall+lb,0,w1,wall,height)
    body += _block(wall+w1+wall,length-2*wall-lt2,0,w2,wall,height)
    body += _block(wall+w1+wall+w2+wall,wall+lw,0,w3,wall,height)

    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    bed = body.bounding_box().min.Z
    bb = body.bounding_box()
    keep = body.edges().filter_by(Axis.Z).filter_by(lambda e: abs(e.bounding_box().max.X-bb.max.X) < 0.05 or abs(e.bounding_box().min.X-bb.min.X) < 0.05)
    return polish(body, keep, 1.0)

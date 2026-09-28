from nurb import *
import math

@part
def sim_puck(width=40.0, depth=30.0, height=20.0, wall=2.0, draft=False):
    pf_diameter = 57.0
    puck_dz = 13.5
    body = Cylinder(pf_diameter/2.0, puck_dz)
    bb = body.bounding_box()
    drain_dz = 1.0

    half_r = pf_diameter/3.0
    hole_r = 1.0
    for a in (0,60,120,180,240,300):    
        rad = math.radians(a)
        cx = math.cos(rad)*half_r
        cy = math.sin(rad)*half_r
        body -= Pos(cx,cy,0) * Cylinder(hole_r, puck_dz*2)
        body -= Pos(0,0,bb.min.Z) * Rot(0,0,a) * Box(half_r,hole_r*2,drain_dz,align=(Align.MIN,Align.CENTER,Align.MIN))

    probe_cx = 0
    probe_cy = pf_diameter/4
    probe_r = 1.0
    body -= Pos(probe_cx,probe_cy-puck_dz/2-probe_r,0) * Rot(45,0,0) * Cylinder(probe_r,puck_dz*2, align=(Align.CENTER,Align.MIN,Align.CENTER))

    joint_outer_d = 9.1
    joint_inner_d = 6.45
    joint_dz = 1.0
    cmin = (Align.CENTER,Align.CENTER,Align.MIN)
    joint_cutter = Cylinder(joint_outer_d/2,joint_dz,align=cmin)-Cylinder(joint_inner_d/2,joint_dz,align=cmin)
    body -= Pos(probe_cx,probe_cy,bb.min.Z)*joint_cutter
    
    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed+1)
    return polish(body, keep, 1.0)

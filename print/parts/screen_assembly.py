from nurb import *
from math import sin, cos, radians

# Wedge only for now — screen_base is out of scope. Print pose is large rear
# plate on the bed; this stands it at tilt_degrees.


@assembly
def screen_assembly(tilt_degrees=60.0, draft=False):
    """Waveshare carrier at desk tilt (base out of scope).

    tilt_degrees: angle between the screen face and the desk
    """
    wedge = use("screen_wedge")

    module_w = measured("module_width")
    module_h = measured("module_height")
    module_t = measured("module_thickness")
    border_tb = 10.0
    rear_wall = 2.5
    cavity_depth = 10.0
    frame_th = 1.6
    face_h = module_h + 2 * border_tb

    wedge_stood = _stand_wedge(wedge, tilt_degrees, face_h)

    tilt = radians(tilt_degrees)
    outward = Vector(0, -sin(tilt), cos(tilt))
    along = Vector(0, cos(tilt), sin(tilt))
    # Print +Z toward the glass frame. Face centre at the small frame.
    face_centre = (
        along * (face_h / 2)
        + outward * (rear_wall + cavity_depth + frame_th / 2)
    )

    module = Plane(
        origin=face_centre + (-outward) * (frame_th / 2 + module_t / 2),
        x_dir=(1, 0, 0),
        z_dir=outward,
    ) * Box(module_w, module_h, module_t)
    obstacle(module, name="Waveshare 4.3 module (approx)")

    return wedge_stood


def _stand_wedge(wedge, tilt_degrees, face_h):
    """Rear-on-bed print → desk pose: glass at tilt, bottom edge on Y=0 / Z=0."""
    # Print +Y up the face, print +Z toward the glass — no Y180.
    w = Pos(0, face_h / 2, 0) * wedge
    w = Rot(tilt_degrees, 0, 0) * w
    bb = w.bounding_box()
    return Pos(0, -bb.min.Y, -bb.min.Z) * w

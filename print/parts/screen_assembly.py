from nurb import *
from math import sin, cos, radians

# screen_wedge lying in screen_base's cradle. Both print flat and separately;
# this is where the fit between them is checked. The wedge's rear plate lands on
# the base's seat slope, its bottom face against the lip, its borders between
# the two rails, and the base's two foot pins enter the wedge's sockets.
#
# `slide` walks the wedge back up the slope so the pins and the rebate are
# visible in the viewer. Everything is seated at 0.


@assembly
def screen_assembly(slide=0.0, tilt=60.0, seat_height=20.0, draft=False):
    """Waveshare 4.3 carrier seated in its cradle.

    slide: how far up the slope the wedge is pulled, to see the joint. 0 is home
    tilt: screen angle, and it has to be the same in both parts
    seat_height: base height where the wedge's foot lands, shared the same way
    """
    # Unwrap: the runtime hands an assembly its floats wrapped so the viewer can
    # tie a slider to a hinge, and that wrapper does not survive `use()`.
    tilt = float(tilt)
    seat_height = float(seat_height)
    slide = float(slide)

    wedge = use("screen_wedge")
    base = use("screen_base", tilt=tilt, seat_height=seat_height)

    module_w = measured("module_width")
    module_h = measured("module_height")
    module_t = measured("module_thickness")

    t = radians(tilt)
    s, c = sin(t), cos(t)

    # The base's own numbers, from the same expressions screen_base uses.
    wedge_thickness = 19.60
    wedge_length = module_h + 2 * 5.0          # border_top_bottom, both ends
    rim_z = 19.60
    seat_y = wedge_thickness * s
    notch = Vector(0, seat_y, seat_height)
    n = Vector(0, -s, c)                       # out of the seat
    seat = Plane(origin=notch, x_dir=(1, 0, 0), z_dir=n)   # its y runs up the slope

    placed = seat * Pos(0, wedge_length / 2 + slide, 0) * wedge

    # The module drops into the pocket, glass flush with the rim.
    module = (
        seat
        * Pos(0, wedge_length / 2 + slide, 0)
        * Pos(0, 0, rim_z - module_t / 2)
        * Box(module_w, module_h, module_t)
    )
    obstacle(module, name="Waveshare 4.3 module")

    return base + placed

from nurb import *
from math import sin, cos, radians

from system import back_face_layout

# screen_wedge lying in screen_base's cradle. Both print flat and separately;
# this is where the fit between them is checked. The wedge's rear plate lands on
# the base's seat slope, its bottom face against the lip, its borders between
# the two rails, and the base's two foot pins enter the wedge's sockets.
#
# `slide` walks the wedge back up the slope so the pins and the rebate are
# visible in the viewer. Everything is seated at 0.


@assembly
def screen_assembly(
    slide=0.0,
    tilt=45.0,
    seat_height=5.0,
    show_machine_face=True,
    machine_face_thickness=2.0,
    machine_face_overhang=10.0,
    draft=False,
):
    """Waveshare 4.3 carrier seated in its cradle.

    slide: how far up the slope the wedge is pulled, to see the joint. 0 is home
    tilt: screen angle, and it has to be the same in both parts
    seat_height: base height where the wedge's foot lands, shared the same way
    show_machine_face: show the espresso machine's own rear panel, plaqued
        against screen_base's back face — an obstacle, not a printed part, so
        it can be hidden once it has made its point
    machine_face_thickness: thickness of that panel
    machine_face_overhang: how far the panel runs past screen_base's back
        face in -Z and +X, to read as its own surface rather than a patch cut
        to size
    """
    # Unwrap: the runtime hands an assembly its floats wrapped so the viewer can
    # tie a slider to a hinge, and that wrapper does not survive `use()`.
    tilt = float(tilt)
    seat_height = float(seat_height)
    slide = float(slide)
    machine_face_thickness = float(machine_face_thickness)
    machine_face_overhang = float(machine_face_overhang)

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

    # The espresso machine's own rear panel, plaqued against screen_base's
    # back face. Same size as that face, overhanging machine_face_overhang
    # past it in -Z and +X only, so it reads as a surface the box is mounted
    # to rather than a patch cut exactly to size. Two round openings, centred
    # on screen_base's own two back openings, sized to each opening's own
    # width — a rough stand-in, not a measured cutout.
    if show_machine_face:
        layout = back_face_layout(tilt=tilt, seat_height=seat_height)
        plate_width = layout["width"] + machine_face_overhang
        plate_height = layout["north_height"] + machine_face_overhang
        plate_x = machine_face_overhang / 2
        plate_z = (layout["north_height"] - machine_face_overhang) / 2
        plate_y = layout["north_y"] + machine_face_thickness / 2
        plate = Pos(plate_x, plate_y, plate_z) * Box(
            plate_width, machine_face_thickness, plate_height
        )
        for ox, oz, dia in (
            (layout["back_opening_x"], layout["back_opening_z"], 16.0),
            (layout["second_opening_x"], layout["second_opening_z"], 6.0),
        ):
            hole_plane = Plane(origin=(ox, layout["north_y"], oz), x_dir=(1, 0, 0), z_dir=(0, 1, 0))
            plate -= hole_plane * Cylinder(
                dia / 2, machine_face_thickness, align=(Align.CENTER, Align.CENTER, Align.MIN)
            )
        obstacle(plate, name="espresso machine rear panel")
        return base + placed, plate

    return base + placed

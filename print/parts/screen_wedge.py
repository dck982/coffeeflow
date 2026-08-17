from nurb import *

# Waveshare 4.3 carrier. Print pose: large rear plate on the bed, solid border
# skirt rising, module pocket cut through it. The skirt top *is* the frame: it
# lands level with the glass, so there is no plate bridging the pocket. The
# module's own four M2.5 standoffs land on the seat pads and the screws come up
# from under the bed face into them. Outer face stays closed; USB-C / UART drop
# through skirt channels and out the rear plate.


@part
def screen_wedge(
    border_side=10.0,
    border_top_bottom=5.0,
    rear_wall=2.5,
    pocket_depth=12.8,
    module_clearance=0.5,
    cutout_inset=12.0,
    seat_height=4.3,
    cable_slot_width=9.6,
    cable_slot_depth=7.0,
    outer_wall=2.0,
    foot_hole_width=5.5,
    foot_hole_depth=3.5,
    foot_hole_span=60.0,
    draft=False,
):
    """Open-back Waveshare 4.3 carrier: solid skirt, frame rim level with the glass.

    border_side: how far the frame sticks past the glass left and right. This is
        the only border that has a job: it houses the two USB-C channels
    border_top_bottom: how far the frame sticks past the glass top and bottom.
        The module already carries its own black bezel on the glass, so this
        adds nothing visually and only has to be a sound wall
    rear_wall: thickness of the large rear plate (on the bed)
    pocket_depth: from the seat pad tops up to the frame rim, so the rim comes
        out level with the glass: 8.80 glass to PCB plus the module's own 4.00
        M2.5 standoffs, which are what the pads carry
    module_clearance: free fit around the module inside the pocket
    cutout_inset: how far the rear opening stops short of the screw pads
    seat_height: how far the four seat pads stand off the rear plate. Under
        3.2 the counterbore's bridging steps leave too little pad above them.
        It also sets the screw: an M2.5x8 crosses rear_wall + seat_height minus
        the 2.7 head pocket, and whatever is left of the 8 goes into the
        module's 4mm standoff, so raising this shortens the thread engagement
    cable_slot_width: width of the USB-C / UART channels, set by the bare
        right-angle plug's 8.34mm shell plus clearance
    cable_slot_depth: how far each channel runs from the pocket into the border,
        i.e. how far the plug stands out past the socket face
    outer_wall: material left between the end of a channel and the outside face
    foot_hole_width: the two sockets in the bottom face that screen_base's foot
        pins plug into. Free fit on a 5mm pin, because two pins 60mm apart on a
        separately printed part will never line up to a snug fit
    foot_hole_depth: how deep those sockets go into the bottom border, which
        has 4.5mm of solid before the pocket
    foot_hole_span: centre-to-centre of the two sockets, matched to
        screen_base's rib_span
    """
    module_w = measured("module_width")
    module_h = measured("module_height")
    mount_x = measured("mount_spacing_x")
    mount_y = measured("mount_spacing_y")
    hole = measured("screw_clearance")
    head_dia = measured("screw_head_dia")
    head_h = measured("screw_head_height")
    usb_from_top = measured("usb_from_top_back")
    uart_from_top = measured("uart_from_top_back")

    outer_w = module_w + 2 * border_side
    outer_h = module_h + 2 * border_top_bottom
    pocket_w = module_w + 2 * module_clearance
    pocket_h = module_h + 2 * module_clearance

    usb_y = module_h / 2 - usb_from_top
    uart_y = module_h / 2 - uart_from_top

    cut_w = mount_x - 2 * cutout_inset
    cut_h = mount_y - 2 * cutout_inset
    if cut_w < 20 or cut_h < 20:
        reject(
            f"cutout_inset {cutout_inset} leaves a rear opening under 20mm; lower it",
            param="cutout_inset",
        )
    left_over = border_side - module_clearance - cable_slot_depth - 0.1
    if left_over < outer_wall:
        reject(
            f"cable_slot_depth {cable_slot_depth} leaves only {left_over:.1f}mm of "
            f"outer wall, under outer_wall {outer_wall}; shorten it or widen "
            f"border_side past {module_clearance + cable_slot_depth + 0.1 + outer_wall:.1f}",
            param="cable_slot_depth",
        )
    slot_x = pocket_w / 2 + cable_slot_depth / 2

    # --- volumes first: rear plate on the bed, then the skirt up to the rim ---
    rim_z = rear_wall + seat_height + pocket_depth
    body = Pos(0, 0, rear_wall / 2) * Box(outer_w, outer_h, rear_wall)
    body = body + Pos(0, 0, (rear_wall + rim_z) / 2) * Box(
        outer_w, outer_h, rim_z - rear_wall
    )

    # --- openings, before the pads, or the pocket cut would eat them ---
    body = body - Pos(0, 0, (rear_wall + rim_z + 1) / 2) * Box(
        pocket_w, pocket_h, rim_z + 1 - rear_wall
    )
    body = body - Pos(0, 0, rear_wall / 2) * Box(cut_w, cut_h, rear_wall + 2)

    seat_z = rear_wall + seat_height

    # --- four seat pads standing in the pocket; the module lands on these ---
    # Rectangular and flush into the pocket corner: a round pad leaves a wedge
    # of void between its arc and the corner, too thin for the printer to lay.
    # Sized to the doctrine's "a loaded hole earns a fastener diameter of wall",
    # so 2.5mm of PETG around the M2.5 bore and not a millimetre more: the top
    # pads share the border with the UART plug and every extra millimetre here
    # is one the plug does not get.
    pad_wall = 2.5
    pad_x = (pocket_w / 2 - mount_x / 2) + hole / 2 + pad_wall
    pad_y = (pocket_h / 2 - mount_y / 2) + hole / 2 + pad_wall

    # The UART socket sits 12.93mm from the top edge while the screw sits 4.00mm
    # from it, so the channel and the top pads are fighting over the same 8.9mm.
    # This is what put a pad across the opening; the guard is what keeps it out.
    pad_edge = pocket_h / 2 - pad_y
    slot_edge = max(usb_y, uart_y) + cable_slot_width / 2
    if slot_edge > pad_edge:
        reject(
            f"the upper channel reaches y {slot_edge:.2f} but the screw pad starts "
            f"at y {pad_edge:.2f}, so the pad stands across the opening; drop "
            f"cable_slot_width under {2 * (pad_edge - max(usb_y, uart_y)):.1f}",
            param="cable_slot_width",
        )

    pads = [
        (sx, sy)
        for sx in (-mount_x / 2, mount_x / 2)
        for sy in (-mount_y / 2, mount_y / 2)
    ]
    for sx, sy in pads:
        dx = 1.0 if sx > 0 else -1.0
        dy = 1.0 if sy > 0 else -1.0
        body = body + Pos(
            dx * (pocket_w / 2 - pad_x / 2),
            dy * (pocket_h / 2 - pad_y / 2),
            seat_z / 2,
        ) * Box(pad_x, pad_y, seat_z)

    # USB-C / UART, cut after the pads so a pad corner cannot leave a tongue in
    # the channel. The sockets hang under the PCB, so the channel runs from
    # under the bed face up to the PCB's own back plane, which roofs it. Any
    # higher and it would slice the frame rim.
    pcb_back_z = rim_z - measured("module_glass_to_pcb")
    for y in (usb_y, uart_y):
        body = body - Pos(slot_x, y, (pcb_back_z - 1) / 2) * Box(
            cable_slot_depth + 0.2, cable_slot_width, pcb_back_z + 1
        )

    # Sockets for screen_base's foot pins, drilled into the bottom face. The
    # bottom border is solid from -outer_h/2 to -pocket_h/2, so there is
    # (outer_h - pocket_h) / 2 of material and the socket must not eat it all.
    border_solid = (outer_h - pocket_h) / 2
    if foot_hole_depth > border_solid - 0.5:
        reject(
            f"foot_hole_depth {foot_hole_depth} leaves under 0.5mm behind the "
            f"socket in a {border_solid:.1f}mm border; shorten it or raise "
            f"border_top_bottom",
            param="foot_hole_depth",
        )
    for sx in (-foot_hole_span / 2, foot_hole_span / 2):
        body = body - Pos(
            sx, -outer_h / 2 + foot_hole_depth / 2 - 0.5, rim_z / 2
        ) * Box(foot_hole_width, foot_hole_depth + 1, foot_hole_width)

    # Counterbore mouths on the bed (M2.5 medium clearance).
    for sx, sy in pads:
        body = body - Pos(sx, sy, 0) * counterbore(
            hole, head_dia + 0.4, head_h + 0.2, seat_z
        )

    if draft:
        return body

    def in_pocket(b):
        return (
            abs(b.min.X + b.max.X) / 2 < pocket_w / 2 + 0.05
            and abs(b.min.Y + b.max.Y) / 2 < pocket_h / 2 + 0.05
        )

    def buried(edge):
        # Inside the pocket, below the seat: the module covers all of it, and a
        # 1mm chamfer there only shaves the seat pads into slivers.
        b = edge.bounding_box()
        return b.max.Z < seat_z + 0.05 and in_pocket(b)

    def against_glass(edge):
        # The rim edge running round the top of the pocket is the one the glass
        # sits in. A 1mm chamfer there opens the pocket by 1mm a side, so the
        # 0.5mm fit gap reads as a 2.5mm bevelled shadow all the way round.
        # This edge stays sharp; the outer rim, on the outside face, does not.
        b = edge.bounding_box()
        return b.min.Z > rim_z - 0.05 and in_pocket(b)

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    keep = keep.filter_by(lambda e: not buried(e) and not against_glass(e))
    return polish(body, keep, 1.0)

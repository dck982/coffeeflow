from nurb import *

# Waveshare 4.3 carrier. Print pose: large rear plate on the bed, solid border
# skirt rising, module pocket cut through it. The skirt top *is* the frame: it
# lands level with the glass, so there is no plate bridging the pocket. The
# module's own four M2.5 standoffs land on the seat pads and the screws come up
# from under the bed face into them. USB-C / UART channels run through the
# skirt and out the outer face as well as the rear plate, so a wrapping base
# can still reach the plugs from the side.


@part
def screen_wedge(
    border_side=10.0,
    border_top_bottom=5.0,
    rear_wall=2.5,
    pocket_depth=12.8,
    module_clearance_width=0.25,
    module_clearance_height=0.5,
    rear_frame_margin=0.0,
    seat_height=4.3,
    cable_slot_width=9.6,
    cable_slot_depth=7.0,
    outer_wall=0.0,
    connector_window_from_top=12.5,
    connector_window_length=41.0,
    connector_window_margin=1.5,
    cable_relief_depth=0.6,
    cable_relief_width=18.0,
    cable_relief_from_usb_edge=19.3,
    cable_relief_height=5.0,
    cable_relief_below_rim=1.0,
    glass_lip_depth=0.8,
    glass_lip_height=1.0,
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
    module_clearance_width: free fit each side of the module, left and right.
        Half what it is top and bottom: the real board came in 0.5mm narrower
        than the drawing's 106.10 across the pocket
    module_clearance_height: free fit above and below the module. The pocket is
        sized to the PCB here, which is the tallest thing in the stack
    rear_frame_margin: how far the rear opening stops short of the seat pads.
        0 puts it flush with them, so the rear plate is a frame exactly the
        width of the pads and the whole back of the module is reachable
    seat_height: how far the four seat pads stand off the rear plate. Under
        3.2 the counterbore's bridging steps leave too little pad above them.
        It also sets the screw: an M2.5x8 crosses rear_wall + seat_height minus
        the 2.7 head pocket, and whatever is left of the 8 goes into the
        module's 4mm standoff, so raising this shortens the thread engagement
    cable_slot_width: width of the USB-C / UART channels, set by the bare
        right-angle plug's 8.34mm shell plus clearance
    cable_slot_depth: how far the plug stands out past the socket face; the
        opening itself runs past this to the outer face when outer_wall is 0
    outer_wall: material left between the channel and the outside face. 0
        opens the outer face so a wrapping base can reach the plugs from the
        side; raise it to close the skirt again
    connector_window_from_top: the JST connectors live on the edge *opposite*
        the USB-C / UART one, and plug in perpendicular to the PCB, so the rear
        frame is notched out to the pocket wall in front of them. This is the
        distance from the module's top edge to the nearest connector edge
    connector_window_length: how far that run of connectors reaches, from the
        first connector to the far side of the battery connector
    connector_window_margin: added to the window at each end, so a caliper
        reading off by a millimetre does not put plastic over a latch
    cable_relief_depth: how much thinner the bottom skirt gets where the LCD's
        flat cable wraps round the panel edge and stands proud of the PCB
    cable_relief_width: how wide that thinned stretch is, along the bottom edge.
        The cable is a lozenge, 15mm across where it stands proudest, so this
        carries margin on both sides rather than tracking that 15 exactly
    cable_relief_from_usb_edge: centre of the thinned stretch, measured in from
        the same edge the USB-C and UART sockets are on, i.e. onto the yellow
        CAN connector. The cable is on the connector side, not the far side
    cable_relief_height: how tall it is, matching the LCD stack the cable wraps
    cable_relief_below_rim: frame left full thickness above the relief, so the
        glass still sits against an unbroken rim
    glass_lip_depth: the PCB runs past the glass along the bottom edge only, so
        the pocket's bottom wall carries a lip that overhangs the glass by this
        much. The module goes in at an angle, bottom first, and the PCB slides
        under it; the glass then lands against the lip with the same gap it has
        everywhere else
    glass_lip_height: how far that lip reaches down from the rim. It has to stop
        well above the PCB's front face or the board cannot pass under it, so it
        covers the glass thickness and nothing more
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
    pocket_w = module_w + 2 * module_clearance_width
    pocket_h = module_h + 2 * module_clearance_height

    usb_y = module_h / 2 - usb_from_top
    uart_y = module_h / 2 - uart_from_top

    # Pad size is arithmetic, and the rear frame is cut to it, so it has to be
    # known before the openings even though the pads are built much later.
    # Sized to the doctrine's "a loaded hole earns a fastener diameter of
    # wall", so 2.5mm of PETG around the M2.5 bore and not a millimetre more:
    # the top pads share the border with the UART plug and every extra
    # millimetre here is one the plug does not get.
    pad_wall = 2.5
    pad_x = (pocket_w / 2 - mount_x / 2) + hole / 2 + pad_wall
    pad_y = (pocket_h / 2 - mount_y / 2) + hole / 2 + pad_wall

    # The rear plate is a frame the width of the seat pads: everything inboard
    # of them is void. The pads take their strength from the skirt they are
    # fused into, and the plate around the counterbore carries nothing (it sits
    # *below* the shoulder the head bears on), so the plate's only job inboard
    # of the pads was to exist.
    frame_x = pocket_w / 2 - pad_x + rear_frame_margin
    frame_y = pocket_h / 2 - pad_y + rear_frame_margin
    bore_wall = min(
        mount_x / 2 - (head_dia + 0.4) / 2 - frame_x,
        mount_y / 2 - (head_dia + 0.4) / 2 - frame_y,
    )
    if bore_wall < 1.2:
        reject(
            f"rear_frame_margin {rear_frame_margin} leaves {bore_wall:.2f}mm of "
            f"plate around the {head_dia + 0.4:.1f}mm head pocket, under the "
            f"1.2mm the printer can lay; raise it past "
            f"{rear_frame_margin + 1.2 - bore_wall:.2f}",
            param="rear_frame_margin",
        )
    left_over = border_side - module_clearance_width - cable_slot_depth - 0.1
    if outer_wall < 0:
        reject(
            f"outer_wall {outer_wall} is negative: raise it to 0 to open the "
            "outer face, or above 0 to leave a closed wall",
            param="outer_wall",
        )
    if left_over < outer_wall:
        reject(
            f"cable_slot_depth {cable_slot_depth} leaves only {left_over:.1f}mm of "
            f"outer wall, under outer_wall {outer_wall}; shorten it or widen "
            f"border_side past {module_clearance_width + cable_slot_depth + 0.1 + outer_wall:.1f}",
            param="cable_slot_depth",
        )
    slot_inner = pocket_w / 2 - 0.1
    if outer_wall < 0.05:
        slot_outer = outer_w / 2 + 0.2
    else:
        slot_outer = outer_w / 2 - outer_wall
    slot_len = slot_outer - slot_inner
    slot_x = (slot_inner + slot_outer) / 2

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
    body = body - Pos(0, 0, rear_wall / 2) * Box(
        2 * frame_x, 2 * frame_y, rear_wall + 2
    )

    # The connector notch, from the frame's lip out to the pocket wall. It runs
    # 1mm into the opening that is already void, so no cut face lands coplanar
    # with another. It cannot go past the pocket wall: beyond it is the 9.5mm
    # rail that ties the skirt to the rear plate on this side.
    win_top = module_h / 2 - connector_window_from_top + connector_window_margin
    win_bot = win_top - connector_window_length - 2 * connector_window_margin
    if win_top > frame_y or win_bot < -frame_y:
        reject(
            f"the connector window runs y {win_bot:.2f} to {win_top:.2f} but the "
            f"seat pads start at y {frame_y:.2f}, so it would undercut a screw "
            f"pad; shorten connector_window_length below "
            f"{2 * frame_y - 2 * connector_window_margin:.1f}",
            param="connector_window_length",
        )
    win_w = pocket_w / 2 - frame_x + 1.0
    body = body - Pos(
        -(pocket_w / 2 + frame_x - 1.0) / 2,
        (win_bot + win_top) / 2,
        rear_wall / 2,
    ) * Box(win_w, win_top - win_bot, rear_wall + 2)

    seat_z = rear_wall + seat_height

    # The LCD's flat cable wraps round the panel edge and stands proud of the
    # PCB outline, so the bottom skirt is thinned over its width only. It stops
    # cable_relief_below_rim short of the top: that lip is the glass thickness,
    # and keeping it full width is what stops the frame line breaking where the
    # relief is. The lip therefore prints as a cable_relief_depth overhang.
    relief_cx = module_w / 2 - cable_relief_from_usb_edge
    relief_top = rim_z - cable_relief_below_rim
    relief_bot = relief_top - cable_relief_height
    border_solid = (outer_h - pocket_h) / 2
    socket_top_z = rim_z / 2 + foot_hole_width / 2
    if relief_bot < socket_top_z:
        left_behind = border_solid - cable_relief_depth - foot_hole_depth
        if left_behind < 0.5:
            reject(
                f"the relief reaches down to z {relief_bot:.2f} while the foot "
                f"sockets top out at z {socket_top_z:.2f}, and behind the thinned "
                f"wall only {left_behind:.1f}mm is left of the {border_solid:.1f}mm "
                f"border; raise cable_relief_height or shorten foot_hole_depth",
                param="cable_relief_height",
            )
    if cable_relief_depth > 0:
        body = body - Pos(
            relief_cx,
            -(pocket_h + cable_relief_depth) / 2,
            (relief_bot + relief_top) / 2,
        ) * Box(cable_relief_width, cable_relief_depth, cable_relief_height)

    # The PCB is glass_lip_depth taller than the glass in y, and all of that
    # extra sits on the *bottom* edge (Waveshare's own room for the flat cable).
    # The pocket is therefore sized to the PCB, which leaves the glass floating
    # in an oversize hole at the bottom. A lip along the bottom wall, in the
    # glass band only, closes it: the module goes in at an angle, bottom first,
    # the PCB passes under the lip, and the glass then lands against it with the
    # same module_clearance_height it has on the other three sides.
    lip_bot = rim_z - glass_lip_height
    pcb_front_z = rim_z - measured("module_glass_to_pcb")
    if glass_lip_depth > 0 and lip_bot < pcb_front_z + 1.0:
        reject(
            f"the lip reaches down to z {lip_bot:.2f} but the PCB's front face is "
            f"at z {pcb_front_z:.2f}, so the board cannot slide under it; drop "
            f"glass_lip_height below "
            f"{measured('module_glass_to_pcb') - 1.0:.1f}",
            param="glass_lip_height",
        )
    if glass_lip_depth > 0:
        body = body + Pos(
            0,
            -(pocket_h - glass_lip_depth) / 2,
            (lip_bot + rim_z) / 2,
        ) * Box(pocket_w, glass_lip_depth, glass_lip_height)

    # --- four seat pads standing in the pocket; the module lands on these ---
    # Rectangular and flush into the pocket corner: a round pad leaves a wedge
    # of void between its arc and the corner, too thin for the printer to lay.
    # pad_x / pad_y are worked out above, with the rear frame that is cut to
    # them.
    #
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
    # higher and it would slice the frame rim. With outer_wall 0 the cut also
    # breaks the outer face, so the plugs can be reached from the side once a
    # wrapping base hides this skirt.
    pcb_back_z = rim_z - measured("module_glass_to_pcb")
    for y in (usb_y, uart_y):
        body = body - Pos(slot_x, y, (pcb_back_z - 1) / 2) * Box(
            slot_len, cable_slot_width, pcb_back_z + 1
        )

    # Sockets for screen_base's foot pins, drilled into the bottom face. The
    # bottom border is solid from -outer_h/2 to -pocket_h/2, so there is
    # (outer_h - pocket_h) / 2 of material and the socket must not eat it all.
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
        # Inside the pocket, below the PCB's back plane: the module covers all
        # of it, and a 1mm chamfer there only shaves the seat pads into
        # slivers. The ceiling is pcb_back_z and not seat_z because the two
        # channel mouths rise to exactly that plane, and they are as hidden as
        # anything at the seat: a chamfer on the mouth against the top pad ran
        # 0.72mm past the pad's edge in the pocket-wall plane, which reads as a
        # bitten-off pad corner while removing nothing but skirt.
        b = edge.bounding_box()
        return b.max.Z < pcb_back_z + 0.05 and in_pocket(b)

    def against_glass(edge):
        # The rim edge running round the top of the pocket is the one the glass
        # sits in. A 1mm chamfer there opens the pocket by 1mm a side, so the
        # 0.5mm fit gap reads as a 2.5mm bevelled shadow all the way round.
        # This edge stays sharp; the outer rim, on the outside face, does not.
        b = edge.bounding_box()
        return b.min.Z > rim_z - 0.05 and in_pocket(b)

    def in_relief(edge):
        # The relief is 0.6mm deep and the module hides all of it. A 1mm
        # chamfer here is bigger than the feature it would be chamfering.
        b = edge.bounding_box()
        return (
            b.min.Z > relief_bot - 0.05
            and b.max.Z < relief_top + 0.05
            and b.min.Y < -pocket_h / 2 + 0.05
            and b.min.X > relief_cx - cable_relief_width / 2 - 0.05
            and b.max.X < relief_cx + cable_relief_width / 2 + 0.05
        )

    def on_glass_lip(edge):
        # The lip's underside runs the width of the pocket at z = lip_bot. A 1mm
        # chamfer is larger than the 0.8mm it would be chamfering, and it would
        # eat the very edge the glass is meant to land against.
        b = edge.bounding_box()
        return (
            abs(b.min.Z - lip_bot) < 0.05
            and abs(b.max.Z - lip_bot) < 0.05
            and b.min.Y > -pocket_h / 2 + 0.05
            and b.max.Y < -pocket_h / 2 + glass_lip_depth + 0.05
        )

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    keep = keep.filter_by(
        lambda e: not buried(e)
        and not against_glass(e)
        and not in_relief(e)
        and not on_glass_lip(e)
    )
    return polish(body, keep, 1.0)

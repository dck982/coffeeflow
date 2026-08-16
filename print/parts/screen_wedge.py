from nurb import *

# Waveshare 4.3 carrier. Print pose: large rear plate on the bed, solid border
# skirt rising, module pocket cut through it. The skirt top *is* the frame: it
# lands level with the glass, so there is no plate bridging the pocket. The
# module's own four M2.5 standoffs land on the seat pads and the screws come up
# from under the bed face into them. Outer face stays closed; USB-C / UART drop
# through skirt channels and out the rear plate.


@part
def screen_wedge(
    border_side=20.0,
    border_top_bottom=10.0,
    rear_wall=2.5,
    pocket_depth=12.8,
    module_clearance=0.5,
    cutout_inset=12.0,
    seat_height=4.0,
    cable_slot_width=14.5,
    cable_slot_depth=16.0,
    draft=False,
):
    """Open-back Waveshare 4.3 carrier: solid skirt, frame rim level with the glass.

    border_side: how far the frame sticks past the glass left and right
    border_top_bottom: how far the frame sticks past the glass top and bottom
    rear_wall: thickness of the large rear plate (on the bed)
    pocket_depth: from the seat pad tops up to the frame rim, so the rim comes
        out level with the glass: 8.80 glass to PCB plus the module's own 4.00
        M2.5 standoffs, which are what the pads carry
    module_clearance: free fit around the module inside the pocket
    cutout_inset: how far the rear opening stops short of the screw pads
    seat_height: how far the four seat pads stand off the rear plate. Under
        3.2 the counterbore's bridging steps leave too little pad above them
    cable_slot_width: width of the USB-C / UART channels
    cable_slot_depth: how far each channel runs from the pocket into the border:
        the right-angle adapter body plus the travel to plug it in
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
    border_t = (outer_w - pocket_w) / 2
    if cable_slot_depth > border_t - 0.5:
        reject(
            f"cable_slot_depth {cable_slot_depth} would break the outer wall; "
            f"keep it under {border_t - 0.5:.1f}",
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
    # Square and flush into the pocket corner: a round pad leaves a wedge of
    # void between its arc and the corner, too thin for the printer to lay.
    pad_span = (
        max(pocket_w / 2 - mount_x / 2, pocket_h / 2 - mount_y / 2) + hole / 2 + 3.0
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
            dx * (pocket_w / 2 - pad_span / 2),
            dy * (pocket_h / 2 - pad_span / 2),
            seat_z / 2,
        ) * Box(pad_span, pad_span, seat_z)

    # USB-C / UART, cut after the pads so a pad corner cannot leave a tongue in
    # the channel. The sockets hang under the PCB, so the channel runs from
    # under the bed face up to the PCB's own back plane, which roofs it. Any
    # higher and it would slice the frame rim.
    pcb_back_z = rim_z - measured("module_glass_to_pcb")
    for y in (usb_y, uart_y):
        body = body - Pos(slot_x, y, (pcb_back_z - 1) / 2) * Box(
            cable_slot_depth + 0.2, cable_slot_width, pcb_back_z + 1
        )

    # Counterbore mouths on the bed (M2.5 medium clearance).
    for sx, sy in pads:
        body = body - Pos(sx, sy, 0) * counterbore(
            hole, head_dia + 0.4, head_h + 0.2, seat_z
        )

    if draft:
        return body

    def buried(edge):
        # Inside the pocket, below the seat: the module covers all of it, and a
        # 1mm chamfer there only shaves the seat pads into slivers.
        b = edge.bounding_box()
        return (
            b.max.Z < seat_z + 0.05
            and abs(b.min.X + b.max.X) / 2 < pocket_w / 2 + 0.05
            and abs(b.min.Y + b.max.Y) / 2 < pocket_h / 2 + 0.05
        )

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    keep = keep.filter_by(lambda e: not buried(e))
    return polish(body, keep, 1.0)

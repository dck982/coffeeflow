from nurb import *
from math import radians, tan, sin, cos, hypot

# Side profile, front (south) to back:
#   - front wall rising to the lip crest
#   - the lip: a face perpendicular to the seat slope, 19.6mm long (the full
#     thickness of screen_wedge), so the wedge's bottom face bears on it. It
#     lives on the side walls and on the ergots only, never as a full-width
#     plate: a plate there is a 30 deg downward face over 2470mm2.
#   - the notch, then the seat slope at 60 deg, backing the whole wedge
#   - the ridge, then a back face falling to the end of the footing
#
# Each side wall is a rebate: a shelf whose top lies on the slope, for the
# wedge's border to rest on, and a rail standing proud of it that stops the
# wedge moving sideways. The wedge slides down between the rails onto the foot
# pins. Seen from above the box is empty; the wedge is the lid, and it is also
# the shear panel that keeps an open-top shell from racking.

WEDGE_THICKNESS = 19.60
WEDGE_LENGTH = 77.80
ACTIVE_TOP = 9.09 + 54.36        # top of the touchscreen, up from the wedge's foot


@part
def screen_base(
    wedge_width=126.1,
    seat_height=30.0,
    backing=WEDGE_LENGTH,
    tilt=45.0,
    wall=2.0,
    floor=2.0,
    footing=0.0,
    footing_edge=4.0,
    channel_fit=0.4,
    rail_width=2.0,
    rail_height=3.0,
    shelf_width=4.0,
    lip_ribs=2,
    rib_width=12.0,
    rib_span=60.0,
    foot_pin_diameter=5.0,
    foot_pin_length=3.0,
    side_panel_depth=WEDGE_THICKNESS,
    side_panel_fit=-0.1,
    draft=False,
):
    """Hollow wedge cradle. The wedge lies on the slope and is the lid.

    wedge_width: the wedge this cradles, 126.1. The box comes out wider by the
        channel fit and the two rails
    seat_height: box height where the wedge's bottom edge lands, set by the
        electronics that have to fit under the screen
    backing: how much of the wedge's 77.8mm lies on the slope. 77.8 backs all of
        it, so a press anywhere on the glass is compression into the slope; under
        63.5 the top of the touchscreen overhangs the ridge and a press rocks the
        screen. It sets the ridge height, which is why seat_height and tilt can
        move without changing what the screen rests on
    tilt: seat slope, i.e. the screen angle
    wall: wall thickness
    floor: bottom thickness
    footing: how far the floor runs behind the ridge, the mason's footing. At 0
        the back face drops straight down from the ridge instead, for mounting
        against a vertical surface, and footing_edge is unused
    footing_edge: height of the short vertical face at the back tip, so the
        falling face does not end in a feather edge. Only applies when footing
        is above 0
    channel_fit: total play between the wedge and the two rails, side to side
    rail_width: thickness of the rail that stops the wedge sliding sideways
    rail_height: how far each rail stands proud of the seat
    shelf_width: the ledge inside each rail that the wedge's border rests on
    lip_ribs: how many ergots carry the lip between the two side walls
    rib_width: how wide each ergot is
    rib_span: centre-to-centre of the ergots
    foot_pin_diameter: the pin on each ergot, plugging into the wedge's foot
    foot_pin_length: how far each foot pin stands out of the lip face
    side_panel_depth: how far the two closing side panels reach out from the
        seat, perpendicular to the slope. Has to clear the wedge's full
        19.6mm thickness or its USB-C openings stay exposed above the low
        rail
    side_panel_fit: clearance between the side panels and the wedge's edge,
        negative for a press fit so the wedge grips instead of sliding loose
    """
    t = radians(tilt)
    s, c = sin(t), cos(t)

    channel_half = wedge_width / 2 + channel_fit / 2
    outer_half = channel_half + rail_width
    width = 2 * outer_half

    crest = seat_height + WEDGE_THICKNESS * c
    seat_y = WEDGE_THICKNESS * s
    # The ridge is where `backing` mm up the slope lands, so seat_height and tilt
    # move without changing how much of the wedge the slope carries.
    north_height = seat_height + backing * s
    run = backing * c
    north_y = seat_y + run
    depth = north_y + footing

    if backing < ACTIVE_TOP:
        reject(
            f"the slope backs {backing:.1f}mm of the wedge but the touchscreen "
            f"reaches {ACTIVE_TOP:.1f}mm, so a press up there rocks the screen off "
            f"its seat; raise backing past {ACTIVE_TOP:.1f}",
            param="backing",
        )
    if backing > WEDGE_LENGTH:
        reject(
            f"backing {backing:.1f}mm is more slope than the wedge's "
            f"{WEDGE_LENGTH:.1f}mm to lie on it, so the ridge stands proud of the "
            "screen",
            param="backing",
        )
    if footing > 0 and footing_edge >= north_height:
        reject("footing_edge must stay under north_height", param="footing_edge")
    if shelf_width < 3.0:
        reject(
            f"shelf_width {shelf_width} is less than the 3mm of ledge the wedge's "
            "border needs to sit on",
            param="shelf_width",
        )
    if side_panel_depth < WEDGE_THICKNESS:
        reject(
            f"side_panel_depth {side_panel_depth:.1f}mm is under the wedge's "
            f"{WEDGE_THICKNESS:.1f}mm thickness, so its USB-C openings stay "
            "exposed past the panel",
            param="side_panel_depth",
        )
    up = Vector(0, c, s)                       # up the slope
    n = Vector(0, -s, c)                       # out of the seat
    notch = Vector(0, seat_y, seat_height)

    # --- the seat silhouette: the shelf, and the outline the shell is clipped
    #     to so nothing overshoots. With no footing the back face is already
    #     vertical, so footing_edge's own short face would be a zero-length
    #     redundant vertex; it only earns its place once footing pushes the
    #     back face into a slope that would otherwise end in a feather edge. ---
    back_tail = (
        [(north_y, north_height), (depth, footing_edge), (depth, 0.0)]
        if footing > 0
        else [(north_y, north_height), (depth, 0.0)]
    )
    seat_pts = [
        (0.0, 0.0),
        (0.0, crest),
        (seat_y, seat_height),
        *back_tail,
    ]
    seat_profile = Plane.YZ * Polygon(*seat_pts, align=None)
    silhouette = extrude(seat_profile, amount=outer_half, both=True)

    # --- the rail silhouette: the same, with the slope pushed out by
    #     rail_height so the rail stands proud of the shelf ---
    ridge = Vector(0, north_y, north_height)
    rail_notch = notch + n * rail_height
    rail_ridge = ridge + n * rail_height
    rail_pts = [
        (0.0, 0.0),
        (0.0, crest),
        (rail_notch.Y, rail_notch.Z),
        (rail_ridge.Y, rail_ridge.Z),
        *back_tail,
    ]
    rail_profile = Plane.YZ * Polygon(*rail_pts, align=None)

    # --- floor, front wall, back face ---
    body = Pos(0, depth / 2, floor / 2) * Box(width, depth, floor)
    body += Pos(0, wall / 2, crest / 2) * Box(width, wall, crest)

    ay, az = north_y, north_height
    by, bz = (depth, footing_edge) if footing > 0 else (depth, 0.0)
    length = hypot(by - ay, bz - az)
    back_n = Vector(0, (az - bz) / length, (by - ay) / length)
    back_mid = Vector(0, (ay + by) / 2, (az + bz) / 2)
    body += Plane(
        origin=back_mid - back_n * (wall / 2), x_dir=(1, 0, 0), z_dir=back_n
    ) * Box(width, length + 4 * wall, wall)

    # --- the ergots: the lip triangle only, standing on the floor ---
    lip = Plane.YZ * Polygon(
        (0.0, 0.0), (0.0, crest), (seat_y, seat_height), align=None
    )
    xs = (
        [0.0]
        if lip_ribs == 1
        else [-rib_span / 2 + i * rib_span / (lip_ribs - 1) for i in range(lip_ribs)]
    )
    for px in xs:
        body += Pos(px, 0, 0) * extrude(lip, amount=rib_width / 2, both=True)

    body = body & silhouette

    # --- the rebate: shelf on the slope, rail standing proud of it ---
    for sx in (-1, 1):
        shelf_mid = sx * (channel_half - shelf_width / 2)
        body += Pos(shelf_mid, 0, 0) * extrude(
            seat_profile, amount=shelf_width / 2, both=True
        )
        rail_mid = sx * (channel_half + rail_width / 2)
        body += Pos(rail_mid, 0, 0) * extrude(
            rail_profile, amount=rail_width / 2, both=True
        )

    # --- side panels: close the outer sides over the wedge's full thickness,
    #     not just the rail's rail_height stub, so the USB-C openings
    #     screen_wedge cuts into its own border are hidden rather than
    #     showing above the low rail. Positioned off wedge_width itself, with
    #     side_panel_fit negative, so the wedge presses into place. Follows
    #     the wedge's own back corner exactly, flush, rather than cutting a
    #     chamfer there: cutting it back exposes the wedge behind it, since
    #     the wedge fills right up to that corner too. screen_wedge drops its
    #     own chamfer on that same edge so the two land flush with no reveal.
    #     Added after the notch relief above so that cut never touches it. ---
    press_half = wedge_width / 2 + side_panel_fit / 2
    notch_out = notch + n * side_panel_depth
    ridge_out = ridge + n * side_panel_depth
    side_panel_profile = Plane.YZ * Polygon(
        (seat_y, seat_height),
        (north_y, north_height),
        (ridge_out.Y, ridge_out.Z),
        (notch_out.Y, notch_out.Z),
        align=None,
    )
    for sx in (-1, 1):
        panel_mid = sx * (press_half + outer_half) / 2
        body += Pos(panel_mid, 0, 0) * extrude(
            side_panel_profile, amount=(outer_half - press_half) / 2, both=True
        )

    # --- foot pins: one per ergot, standing out of the lip face along the
    #     slope. They lean 30 deg off vertical, so their own underside sits at
    #     60 deg and prints without support. Pins normal to the seat instead
    #     would lean 60 deg and overhang at 30. ---
    for px in xs:
        base = notch + n * (WEDGE_THICKNESS / 2) + Vector(px, 0, 0)
        body += Plane(origin=base, z_dir=up) * Cylinder(
            foot_pin_diameter / 2, foot_pin_length,
            align=(Align.CENTER, Align.CENTER, Align.MIN),
        )

    if draft:
        return body

    # The wedge has to lie flat on this, so every face it touches stays sharp:
    # a 1mm chamfer on the seat or the lip is a 1mm gap under the screen.
    def on_plane(edge, origin, normal):
        b = edge.bounding_box()
        for py in (b.min.Y, b.max.Y):
            for pz in (b.min.Z, b.max.Z):
                if abs((py - origin.Y) * normal.Y + (pz - origin.Z) * normal.Z) > 0.05:
                    return False
        return True

    def on_flat_face(edge, x):
        # any edge lying flat on a face perpendicular to X, at +-x — used for
        # the panel's inner (mating) face at press_half and the shelf's inner
        # face at channel_half - shelf_width. Chamfering any edge on either
        # face recedes it behind the wedge, which presses or rests flush
        # against the whole face, not just the one named edge a narrower
        # check might anticipate.
        b = edge.bounding_box()
        return abs(b.min.X - x) < 0.05 and abs(b.max.X - x) < 0.05

    shelf_inner = channel_half - shelf_width

    def seating(edge):
        return (
            on_plane(edge, notch, n)
            or on_plane(edge, notch, up)
            or on_plane(edge, notch_out, n)
            or on_flat_face(edge, press_half)
            or on_flat_face(edge, -press_half)
            or on_flat_face(edge, shelf_inner)
            or on_flat_face(edge, -shelf_inner)
        )

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    keep = keep.filter_by(lambda e: not seating(e))
    return polish(body, keep, 1.0)

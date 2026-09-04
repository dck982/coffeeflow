from nurb import *
from math import radians, tan, sin, cos, hypot, sqrt

from system import MARGE_PUIT, PETIT_PUIT_DIAMETRE, anti_tirage_ns, puit_couche

# Side profile, front (south) to back:
#   - front wall rising to the lip crest
#   - the lip: a face perpendicular to the seat slope, 19.6mm long (the full
#     thickness of screen_wedge), so the wedge's bottom face bears on it. It
#     lives on the side walls and on the ergots only, never as a full-width
#     plate: a plate there is a 30 deg downward face over 2470mm2.
#   - the notch, then the seat slope at 60 deg, backing the whole wedge
#   - the ridge, then a back face dropping straight down to the floor, plaqued
#     against a vertical surface
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
    seat_height=5.0,
    backing=WEDGE_LENGTH,
    tilt=45.0,
    wall=2.0,
    floor=2.0,
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
    back_opening_width=16.0,
    back_opening_height=16.0,
    back_opening_from_left=25.0,
    back_opening_below_top=40.0,
    back_opening_chamfer=4.0,
    magnet_diameter=PETIT_PUIT_DIAMETRE,
    magnet_cover=0.6,
    magnet_wall=1.6,
    magnet_slot_gap=2.2,
    straight_magnet_diameter=8.2,
    straight_magnet_height=3.0,
    straight_magnet_cover=0.6,
    straight_magnet_from_right=15.0,
    cable_tie_gap=1.2,
    cable_tie_slot_width=3.0,
    cable_tie_span=5.6,
    cable_tie_height=5.0,
    cable_tie_low_z=15.0,
    cable_tie_right_high_z=30.0,
    cable_tie_left_z=37.0,
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
    back_opening_width: width of the port cut through the back face
    back_opening_height: height of the port cut through the back face
    back_opening_from_left: distance from the base's left edge to the port's centre
    back_opening_below_top: distance down from the top of the back face to the
        port's centre
    back_opening_chamfer: size of the 45deg cut on the port's top two corners,
        standing in for a circle without the overhang a round hole would print
    magnet_diameter: bore diameter for the two magnets set into the back
        face, near the top, flush with the seat plane so they sit right
        under the wedge. Each well runs through to the back face for the
        magnet to be pushed in from behind. Defaults to system.py's
        PETIT_PUIT_DIAMETRE, 0.2mm over the 5mm disc — this well is its own
        angled shape (a ramp cut into a sloped face), not the shared
        petit_puit, but the diameter matches screen_wedge's
    magnet_cover: plastic left over the magnet on the seat-facing side, thin
        enough for the magnet to still act through it
    magnet_wall: plastic thickness wrapped around each magnet, front and sides
    magnet_slot_gap: width of the coin slot each of these two magnets slides
        in through, across its own 2mm thickness
    straight_magnet_diameter: bore for a third magnet, Ø8x3, on a
        horizontal (lying) axis through the back face rather than off the
        sloped ramp the pair above stand on
    straight_magnet_height: how deep that magnet's pocket runs
    straight_magnet_cover: plastic left on the exterior back face over that
        magnet
    straight_magnet_from_right: distance from the base's right edge to that
        magnet's centre
    cable_tie_gap: clearance between the ring and the wall, enough for a
        cable tie to pass through
    cable_tie_slot_width: width of the vertical channel the tie threads
        through
    cable_tie_span: overall width of the ring across both legs
    cable_tie_height: how tall the ring's channel stands
    cable_tie_low_z: height up the back face of the lower ring, at the
        right-hand magnet well's X
    cable_tie_right_high_z: height up the back face of the upper ring at the
        right-hand magnet well's X
    cable_tie_left_z: height up the back face of the lone ring at the
        left-hand magnet well's X
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
    depth = north_y

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
    if shelf_width < 3.0:
        reject(
            f"shelf_width {shelf_width} is less than the 3mm of ledge the wedge's "
            "border needs to sit on",
            param="shelf_width",
        )
    if back_opening_chamfer * 2 >= back_opening_height:
        reject(
            f"back_opening_chamfer {back_opening_chamfer:.1f}mm leaves no flat top "
            f"on a {back_opening_height:.1f}mm-tall opening",
            param="back_opening_chamfer",
        )
    back_opening_x = -outer_half + back_opening_from_left
    back_opening_z = north_height - back_opening_below_top
    if abs(back_opening_x) + back_opening_width / 2 > outer_half:
        reject(
            f"back_opening_from_left {back_opening_from_left:.1f}mm puts the "
            f"{back_opening_width:.1f}mm-wide opening past the base's {width:.1f}mm "
            "width",
            param="back_opening_from_left",
        )
    if (
        back_opening_z - back_opening_height / 2 < 0
        or back_opening_z + back_opening_height / 2 > north_height
    ):
        reject(
            f"back_opening_below_top {back_opening_below_top:.1f}mm puts the "
            f"{back_opening_height:.1f}mm-tall opening past the {north_height:.1f}mm "
            "back face",
            param="back_opening_below_top",
        )
    if side_panel_depth < WEDGE_THICKNESS:
        reject(
            f"side_panel_depth {side_panel_depth:.1f}mm is under the wedge's "
            f"{WEDGE_THICKNESS:.1f}mm thickness, so its USB-C openings stay "
            "exposed past the panel",
            param="side_panel_depth",
        )
    if cable_tie_slot_width < 2.5:
        reject(
            f"cable_tie_slot_width {cable_tie_slot_width:.1f}mm is tighter "
            "than a cable tie needs: raise it",
            param="cable_tie_slot_width",
        )
    if cable_tie_gap < 1.0:
        reject(
            f"cable_tie_gap {cable_tie_gap:.1f}mm will not pass a cable tie: "
            "raise it",
            param="cable_tie_gap",
        )
    cable_tie_leg = 0.5 * (cable_tie_span - cable_tie_slot_width)
    if cable_tie_leg < 1.0:
        reject(
            f"cable_tie_span {cable_tie_span:.1f}mm leaves only "
            f"{cable_tie_leg:.1f}mm each side of the {cable_tie_slot_width:.1f}mm "
            "slot: raise it",
            param="cable_tie_span",
        )
    cable_tie_out = cable_tie_gap + wall
    for label, z_center in (
        ("cable_tie_low_z", cable_tie_low_z),
        ("cable_tie_right_high_z", cable_tie_right_high_z),
        ("cable_tie_left_z", cable_tie_left_z),
    ):
        top = z_center + cable_tie_height / 2 + cable_tie_out
        bottom = z_center - cable_tie_height / 2 - cable_tie_out
        if top > north_height:
            reject(
                f"{label} {z_center:.1f}mm plus the ring's own height runs "
                f"past the {north_height:.1f}mm back face",
                param=label,
            )
        if bottom < 0:
            reject(f"{label} {z_center:.1f}mm puts the ring below the floor", param=label)
    up = Vector(0, c, s)                       # up the slope
    n = Vector(0, -s, c)                       # out of the seat
    notch = Vector(0, seat_y, seat_height)

    # --- the seat silhouette: the shelf, and the outline the shell is clipped
    #     to so nothing overshoots. The back face is vertical, plaqued
    #     against a wall, so the tail is a straight drop from the ridge. ---
    back_tail = [(north_y, north_height), (depth, 0.0)]
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
    back_n = Vector(0, 1, 0)
    back_mid = Vector(0, ay, az / 2)
    body += Plane(
        origin=back_mid - back_n * (wall / 2), x_dir=(1, 0, 0), z_dir=back_n
    ) * Box(width, az + 4 * wall, wall)

    # --- the back opening: a port through the back face, standing in for a
    #     circle without the overhang one would print. Flat bottom and sides,
    #     since only the roof of a horizontal hole ever overhangs; the two top
    #     corners are cut at 45deg so the last few millimetres close as a
    #     short bridge instead of a horizontal ceiling. ---
    hw = back_opening_width / 2
    hh = back_opening_height / 2
    ch = back_opening_chamfer
    ox, oz = back_opening_x, back_opening_z
    opening_pts = [
        (ox - hw, oz - hh),
        (ox + hw, oz - hh),
        (ox + hw, oz + hh - ch),
        (ox + hw - ch, oz + hh),
        (ox - hw + ch, oz + hh),
        (ox - hw, oz + hh - ch),
    ]
    opening_profile = Plane.XZ * Polygon(*opening_pts, align=None)
    body -= Pos(0, north_y, 0) * extrude(opening_profile, amount=wall * 2, both=True)

    # --- the ergots: standing on the floor. The underside runs flat at Z=0
    #     instead of tapering to the origin point-first: at low seat_height
    #     that taper is a shallow ramp (overhangs past 45deg once seat_height
    #     drops much below ~14mm at this tilt), where a flat base prints with
    #     no overhang at all. ---
    lip = Plane.YZ * Polygon(
        (0.0, 0.0), (0.0, crest), (seat_y, seat_height), (seat_y, 0.0), align=None
    )
    xs = (
        [0.0]
        if lip_ribs == 1
        else [-rib_span / 2 + i * rib_span / (lip_ribs - 1) for i in range(lip_ribs)]
    )
    for px in xs:
        body += Pos(px, 0, 0) * extrude(lip, amount=rib_width / 2, both=True)

    # --- magnet wells: two bosses on the back face's interior side, near the
    #     top, each holding a magnet flush with the seat plane so it sits
    #     right under the screen_wedge underside once the wedge is seated.
    #     Each boss is a triangular prism standing off the interior wall
    #     face: a 45deg ramp away from the wall, then a 45deg ramp back to
    #     it, so the second ramp's face is parallel to the seat plane at
    #     tilt=45. That triangle is unchanged. The magnet is a coin slid in
    #     like a coin slot rather than pushed into a round bore: a hole bored
    #     straight into the ramp face (perpendicular to it) is round, and
    #     printed lying on its side it overhangs past 45deg at its own top,
    #     same problem the straight well below solves with puit_couche's
    #     teardrop. A slot instead: mouth cut into the exterior back face
    #     itself, magnet_cover below the ridge (north_height, at max_y), then
    #     straight down-and-in at 45deg (-up) until magnet_wall of material
    #     is left before it would break out through the boss's first ramp
    #     (A→B) near its own peak. The slot's own flat side walls get it
    #     there without a round hole to overhang. ---
    well_width = magnet_diameter + 2 * magnet_wall + 0.9
    ramp = well_width / sqrt(2)
    climb = 2 * ramp

    interior_y = ay - wall * back_n.Y
    interior_z = az - wall * back_n.Y * tan(t)
    z_start = interior_z - climb

    magnet_profile = Plane.YZ * Polygon(
        (interior_y, z_start),
        (interior_y - ramp, z_start + ramp),
        (interior_y, z_start + climb),
        align=None,
    )

    # Mouth at the exterior back face, dropped below the ridge; travels -up
    # (z and y both decreasing at 45deg) until it would break out through
    # the first ramp (A→B, A = (interior_y, z_start), B = the peak,
    # (interior_y - ramp, z_start + ramp)) — solved as the line-line
    # intersection between the ray and A→B — minus magnet_wall of material
    # left standing there. The slot's top face ends up parallel to the B→C
    # ramp (both contain X and the 45deg "up" direction), magnet_cover apart
    # from it — but a straight drop in Z only closes *that* gap by
    # magnet_cover/sqrt(2) (it's not normal to the ramp), so the drop itself
    # has to be magnet_cover*sqrt(2) for the skin left there to actually be
    # magnet_cover thick.
    mouth = Vector(0, ay, az - magnet_cover * sqrt(2))
    slot_dir = -up
    a_pt_y, a_pt_z = interior_y, z_start
    ab_y, ab_z = -ramp, ramp  # B - A
    denom = slot_dir.Y * ab_z - slot_dir.Z * ab_y
    to_break = ((a_pt_y - mouth.Y) * ab_z - (a_pt_z - mouth.Z) * ab_y) / denom
    slot_length = to_break - magnet_wall
    # mouth is the slot's own TOP edge (magnet_cover below the ridge), not
    # its centre — Align.MAX on the thickness axis hangs the box below that
    # line instead of straddling it, which ate into the ramp's own top skin.
    slot_cutter = Box(
        magnet_diameter,
        magnet_slot_gap,
        slot_length,
        align=(Align.CENTER, Align.MAX, Align.MIN),
    )

    # Both wells are built by this one function, called once per side, so
    # the boss and the slot's own dimensions can't drift apart between them
    # — the only thing that ever differs is x0.
    def ramp_magnet_well(x0):
        boss = Pos(x0, 0, 0) * extrude(magnet_profile, amount=well_width / 2, both=True)
        slot_solid = Plane(
            origin=(x0, mouth.Y, mouth.Z), x_dir=(1, 0, 0), z_dir=slot_dir
        ) * slot_cutter
        return boss, slot_solid

    magnet_slot_bounds = []
    for sx in (-1, 1):
        boss, slot_solid = ramp_magnet_well(sx * outer_half / 3)
        body += boss
        magnet_slot_bounds.append(slot_solid.bounding_box())
        body -= slot_solid

    # --- straight magnet well: a third magnet, bored through the back face
    #     with a horizontal (lying) axis rather than off the 45deg ramp the
    #     pair above stand on. A round hole printed with its axis lying flat
    #     overhangs past 45deg at its own top, same as any puit_couche bore;
    #     system.py's puit_couche cuts the teardrop roof that replaces it, the
    #     same shape base_pesage's own wall magnet used before it was retired
    #     (see that file's history). puit_couche is a cutter only, so the pad
    #     is a plain box here rather than puit_debout's round one: a box's
    #     side faces are all vertical or upward, and only the boss's own
    #     protrusion past the plain wall (straight_magnet_cover +
    #     straight_magnet_height - wall) is unsupported underneath, short
    #     enough to bridge. The magnet is pushed in from the interior — the
    #     mouth, where puit_couche's own entry chamfer sits — and travels
    #     toward the exterior, stopping straight_magnet_cover short of it,
    #     the thickness the user asked the back face be reduced to here. ---
    straight_magnet_r = straight_magnet_diameter / 2
    straight_magnet_half = straight_magnet_r + MARGE_PUIT
    straight_magnet_x = outer_half - wall - straight_magnet_from_right
    straight_magnet_z = back_opening_z
    if abs(straight_magnet_x) + straight_magnet_half > outer_half:
        reject(
            f"straight_magnet_from_right {straight_magnet_from_right:.1f}mm puts "
            f"the well past the base's {width:.1f}mm width",
            param="straight_magnet_from_right",
        )
    straight_magnet_mouth_y = north_y - straight_magnet_cover - straight_magnet_height
    body += Pos(straight_magnet_x, (straight_magnet_mouth_y + north_y) / 2, straight_magnet_z) * Box(
        2 * straight_magnet_half,
        north_y - straight_magnet_mouth_y,
        2 * straight_magnet_half,
    )
    straight_magnet_plane = Plane(
        origin=(straight_magnet_x, straight_magnet_mouth_y, straight_magnet_z),
        x_dir=(-1, 0, 0),
        z_dir=(0, 1, 0),
    )
    body -= straight_magnet_plane * puit_couche(straight_magnet_r, straight_magnet_height)

    # --- cable-tie rings: three loops against the back face's own interior
    #     side, standing off `interior_y` (the same interior-face Y the
    #     magnet ramps stand off), for a tie to strap cables against the
    #     wall. Same shape as boitier_ps's anti_tirage_ns, called on this
    #     wall the same way as its own north wall (toward_plus_y False, the
    #     ring projecting into the box's interior). Centred on the magnet
    #     wells' own X: two on the right-hand well's column, one lone one on
    #     the left-hand well's column. ---
    cable_tie_kw = dict(
        wall=wall,
        jeu=cable_tie_gap,
        largeur=cable_tie_slot_width,
        bords=cable_tie_span,
        hauteur_u=cable_tie_height,
    )
    for x_center, z_center in (
        (outer_half / 3, cable_tie_low_z),
        (outer_half / 3, cable_tie_right_high_z),
        (-outer_half / 3, cable_tie_left_z),
    ):
        tie_x0 = x_center - cable_tie_span / 2
        tie_z0 = z_center - cable_tie_height / 2
        body += anti_tirage_ns(tie_x0, interior_y, tie_z0, False, **cable_tie_kw)

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

    def on_magnet_well(edge):
        # the boss's own two end faces (the well's flat X-bounds) — a
        # chamfer on either eats into the magnet_wall thickness that's
        # already sized tight around the slot.
        for x0 in (-outer_half / 3, outer_half / 3):
            if on_flat_face(edge, x0 - well_width / 2) or on_flat_face(edge, x0 + well_width / 2):
                return True
        # the straight well's own boss: a plain box sitting flush against
        # the back wall, tangent with it at the boss's own outer frame —
        # chamfering that frame tessellates a sliver at the tangency, same
        # failure mode as the ramped wells' end faces above.
        if on_flat_face(edge, straight_magnet_x - straight_magnet_half) or on_flat_face(
            edge, straight_magnet_x + straight_magnet_half
        ):
            return True
        return False

    def on_magnet_slot(edge):
        # no chamfers anywhere on the coin slot — its mouth, side walls and
        # rim are all cut close together, and a chamfer on any of them
        # either narrows the slot below magnet_slot_gap or nicks the ramp's
        # own thin top skin right next to it.
        b = edge.bounding_box()
        for slot_bb in magnet_slot_bounds:
            if (
                slot_bb.min.X - 0.1 <= b.min.X
                and b.max.X <= slot_bb.max.X + 0.1
                and slot_bb.min.Y - 0.1 <= b.min.Y
                and b.max.Y <= slot_bb.max.Y + 0.1
                and slot_bb.min.Z - 0.1 <= b.min.Z
                and b.max.Z <= slot_bb.max.Z + 0.1
            ):
                return True
        return False

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    keep = keep.filter_by(
        lambda e: not seating(e) and not on_magnet_well(e) and not on_magnet_slot(e)
    )
    return polish(body, keep, 1.0)

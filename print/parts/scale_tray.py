import math

from nurb import *

from system import AMIN, CMIN, scale_circle, scale_frame, scale_layout

# Thickness of the ledges the outer grille lies on.
RAIL_THICKNESS = 2.5

# Structural 45° fill where each rear peg leaves the sloping rear wall.
PEG_ROOT_CHAMFER = 2.5

# Least air between the floor's underside and the highest part of the scale
# under it, and below the mouth's ceiling for the rim: the tray comes in
# tilted and must clear both.
SCALE_CLEARANCE = 1.3
MOUTH_CEILING_CLEARANCE = 2.0
# Least air between the floor's underside and the base plate's front lip,
# which the tray slides over on its way in.
FRONT_LIP_CLEARANCE = 1.0

# Printed floor on the bed, the tray grows upward.
PRINT_UP = Vector(0.0, 0.0, 1.0)


def _block(x0, x1, y0, y1, z0, z1):
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=AMIN)


def _section_prism(section, x0, x1):
    """A section drawn with sketch x as Y and sketch y as Z, extruded along X from x0 to x1."""
    return Pos(x0, 0, 0) * extrude(Plane.YZ * section, x1 - x0, dir=(1, 0, 0))


def _profile_along_y(points, y0, y1):
    """A profile drawn in (X, Z), extruded from Y = y0 to y1."""
    face = Plane.XZ * Polygon(*points, align=None)
    return Pos(0, y0, 0) * extrude(face, y1 - y0, dir=(0, 1, 0))


def _mirrored_pair(left, centre_x):
    """A west-side feature and its twin across the tray's centre plane."""
    return left + mirror(left, Plane.YZ.offset(centre_x))


def _rear_slope_inner_y(tray, z):
    """Y of the rear slope's inner face at height Z: it rises 45° toward the
    rear and reaches the rim at y_rear."""
    return tray.y_rear + (tray.rim_top - z)


def _rear_slope_outer_z(tray, y):
    """Z of the rear slope's outer face, one wall below the inner face measured square to it."""
    return tray.rim_top - (y - tray.y_rear) - tray.wall * math.sqrt(2.0)


def _section(tray):
    """The tub's outer section: above the floor's underside, in front of the
    rear slope's outer face, up past the rim. The cavity is this section
    offset inward by one wall, which is why floor and wall must be equally
    thick."""
    far_rear = tray.y_rear - (tray.y_front - tray.y_rear)
    top = tray.rim_top + 2.0 * tray.wall
    above_floor = Polygon((tray.y_front, tray.underside), (far_rear, tray.underside),
                          (far_rear, top), (tray.y_front, top), align=None)
    slope_low = _rear_slope_outer_z(tray, tray.y_front)
    slope_high = _rear_slope_outer_z(tray, far_rear)
    in_front_of_slope = Polygon((far_rear, slope_high), (tray.y_front, slope_low),
                                (tray.y_front, slope_high), align=None)
    return above_floor & in_front_of_slope


def _wide_x(tray, plate_width):
    """Outer faces of the full-width part's side walls, one side_play inside the plate's walls."""
    return tray.side_play, plate_width - tray.side_play


def _envelope(tray, plate_width):
    """The tub's outer solid, cut square at the rear end and at the rim: the
    mouth's width from the rear end, the plate's full width from wide_y0."""
    section = _section(tray)
    narrow = _section_prism(section, tray.x0, tray.x1) & _block(
        tray.x0, tray.x1, tray.y_rear, tray.y_front, tray.underside, tray.rim_top)
    wide_x0, wide_x1 = _wide_x(tray, plate_width)
    wide = _section_prism(section, wide_x0, wide_x1) & _block(
        wide_x0, wide_x1, tray.wide_y0, tray.y_front, tray.underside, tray.rim_top)
    return narrow + wide


def _wide_cavity_profile(tray, plate_width):
    """(X, Z) outline of the full-width part's inside: its side walls rise
    from the floor, then a 45° shoulder leans 9 mm inward and meets the
    rail's corbel in one line, so the outer grille keeps the narrow tray's
    width. Above the shoulder the part is solid up to the rim."""
    wide_x0, wide_x1 = _wide_x(tray, plate_width)
    inner_x0 = wide_x0 + tray.wall
    rail_x0 = tray.x0 + tray.wall
    shoulder_top = tray.rail_top - RAIL_THICKNESS - tray.rail_width
    shoulder_foot = shoulder_top - (rail_x0 - inner_x0)
    below = tray.underside - 1.0
    return ((inner_x0, below), (inner_x0, shoulder_foot), (rail_x0, shoulder_top),
            (plate_width - rail_x0, shoulder_top), (plate_width - inner_x0, shoulder_foot),
            (plate_width - inner_x0, below))


def _cavity(tray, plate_width):
    """The tub's inside, open above the rim between the rails' walls."""
    inner = offset(_section(tray), -tray.wall, kind=Kind.INTERSECTION)
    narrow = _section_prism(inner, tray.x0 + tray.wall, tray.x1 - tray.wall)
    wide_x0, wide_x1 = _wide_x(tray, plate_width)
    wide = _profile_along_y(_wide_cavity_profile(tray, plate_width), tray.wide_y0 + tray.wall,
                            tray.y_front) & _section_prism(inner, wide_x0, wide_x1)
    return narrow + wide


def _front_wall_cut(tray):
    """Lowers the front wall to the rails, between the side walls."""
    return _block(tray.x0 + tray.wall, tray.x1 - tray.wall, tray.y_front - tray.wall, tray.y_front,
                  tray.rail_top, tray.rim_top)


def _rails(tray, centre_x, envelope):
    """Ledges for the outer grille along the side walls, on a 45° corbel,
    from the front wall back to where the rear slope cuts them."""
    inner_x0 = tray.x0 + tray.wall
    under = tray.rail_top - RAIL_THICKNESS
    side = _profile_along_y(((inner_x0, tray.rail_top), (inner_x0 + tray.rail_width, tray.rail_top),
                             (inner_x0 + tray.rail_width, under), (inner_x0, under - tray.rail_width)),
                            tray.y_rear, tray.y_front - tray.wall)
    return _mirrored_pair(side, centre_x) & envelope


def _shoulders(tray, centre_x):
    """Flanges resting on the base plate's side walls, from the full-width
    part's outer faces to the walls' outer faces."""
    flange = _block(tray.shoulder_x0, tray.side_play, tray.wide_y0, tray.y_front,
                    tray.shoulder_underside, tray.shoulder_underside + tray.shoulder_thickness)
    return _mirrored_pair(flange, centre_x)


def _kerb_radius(tray):
    return tray.window_diameter / 2.0 + tray.kerb_wall


def _window_and_kerb(tray):
    """The round kerb standing on the floor and the window cut through floor and kerb."""
    floor_top = tray.underside + tray.floor
    kerb = Pos(0, 0, floor_top) * extrude(scale_circle(_kerb_radius(tray)), tray.kerb_top - floor_top)
    window = Pos(0, 0, tray.underside) * extrude(
        scale_circle(tray.window_diameter / 2.0), tray.kerb_top - tray.underside)
    return kerb, window


def _peg(tray, x, z, end_y):
    """A rear stop along -Y, from inside the rear slope to end_y."""
    radius = tray.bumper_boss_diameter / 2.0
    # Start on the slope's inner face at the peg's lowest line: the part of the
    # peg that reaches into the tub is cut away with the cavity.
    start_y = _rear_slope_inner_y(tray, z - radius)
    return Pos(x, start_y, z) * (Rot(90, 0, 0) * Cylinder(radius, start_y - end_y, align=CMIN))


def _add_pegs(body, tray, cavity):
    """The two rear stops, solid through the slope, with a structural 45°
    fill where they leave it."""
    west = _peg(tray, *tray.bumper_west, tray.y_rear)
    east = _peg(tray, *tray.bumper, tray.y_rear - tray.bumper_boss_length)
    pegs = (west + east) - cavity
    joined = body + pegs
    return chamfer(new_edges(body, pegs, combined=joined), PEG_ROOT_CHAMFER)


def build_tray(layout):
    """The unpolished tray in the machine frame."""
    tray = layout.tray
    envelope = _envelope(tray, layout.plate_width)
    cavity = _cavity(tray, layout.plate_width)
    body = envelope - cavity - _front_wall_cut(tray)
    body += _rails(tray, layout.centre_x, envelope) + _shoulders(tray, layout.centre_x)
    kerb, window = _window_and_kerb(tray)
    body = _add_pegs(body + kerb, tray, cavity)
    return body - window


def _scale_tops(layout):
    """Top of each part of the scale that stands under the floor, by name."""
    cell, base, box = layout.cell, layout.base, layout.box
    return {
        "the adapter": layout.adapter.top,
        "the base's bridge beam": cell.z1 + base.bridge_gap + base.bridge_thickness,
        "the PCB box lid": box.wall_top + box.lid_plate,
    }


def _refuse_misfit(layout):
    """Refuse a layout that would not go into the machine or would sit on the scale."""
    tray = layout.tray
    if tray.wide_y0 < measured("scale_vis_facade_saillie"):
        reject(f"the full-width part starts at Y {tray.wide_y0}, on the machine face's screw heads "
               f"(out to Y {measured('scale_vis_facade_saillie')})")
    if tray.floor != tray.wall:
        reject(f"floor {tray.floor} and wall {tray.wall} differ: the tub is one section offset "
               "by a single thickness, so set them equal")
    ceiling = measured("scale_bouche_plafond")
    if tray.rim_top > ceiling - MOUTH_CEILING_CLEARANCE:
        reject(f"rim top {tray.rim_top} leaves under {MOUTH_CEILING_CLEARANCE} mm below the mouth's "
               f"ceiling at {ceiling}: lower it to {ceiling - MOUTH_CEILING_CLEARANCE} or less")
    highest, top = max(_scale_tops(layout).items(), key=lambda item: item[1])
    lowest = top + SCALE_CLEARANCE
    if tray.underside < lowest:
        reject(f"floor underside {tray.underside} would rub {highest} (top at "
               f"{top}): raise it to {lowest} or more")
    front_lip = measured("scale_tole_rebord_avant") + FRONT_LIP_CLEARANCE
    if tray.underside < front_lip:
        reject(f"floor underside {tray.underside} would catch the front lip on the way in: "
               f"raise it to {front_lip} or more")
    half_mouth = measured("scale_bouche_largeur") / 2.0
    if tray.x0 < layout.centre_x - half_mouth or tray.x1 > layout.centre_x + half_mouth:
        reject(f"tray body X {tray.x0}..{tray.x1} is wider than the {2 * half_mouth} mm mouth")
    rear_reach = -measured("scale_bouche_profondeur") + measured("scale_taquet_saillie")
    if tray.y_rear < rear_reach:
        reject(f"tray rear at Y {tray.y_rear} runs into the left bumper at Y {rear_reach}")
    _refuse_sleeve_off_floor(layout)


def _refuse_sleeve_off_floor(layout):
    """The chimney's sleeve over the kerb must stand on the flat floor,
    clear of the rear slope and of the side walls."""
    tray = layout.tray
    reach = _kerb_radius(tray) + layout.chimney.wall
    axis_x, axis_y = layout.axis
    flat_rear = _rear_slope_inner_y(tray, tray.underside + tray.floor)
    inner_x0, inner_x1 = tray.x0 + tray.wall, tray.x1 - tray.wall
    if axis_y - reach < flat_rear or axis_x - reach < inner_x0 or axis_x + reach > inner_x1:
        reject(f"the chimney's sleeve (radius {reach} on the axis) leaves the floor's flat "
               f"(X {inner_x0}..{inner_x1}, Y from {flat_rear})")


def _faces_by_edge(solid):
    """Every edge mapped to the faces that share it."""
    faces_by_edge = {}
    for face in solid.faces():
        for edge in face.edges():
            faces_by_edge.setdefault(edge, []).append(face)
    return faces_by_edge


def _chamfer_faces_down_in_print(edge, faces):
    """True if a chamfer on this edge would look more than 45° downward once
    the tray lies on its floor."""
    point = edge.center()
    bisector = (faces[0].normal_at(point) + faces[1].normal_at(point)).normalized()
    return -bisector.dot(PRINT_UP) > math.sin(math.radians(45.0)) + 1e-6


def _acute(edge, faces):
    """True where the two faces meet at under 90° of material, like the rear
    rim, where the slope's inner face reaches the rim at 45°: a chamfer there
    leaves a sliver thinner than the knife it was meant to blunt."""
    point = edge.center()
    return faces[0].normal_at(point).dot(faces[1].normal_at(point)) < -1e-6


def _flat_in(low, high, value):
    return abs(low - value) < 1e-3 and abs(high - value) < 1e-3


def _mating(edge, tray, plate_width, kerb_box):
    """Edges lying in a face another part bears on or slides along: shoulder
    undersides, the outer faces of the side walls that locate the tray between
    the plate's walls, rail and front wall tops, the inner top edges that
    guide the outer grille in, and the kerb and window."""
    box = edge.bounding_box()
    if any(_flat_in(box.min.Z, box.max.Z, z) for z in (tray.shoulder_underside, tray.rail_top)):
        return True
    if any(_flat_in(box.min.X, box.max.X, x) for x in (tray.side_play, plate_width - tray.side_play)):
        return True
    guides_x = (tray.x0 + tray.wall, tray.x1 - tray.wall)
    if _flat_in(box.min.Z, box.max.Z, tray.rim_top) and any(_flat_in(box.min.X, box.max.X, x) for x in guides_x):
        return True
    return (box.min.Z >= tray.underside - 1e-3 and box.max.Z <= tray.kerb_top + 1e-3
            and box.min.X >= kerb_box.min.X - 1e-3 and box.max.X <= kerb_box.max.X + 1e-3
            and box.min.Y >= kerb_box.min.Y - 1e-3 and box.max.Y <= kerb_box.max.Y + 1e-3)


def _polish_edges(body, tray, plate_width):
    """Straight convex edges that no part bears on, that do not lie in the
    floor's underside (the bed), and whose chamfer prints. Curved edges are
    all mating (window, kerb, pegs) or the pegs' structural roots."""
    kerb_box = scale_circle(_kerb_radius(tray)).bounding_box()
    return [edge for edge, faces in _faces_by_edge(body).items()
            if edge.geom_type == GeomType.LINE
            and len(faces) == 2
            and is_convex(edge, faces[0], faces[1])
            and not _flat_in(edge.bounding_box().min.Z, edge.bounding_box().max.Z, tray.underside)
            and not _mating(edge, tray, plate_width, kerb_box)
            and not _acute(edge, faces)
            and not _chamfer_faces_down_in_print(edge, faces)]


@part
def scale_tray(edge_chamfer=1.0, draft=False):
    """
    Drip tray of the scale: one flat floor over the scale, sloping rear, rails for the outer grille.

    edge_chamfer: size of the 45° chamfer on the exposed outer edges
    """
    layout = scale_layout()
    _refuse_misfit(layout)
    body = build_tray(layout)
    if not draft:
        body = polish(body, _polish_edges(body, layout.tray, layout.plate_width), edge_chamfer)
    return scale_frame(body)

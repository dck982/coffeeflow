import math

from nurb import *

from system import AMIN, CMIN, scale_frame, scale_layout, scale_lens

# Thickness of the ledges the outer grille lies on.
RAIL_THICKNESS = 2.5

# The locating corbels under the shoulders: their vertical face is this tall,
# and they run from the front back to this Y only, clear of whatever screw
# heads may stand near the rear of the base plate's side walls.
LOCATING_FACE_HEIGHT = 3.0
LOCATING_REAR_Y = 40.0

# Structural 45° fill where each rear peg leaves the sloping rear wall. The
# east peg's lowest line leaves the slope 2 mm above the plateau's underside,
# which is as much room as the fill gets: 3 mm does not land there.
PEG_ROOT_CHAMFER = 2.5

# Least air between the plateau's underside and the top of the scale (the
# adapter's centring boss), and below the mouth's ceiling for the rim: the
# tray comes in tilted and must clear both.
SCALE_CLEARANCE = 1.3
MOUTH_CEILING_CLEARANCE = 2.0
# The plateau's west edge and the foot of its ramp stay this far from the scale.
SCALE_SIDE_CLEARANCE = 3.0
# The base plate's rear lip runs from Y -2 to 0; two screw heads stand in front
# of it up to Y 4, lower than the lip. The tray's underside clears the lip by 1.
# Over that stretch the underside is the straight rear slope, so its two ends
# bound it.
REAR_LIP_Y = (-2.0, 4.0)
REAR_LIP_CLEARANCE = 1.0

# Printed standing on its front wall, the part grows toward the mouth: up is -Y.
PRINT_UP = Vector(0.0, -1.0, 0.0)


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


def _plateau_underside(tray):
    """(Y, Z) points of the plateau's underside from the front wall rearward:
    flat in the well, a 45° ramp, flat over the scale."""
    ramp_foot = tray.plateau_y + (tray.raised_underside - tray.well_underside)
    return [(tray.y_front, tray.well_underside), (ramp_foot, tray.well_underside),
            (tray.plateau_y, tray.raised_underside)]


def _west_underside(tray):
    """(Y, Z) points of the west zone's underside from the front wall rearward:
    flat in the well, one long slope, flat toward the mouth."""
    return [(tray.y_front, tray.well_underside), (tray.slope_y_front, tray.well_underside),
            (tray.slope_y_rear, tray.mouth_underside)]


def _polyline_z(points, y):
    """Z of a front-to-rear (Y, Z) polyline at Y, flat past its last point."""
    for (y_a, z_a), (y_b, z_b) in zip(points, points[1:]):
        if y_b <= y <= y_a:
            return z_a + (z_b - z_a) * (y_a - y) / (y_a - y_b)
    return points[-1][1] if y < points[-1][0] else points[0][1]


def _zone_section(tray, underside):
    """Section of one floor zone's outer solid: above its underside, in front
    of the rear slope's outer face, up past the rim. The cavity is this
    section offset inward by one wall, which is why floor and wall must be
    equally thick."""
    far_rear = tray.y_rear - (tray.y_front - tray.y_rear)
    top = tray.rim_top + 2.0 * tray.wall
    below_rim = Polygon(*underside, (far_rear, underside[-1][1]), (far_rear, top), (tray.y_front, top),
                        align=None)
    slope_low = _rear_slope_outer_z(tray, tray.y_front)
    slope_high = _rear_slope_outer_z(tray, far_rear)
    in_front_of_slope = Polygon((far_rear, slope_high), (tray.y_front, slope_low),
                                (tray.y_front, slope_high), align=None)
    return below_rim & in_front_of_slope


def _zones(tray):
    """((section, outer x0, outer x1, cavity x0, cavity x1), ...) for the west
    zone and the plateau. The plateau's cavity starts one wall west of
    plateau_x0, so what is left of the west zone's solid under it is the
    divider wall, and the plateau keeps its full width from plateau_x0."""
    divider_x = tray.plateau_x0 - tray.wall
    return ((_zone_section(tray, _west_underside(tray)), tray.x0, tray.plateau_x0,
             tray.x0 + tray.wall, divider_x),
            (_zone_section(tray, _plateau_underside(tray)), tray.plateau_x0, tray.x1,
             divider_x, tray.x1 - tray.wall))


def _envelope(tray):
    """The tub's outer solid, cut square at the rear end and at the rim."""
    outer = Part() + [_section_prism(section, x0, x1) for section, x0, x1, _, _ in _zones(tray)]
    reach = tray.y_front - tray.y_rear
    return outer & _block(tray.x0, tray.x1, tray.y_rear, tray.y_front,
                          tray.well_underside - reach, tray.rim_top)


def _cavity(tray):
    """The tub's inside, open above the rim."""
    return Part() + [_section_prism(offset(section, -tray.wall, kind=Kind.INTERSECTION), x0, x1)
                     for section, _, _, x0, x1 in _zones(tray)]


def _front_wall_cut(tray):
    """Lowers the front wall to the rails, between the side walls. Its outer
    top edge stays square: it ends on the bed face, where a bevel would lay a
    knife edge into the first layer."""
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
    """Flanges resting on the base plate's side walls, each with a locating
    corbel whose vertical face centres the tray between the walls. Both start
    on the front face, the one the tray prints on."""
    flange = _block(tray.shoulder_x0, tray.x0, tray.shoulder_y0, tray.y_front,
                    tray.shoulder_underside, tray.shoulder_underside + tray.shoulder_thickness)
    face_bottom = tray.shoulder_underside - LOCATING_FACE_HEIGHT
    corbel = _profile_along_y(((tray.side_play, tray.shoulder_underside), (tray.x0, tray.shoulder_underside),
                               (tray.x0, face_bottom - (tray.x0 - tray.side_play)),
                               (tray.side_play, face_bottom)), LOCATING_REAR_Y, tray.y_front)
    return _mirrored_pair(flange + corbel, centre_x)


def _kerb_outline(tray, extra=0.0):
    """The kerb's outer outline, grown by `extra` all round."""
    return scale_lens(tray.window_diameter / 2.0 + tray.kerb_wall + extra,
                      tray.window_tip_radius + tray.kerb_wall + extra)


def _window_and_kerb(tray):
    """The kerb standing on the plateau and the window cut through floor and kerb."""
    raised_top = tray.raised_underside + tray.floor
    kerb = Pos(0, 0, raised_top) * extrude(_kerb_outline(tray), tray.kerb_top - raised_top)
    window = Pos(0, 0, tray.raised_underside) * extrude(
        scale_lens(tray.window_diameter / 2.0, tray.window_tip_radius), tray.kerb_top - tray.raised_underside)
    return kerb, window


def _kerb_web(tray, axis_x, cavity):
    """A web that carries the 45 degree ramp up to the top of the kerb's +Y tip.

    Standing on the front wall, the tip's rounded end would start in the air;
    the web gives it something to grow from. It is drawn as a 45 degree wedge
    falling toward the front and kept only where it is inside the tub."""
    tip_y = _kerb_outline(tray).bounding_box().max.Y
    # Start well inside the window, which is cut afterwards: the web then
    # fuses with both flanks of the tip instead of leaving a crevice beside them.
    inside_tip = tip_y - tray.kerb_web_width
    reach = tray.kerb_top - tray.well_underside
    wedge = Polygon((inside_tip, tray.kerb_top), (tip_y, tray.kerb_top),
                    (tip_y + reach, tray.kerb_top - reach), (inside_tip, tray.kerb_top - reach),
                    align=None)
    half = tray.kerb_web_width / 2.0
    return _section_prism(wedge, axis_x - half, axis_x + half) & cavity


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
    envelope = _envelope(tray)
    cavity = _cavity(tray)
    body = envelope - cavity - _front_wall_cut(tray)
    body += _rails(tray, layout.centre_x, envelope) + _shoulders(tray, layout.centre_x)
    kerb, window = _window_and_kerb(tray)
    body = _add_pegs(body + kerb + _kerb_web(tray, layout.axis[0], cavity), tray, cavity)
    return body - window


def _refuse_misfit(layout):
    """Refuse a layout that would not go into the machine or would sit on the scale."""
    tray = layout.tray
    if tray.floor != tray.wall:
        reject(f"floor {tray.floor} and wall {tray.wall} differ: the tub is one section offset "
               "by a single thickness, so set them equal")
    ceiling = measured("scale_bouche_plafond")
    if tray.rim_top > ceiling - MOUTH_CEILING_CLEARANCE:
        reject(f"rim top {tray.rim_top} leaves under {MOUTH_CEILING_CLEARANCE} mm below the mouth's "
               f"ceiling at {ceiling}: lower it to {ceiling - MOUTH_CEILING_CLEARANCE} or less")
    lowest = layout.adapter.top + SCALE_CLEARANCE
    if tray.raised_underside < lowest:
        reject(f"plateau underside {tray.raised_underside} would rub the scale (top at "
               f"{layout.adapter.top}): raise it to {lowest} or more")
    front_lip = measured("scale_tole_rebord_avant")
    if tray.well_underside >= front_lip:
        reject(f"well underside {tray.well_underside} is not below the front lip's top at "
               f"{front_lip}: the tray would not lock; lower it under {front_lip}")
    scale_west = layout.axis[0] - layout.adapter.radius - SCALE_SIDE_CLEARANCE
    if tray.plateau_x0 > scale_west:
        reject(f"plateau starts at X {tray.plateau_x0}, over the scale's west side: "
               f"move it to X {scale_west} or less")
    if tray.plateau_y < layout.base.y1 + SCALE_SIDE_CLEARANCE:
        reject(f"plateau ramp at Y {tray.plateau_y} would pass over the scale on insertion: "
               f"move it to Y {layout.base.y1 + SCALE_SIDE_CLEARANCE} or more")
    half_mouth = measured("scale_bouche_largeur") / 2.0
    if tray.x0 < layout.centre_x - half_mouth or tray.x1 > layout.centre_x + half_mouth:
        reject(f"tray body X {tray.x0}..{tray.x1} is wider than the {2 * half_mouth} mm mouth")
    rear_reach = -measured("scale_bouche_profondeur") + measured("scale_taquet_saillie")
    if tray.y_rear < rear_reach:
        reject(f"tray rear at Y {tray.y_rear} runs into the left bumper at Y {rear_reach}")
    _refuse_rear_lip(tray)


def _refuse_rear_lip(tray):
    """The west zone's underside, the lowest part of the rear, must clear the
    rear lip and the screw heads in front of it."""
    lowest = measured("scale_tole_rebord_arriere") + REAR_LIP_CLEARANCE
    underside = min(max(_rear_slope_outer_z(tray, y), _polyline_z(_west_underside(tray), y))
                    for y in REAR_LIP_Y)
    if underside < lowest:
        reject(f"west floor underside falls to Z {underside:.1f} over the rear lip: keep it at "
               f"{lowest} or more (raise mouth_underside or move slope_y_rear forward)")


def _refuse_kerb_off_flat(layout):
    """The kerb and the chimney's sleeve over it must stand on the flat of the plateau."""
    tray = layout.tray
    sleeve = _kerb_outline(tray, layout.chimney.wall).bounding_box()
    flat_rear = _rear_slope_inner_y(tray, tray.raised_underside + tray.floor)
    if sleeve.max.Y > tray.plateau_y or sleeve.min.Y < flat_rear or sleeve.min.X < tray.plateau_x0:
        reject(f"the chimney's sleeve (X {sleeve.min.X:.1f}.., Y {sleeve.min.Y:.1f}..{sleeve.max.Y:.1f}) "
               f"leaves the plateau's flat (X {tray.plateau_x0}.., Y {flat_rear}..{tray.plateau_y})")


def _faces_by_edge(solid):
    """Every edge mapped to the faces that share it."""
    faces_by_edge = {}
    for face in solid.faces():
        for edge in face.edges():
            faces_by_edge.setdefault(edge, []).append(face)
    return faces_by_edge


def _chamfer_faces_down_in_print(edge, faces):
    """True if a chamfer on this edge would look more than 45° downward once
    the tray stands on its front wall."""
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
    undersides, locating faces, rail and front wall tops, the inner top edges
    of the side walls that guide the outer grille in, and the kerb and window."""
    box = edge.bounding_box()
    if any(_flat_in(box.min.Z, box.max.Z, z) for z in (tray.shoulder_underside, tray.rail_top)):
        return True
    if any(_flat_in(box.min.X, box.max.X, x) for x in (tray.side_play, plate_width - tray.side_play)):
        return True
    guides_x = (tray.x0 + tray.wall, tray.x1 - tray.wall)
    if _flat_in(box.min.Z, box.max.Z, tray.rim_top) and any(_flat_in(box.min.X, box.max.X, x) for x in guides_x):
        return True
    return (box.min.Z >= tray.raised_underside - 1e-3 and box.max.Z <= tray.kerb_top + 1e-3
            and box.min.X >= kerb_box.min.X - 1e-3 and box.max.X <= kerb_box.max.X + 1e-3
            and box.min.Y >= kerb_box.min.Y - 1e-3 and box.max.Y <= kerb_box.max.Y + 1e-3)


def _polish_edges(body, tray, plate_width):
    """Straight convex edges that no part bears on, that do not lie in the
    front face (the bed), and whose chamfer prints standing. Curved edges are
    all mating (window, kerb, pegs) or the pegs' structural roots."""
    kerb_box = _kerb_outline(tray).bounding_box()
    return [edge for edge, faces in _faces_by_edge(body).items()
            if edge.geom_type == GeomType.LINE
            and len(faces) == 2
            and is_convex(edge, faces[0], faces[1])
            and not _flat_in(edge.bounding_box().min.Y, edge.bounding_box().max.Y, tray.y_front)
            and not _mating(edge, tray, plate_width, kerb_box)
            and not _acute(edge, faces)
            and not _chamfer_faces_down_in_print(edge, faces)]


def _stand_on_front_wall(body):
    """From the real (framed) tray to its print pose: the front wall's outer
    face on the bed, the mouth end up."""
    standing = Rot(90, 0, 0) * body
    return Pos(0, 0, -standing.bounding_box().min.Z) * standing


@part
def scale_tray(edge_chamfer=1.0, standing=False, draft=False):
    """
    Drip tray of the scale: plateau over the scale, well at the front, sloping rear, rails for the outer grille.

    edge_chamfer: size of the 45° chamfer on the exposed outer edges
    standing: stands the tray on its front wall, mouth end up, the way it prints
    """
    layout = scale_layout()
    _refuse_misfit(layout)
    _refuse_kerb_off_flat(layout)
    body = build_tray(layout)
    if not draft:
        body = polish(body, _polish_edges(body, layout.tray, layout.plate_width), edge_chamfer)
    body = scale_frame(body)
    return _stand_on_front_wall(body) if standing else body

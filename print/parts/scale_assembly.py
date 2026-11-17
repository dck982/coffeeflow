from nurb import *

from system import AMIN, CMIN, scale_flip, scale_frame, scale_layout

# Sheet thickness of the mouth's walls: only their inner faces matter here.
_SHEET = 2.0


def _block(x0, x1, y0, y1, z0, z1):
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=AMIN)


def _base_plate(layout):
    """The machine's folded base plate: perforated floor, two side walls,
    front and rear lips."""
    sheet = layout.plate_thickness
    width, depth = layout.plate_width, layout.plate_depth
    floor = _block(-sheet, width + sheet, -sheet, depth + sheet, -sheet, 0.0)
    for x, y in layout.holes:
        floor -= Pos(x, y, -sheet) * Cylinder(layout.hole_diameter / 2.0, sheet, align=CMIN)
    wall_height = measured("scale_tole_paroi_hauteur")
    walls = (
        _block(-sheet, 0.0, 0.0, depth, 0.0, wall_height)
        + _block(width, width + sheet, 0.0, depth, 0.0, wall_height)
        + _block(0.0, width, depth, depth + sheet, 0.0, measured("scale_tole_rebord_avant"))
        + _block(0.0, width, -sheet, 0.0, 0.0, measured("scale_tole_rebord_arriere"))
    )
    return floor + walls


def _mouth(layout):
    """The opening in the machine's face behind the base plate: back wall,
    ceiling and two cheeks. The left rubber bumper stands out of the back wall."""
    half = measured("scale_bouche_largeur") / 2.0
    back = -measured("scale_bouche_profondeur")
    ceiling = measured("scale_bouche_plafond")
    x0, x1 = layout.centre_x - half, layout.centre_x + half
    shell = (
        _block(x0 - _SHEET, x1 + _SHEET, back - _SHEET, back, 0.0, ceiling + _SHEET)
        + _block(x0 - _SHEET, x1 + _SHEET, back, 0.0, ceiling, ceiling + _SHEET)
        + _block(x0 - _SHEET, x0, back, 0.0, 0.0, ceiling)
        + _block(x1, x1 + _SHEET, back, 0.0, 0.0, ceiling)
    )
    bumper = Pos(layout.centre_x - measured("scale_taquet_x"), back, measured("scale_taquet_z")) * (
        Rot(-90, 0, 0) * Cylinder(4.0, measured("scale_taquet_saillie"), align=CMIN)
    )
    return shell + bumper


def _load_cell(layout):
    cell = layout.cell
    return _block(cell.x0, cell.x1, cell.y0, cell.y1, cell.z0, cell.z1)


def _pcb(layout):
    """The scale board with its two terminal blocks as one envelope."""
    box = layout.box
    board_top = box.pcb_underside + 1.6
    board = _block(box.pcb_x0, box.pcb_x1, box.pcb_y0, box.pcb_y1, box.pcb_underside, board_top)
    blocks_top = board_top + measured("scale_pcb_bornier_hauteur")
    block_depth = 6.3
    blocks = (
        _block(box.pcb_x0, box.pcb_x0 + block_depth, box.pcb_y0 + 4.0, box.pcb_y1 - 4.0, board_top, blocks_top)
        + _block(box.pcb_x1 - block_depth, box.pcb_x1, box.pcb_y0 + 4.0, box.pcb_y1 - 4.0, board_top, blocks_top)
    )
    return board + blocks


def _bumper_in_place(layout):
    """The cap is modelled standing on its closed end; on the tray it lies
    along Y with that end toward the back of the mouth."""
    tray = layout.tray
    proud = tray.bumper_boss_length + tray.bumper_cap
    cap = Pos(tray.bumper[0], tray.y_rear - proud, tray.bumper[1]) * (Rot(-90, 0, 0) * use("scale_bumper"))
    return scale_frame(cap)


def _dowel_in_place(layout):
    """The dowel is modelled standing tip up at the origin; in place it hangs
    tip down from the tower's bottom, `proud` into the adapter."""
    dowel = layout.dowel
    hanging = Pos(dowel.x, dowel.y, layout.tower.bottom + dowel.in_tower) * (Rot(180, 0, 0) * use("scale_dowel"))
    return scale_frame(hanging)


@assembly
def scale_assembly(explode=0.0, show_machine=True):
    """The drip tray scale on the machine's base plate, as the parts leave
    `scale_frame`: machine to the north (+Y), user to the south.

    explode: lifts each layer of parts apart along Z, in mm per layer; 0 is assembled
    show_machine: draws the base plate, the mouth, the load cell and the PCB
    """
    explode = float(explode)
    if explode < 0.0:
        reject(f"explode {explode} is negative: raise it to 0 or more", param="explode")
    layout = scale_layout()

    def lifted(solid, layer):
        return Pos(0, 0, explode * layer) * solid

    parts = [
        use("scale_base"),
        lifted(scale_flip(use("scale_lid")), 1),
        lifted(scale_flip(use("scale_adapter")), 1),
        lifted(use("scale_tray"), 2),
        lifted(_bumper_in_place(layout), 2),
        lifted(scale_flip(use("scale_chimney")), 3),
        lifted(use("scale_tower"), 4),
        lifted(_dowel_in_place(layout), 4),
        lifted(scale_flip(use("scale_grille_outer")), 4),
        lifted(scale_flip(use("scale_grille_inner")), 5),
    ]
    if show_machine:
        parts += [
            obstacle(scale_frame(_base_plate(layout)), "the base plate"),
            obstacle(scale_frame(_mouth(layout)), "the mouth"),
            obstacle(scale_frame(_load_cell(layout)), "the load cell"),
            obstacle(scale_frame(_pcb(layout)), "the scale PCB"),
        ]
    return tuple(parts)

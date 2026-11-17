from nurb import *

from system import scale_flip, scale_frame, scale_layout, scale_circle


@part
def scale_chimney(grip=0.2):
    """
    Soft round tube that pushes over the tray window's kerb and keeps splashes off the tower.

    grip: how much smaller the tube's bore is than the kerb's outer diameter, so the TPU stretches onto it
    """
    layout = scale_layout()
    tray, chimney = layout.tray, layout.chimney
    bore_radius = tray.window_diameter / 2.0 + tray.kerb_wall - grip / 2.0
    outline = scale_circle(bore_radius + chimney.wall) - scale_circle(bore_radius)
    tube = Pos(0, 0, chimney.bottom) * extrude(outline, amount=chimney.top - chimney.bottom)
    return scale_flip(scale_frame(tube))

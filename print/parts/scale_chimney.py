from nurb import *

from system import scale_flip, scale_frame, scale_layout, scale_lens


def _web_notch(layout, tip_y, play=0.5):
    """The tray has a web under the kerb's +Y tip, as tall as the kerb: the
    tube straddles it through a notch at its tip, from the floor to just above
    the kerb. Above the notch the tube is whole."""
    tray, chimney = layout.tray, layout.chimney
    half = tray.kerb_web_width / 2.0 + play
    depth = tray.kerb_web_width
    height = tray.kerb_top + play - chimney.bottom
    return Pos(layout.axis[0] - half, tip_y - depth, chimney.bottom) * Box(
        2 * half, depth + 1.0, height, align=(Align.MIN, Align.MIN, Align.MIN))


@part
def scale_chimney(grip=0.2):
    """
    Soft tube that pushes over the tray window's lens-shaped kerb and keeps splashes off the tower.

    grip: how much smaller the tube's bore is than the kerb's outer outline, so the TPU stretches onto it
    """
    layout = scale_layout()
    tray, chimney = layout.tray, layout.chimney
    kerb_radius = tray.window_diameter / 2.0 + tray.kerb_wall
    kerb_tip_radius = tray.window_tip_radius + tray.kerb_wall
    bore_radius = kerb_radius - grip / 2.0
    bore_tip_radius = kerb_tip_radius - grip / 2.0
    height = chimney.top - chimney.bottom

    outline = scale_lens(bore_radius + chimney.wall, bore_tip_radius + chimney.wall) - scale_lens(
        bore_radius, bore_tip_radius)
    tube = Pos(0, 0, chimney.bottom) * extrude(outline, amount=height)
    tube -= _web_notch(layout, outline.bounding_box().max.Y)
    return scale_flip(scale_frame(tube))

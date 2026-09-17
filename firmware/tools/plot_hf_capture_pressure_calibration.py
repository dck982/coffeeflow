#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["matplotlib>=3.8"]
# ///
"""Compare une capture HF aux graduations relevées sur le manomètre.

Les graduations proviennent de l'essai du 2026-09-17 avec le panier de
simulation à trou de 0,2 mm.  Elles sont intentionnellement locales à ce
script : ce n'est pas une calibration générique de la machine.

Exemple :
    uv run firmware/tools/plot_hf_capture_pressure_calibration.py \
        captures/260917-143738.json --output captures/pressure-calibration.png
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt

from plot_hf_capture import validate_capture


# Temps depuis SET dimmer>0, lus dans la vidéo du manomètre.  La dernière
# graduation est 50 ms après la dernière capture ; elle est extrapolée
# linéairement à partir des deux derniers échantillons.
MANOMETER_OBSERVATIONS: tuple[tuple[float, float], ...] = (
    (11.1, 1.0), (14.7, 2.0), (16.8, 3.0), (18.4, 4.0),
    (19.7, 5.0), (20.9, 6.0), (22.4, 7.0), (24.2, 8.0), (27.4, 9.0),
)
CURRENT_FULL_SCALE_BAR = 10.0


def interpolate_or_extrapolate(times: list[float], values: list[float], at_s: float) -> float:
    """Interpolation linéaire, avec extrapolation seulement aux deux bords."""
    for index in range(1, len(times)):
        if times[index] >= at_s:
            left, right = index - 1, index
            break
    else:
        left, right = len(times) - 2, len(times) - 1
    if at_s < times[0]:
        left, right = 0, 1
    fraction = (at_s - times[left]) / (times[right] - times[left])
    return values[left] + fraction * (values[right] - values[left])


def least_squares(observed: list[float], gauge: list[float]) -> tuple[float, float, float]:
    """Retourne pente/offset libre et pente imposée à l'origine."""
    count = len(observed)
    mean_x, mean_y = sum(observed) / count, sum(gauge) / count
    slope = sum((x - mean_x) * (y - mean_y) for x, y in zip(observed, gauge)) / sum(
        (x - mean_x) ** 2 for x in observed
    )
    offset = mean_y - slope * mean_x
    origin_slope = sum(x * y for x, y in zip(observed, gauge)) / sum(x * x for x in observed)
    return slope, offset, origin_slope


def plot(capture: dict[str, Any]) -> tuple[plt.Figure, tuple[float, float, float]]:
    samples: list[dict[str, Any]] = capture["samples"]
    if len(samples) < 2:
        raise RuntimeError("la capture doit contenir au moins deux échantillons")
    times = [float(sample["t_ms"]) / 1000.0 for sample in samples]
    firmware_pressure = [float(sample["pressure_bar"]) for sample in samples]
    observation_times = [entry[0] for entry in MANOMETER_OBSERVATIONS]
    gauge_pressure = [entry[1] for entry in MANOMETER_OBSERVATIONS]
    sampled_pressure = [interpolate_or_extrapolate(times, firmware_pressure, at_s) for at_s in observation_times]
    slope, offset, origin_slope = least_squares(sampled_pressure, gauge_pressure)
    full_scale_origin = CURRENT_FULL_SCALE_BAR * origin_slope

    figure, axes = plt.subplots(1, 2, figsize=(14, 5.5), layout="constrained")
    axes[0].plot(times, firmware_pressure, color="tab:red", alpha=0.55, label="firmware actuel (10 bar FS)")
    axes[0].plot(
        times, [pressure * slope + offset for pressure in firmware_pressure], color="tab:blue",
        label=f"calibration affine ({CURRENT_FULL_SCALE_BAR * slope:.2f} bar FS, {offset:+.2f} bar)",
    )
    axes[0].scatter(observation_times, gauge_pressure, color="black", zorder=3, label="manomètre")
    axes[0].set(xlabel="temps depuis SET dimmer>0 (s)", ylabel="pression (bar)", title="Chronologie")
    axes[0].grid(True, alpha=0.25)
    axes[0].legend()

    maximum = max(gauge_pressure) + 0.5
    axes[1].scatter(sampled_pressure, gauge_pressure, color="black", label="graduations")
    line_x = [0.0, max(sampled_pressure) + 0.2]
    axes[1].plot(line_x, [origin_slope * x for x in line_x], color="tab:blue",
                 label=f"échelle seule : y={origin_slope:.3f}x")
    axes[1].plot(line_x, [slope * x + offset for x in line_x], color="tab:orange",
                 label=f"échelle + offset : y={slope:.3f}x {offset:+.3f}")
    axes[1].plot([0, maximum], [0, maximum], color="0.5", linestyle=":", label="calibration parfaite")
    axes[1].set(xlim=(0, maximum), ylim=(0, maximum), xlabel="pression firmware actuelle (bar)",
                ylabel="pression manomètre (bar)", title="Régression aux instants relevés")
    axes[1].grid(True, alpha=0.25)
    axes[1].legend()
    return figure, (slope, offset, origin_slope)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--output", type=Path, help="image de sortie")
    parser.add_argument("--no-show", action="store_true")
    args = parser.parse_args()
    try:
        capture = validate_capture(json.loads(args.capture.read_text(encoding="utf-8")))
        figure, (slope, offset, origin_slope) = plot(capture)
    except (OSError, json.JSONDecodeError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"Échelle seule : kPressureFullScaleBar = {CURRENT_FULL_SCALE_BAR * origin_slope:.3f} bar")
    print(f"Ajustement affine des observations : kPressureFullScaleBar = {CURRENT_FULL_SCALE_BAR * slope:.3f} bar, "
          f"kPressureOffsetBar = {offset:.3f} bar")
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        figure.savefig(args.output, dpi=160)
        print(f"graphe écrit dans {args.output}")
    if not args.no_show:
        plt.show()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

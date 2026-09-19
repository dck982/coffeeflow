#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["matplotlib>=3.8"]
# ///
"""Trace une capture haute fréquence CoffeeFlow enregistrée localement.

Exemple :
    uv run firmware/tools/plot_hf_capture.py captures/260917-143012.json --output capture.png

Le débit de la balance est une dérivée centrée et lissée du poids. Sa valeur
est en g/s (pratiquement ml/s pour de l'eau), afin de le comparer directement
au débitmètre.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt


def validate_capture(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise RuntimeError("le fichier JSON ne contient pas un objet")
    if payload.get("schema") != "coffeeflow.hf_capture.v1" or not isinstance(payload.get("samples"), list):
        raise RuntimeError("capture inconnue ou incompatible")
    return payload


def values(samples: list[dict[str, Any]], key: str) -> list[float]:
    return [float(sample.get(key, 0.0)) for sample in samples]


def weight_flow_g_s(times: list[float], weight_g: list[float], window_s: float) -> list[float]:
    """Dérivée centrée du poids sur une fenêtre temporelle donnée.

    Les bords, où une fenêtre entière n'est pas disponible, sont laissés à
    NaN pour ne pas leur attribuer une pente artificielle.
    """
    if window_s <= 0:
        raise RuntimeError("la fenêtre de débit balance doit être strictement positive")
    half_window_s = window_s / 2.0
    flow = [float("nan")] * len(times)
    left = 0
    right = 0
    for index, time_s in enumerate(times):
        while left < len(times) and times[left] < time_s - half_window_s:
            left += 1
        while right < len(times) and times[right] <= time_s + half_window_s:
            right += 1
        right_index = right - 1
        if left < index < right_index:
            elapsed_s = times[right_index] - times[left]
            if elapsed_s > 0:
                flow[index] = (weight_g[right_index] - weight_g[left]) / elapsed_s
    return flow


def derivative(values_: list[float], times: list[float], sample_distance: int = 1) -> list[float]:
    """Dérivée arrière sur plusieurs samples, en tenant compte du temps réel."""
    if sample_distance <= 0:
        raise RuntimeError("la distance de dérivation du débit doit être strictement positive")
    result = [float("nan")] * len(values_)
    for index in range(sample_distance, len(values_)):
        delta = values_[index] - values_[index - sample_distance]
        result[index] = delta
    return result


def plot(capture: dict[str, Any], weight_flow_window_s: float = 2.0,
         flow_derivative_samples: int = 0) -> plt.Figure:
    samples: list[dict[str, Any]] = capture["samples"]
    if not samples:
        raise RuntimeError("la capture ne contient aucun échantillon")
    elapsed_s = [float(sample["t_ms"]) / 1000.0 for sample in samples]
    pressure, temperature = values(samples, "pressure_bar"), values(samples, "temperature_c")
    flow, volume, weight = values(samples, "flow_ml_s"), values(samples, "volume_ml"), values(samples, "weight_g")
    balance_flow = weight_flow_g_s(elapsed_s, weight, weight_flow_window_s)
    if flow_derivative_samples>0:
        flow_derivative = derivative(flow, elapsed_s, flow_derivative_samples)
    else:
        flow_derivative = None
    commanded, reported = values(samples, "pump_pct_commanded"), values(samples, "pump_pct_reported")

    figure, axes = plt.subplots(4, 1, figsize=(13, 10), sharex=True, layout="constrained")
    duration_s = (int(capture["ended_at_us"]) - int(capture["started_at_us"])) / 1_000_000.0
    figure.suptitle(f"CoffeeFlow · {capture['origin']} · {duration_s:.1f} s · {capture['sample_count']} échantillons")

    axes[0].plot(elapsed_s, pressure, color="tab:red", label="pression")
    axes[0].set_ylabel("bar")
    pump_axis = axes[0].twinx()
    pump_axis.step(elapsed_s, commanded, where="post", color="tab:blue", linestyle="--", label="pompe demandée")
    pump_axis.step(elapsed_s, reported, where="post", color="tab:blue", label="pompe rapportée")
    pump_axis.set_ylabel("pompe (%)")
    axes[0].legend(loc="upper left")
    pump_axis.legend(loc="upper right")

    axes[1].plot(elapsed_s, flow, color="tab:green", label="débitmètre (ml/s)")
    axes[1].plot(elapsed_s, balance_flow, color="tab:purple", label=f"balance ({weight_flow_window_s:g} s, g/s)")
    axes[1].set_ylabel("débit (ml/s ou g/s)")
    if flow_derivative is not None:
        flow_derivative_axis = axes[1].twinx()
        flow_derivative_axis.plot(
            elapsed_s,
            flow_derivative,
            color="tab:brown",
            label=f"dérivée du débitmètre ({flow_derivative_samples} sample(s), ml/s²)",
        )
        flow_derivative_axis.set_ylabel("variation du débit (ml/s²)")
        axes[1].legend(loc="upper left")
        flow_derivative_axis.legend(loc="upper right")

    axes[2].plot(elapsed_s, temperature, color="tab:orange")
    axes[2].set_ylabel("température (°C)")
    axes[3].plot(elapsed_s, weight, color="tab:purple", label="poids")
    axes[3].set_ylabel("poids (g)")
    volume_axis = axes[3].twinx()
    volume_axis.plot(elapsed_s, volume, color="tab:olive", label="volume depuis début")
    volume_axis.set_ylabel("volume (ml)")
    axes[3].set_xlabel("temps depuis SET dimmer>0 (s)")
    axes[3].legend(loc="upper left")
    volume_axis.legend(loc="upper right")
    for axis in axes:
        axis.grid(True, alpha=0.25)
    return figure


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="fichier JSON produit par download_hf_capture.py")
    parser.add_argument("--output", type=Path, help="fichier image de sortie (PNG, PDF, SVG…)")
    parser.add_argument("--weight-flow-window-s", type=float, default=2.0,
                        help="fenêtre centrée de dérivation du poids, en secondes (défaut : 2)")
    parser.add_argument("--flow-derivative", type=int, default=0,
                        help="distance en samples pour la dérivée du débitmètre (défaut : 0)")
    parser.add_argument("--no-show", action="store_true", help="ne pas ouvrir la fenêtre matplotlib")
    args = parser.parse_args()

    try:
        capture = validate_capture(json.loads(args.capture.read_text(encoding="utf-8")))
        figure = plot(capture, args.weight_flow_window_s, args.flow_derivative)
    except (OSError, json.JSONDecodeError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        figure.savefig(args.output, dpi=160)
        print(f"graphe écrit dans {args.output}")
    if not args.no_show:
        plt.show()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

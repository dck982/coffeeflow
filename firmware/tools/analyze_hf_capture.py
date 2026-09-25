#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Calcule la température NTC moyenne pondérée par le volume d'une capture HF.

Exemple : uv run firmware/tools/analyze_hf_capture.py captures/260925-152754.json
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any


def finite_number(value: Any, field: str, index: int) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f"échantillon {index} : {field} absent ou invalide")
    return float(value)


def pump_stop_index(samples: list[dict[str, Any]]) -> int | None:
    pump_was_running = False
    for index, sample in enumerate(samples):
        pump = finite_number(sample.get("pump_pct_reported"), "pump_pct_reported", index)
        if pump_was_running and pump == 0:
            return index
        pump_was_running = pump > 0
    return None


def volume_weighted_temperature(samples: list[dict[str, Any]], end_index: int) -> tuple[float, float]:
    volume_ml = 0.0
    temperature_volume = 0.0
    for index in range(1, end_index + 1):
        before, after = samples[index - 1], samples[index]
        previous_volume = finite_number(before.get("volume_ml"), "volume_ml", index - 1)
        current_volume = finite_number(after.get("volume_ml"), "volume_ml", index)
        delta_volume = current_volume - previous_volume
        if delta_volume < -1e-6:
            raise ValueError(f"échantillon {index} : volume_ml diminue")
        if delta_volume <= 0:
            continue
        if before.get("boiler_temperature_valid") is not True or after.get("boiler_temperature_valid") is not True:
            raise ValueError(f"échantillon {index} : température NTC invalide pendant un écoulement")
        previous_temperature = finite_number(before.get("boiler_temperature_c"), "boiler_temperature_c", index - 1)
        current_temperature = finite_number(after.get("boiler_temperature_c"), "boiler_temperature_c", index)
        temperature_volume += delta_volume * (previous_temperature + current_temperature) / 2
        volume_ml += delta_volume
    if volume_ml <= 0:
        raise ValueError("aucun volume positif dans la capture")
    return volume_ml, temperature_volume / volume_ml


def analyze(capture: dict[str, Any]) -> tuple[tuple[float, float, float] | None, tuple[float, float]]:
    if capture.get("schema") != "coffeeflow.hf_capture.v2":
        raise ValueError("capture HF v2 requise")
    samples = capture.get("samples")
    if not isinstance(samples, list) or len(samples) < 2 or not all(isinstance(sample, dict) for sample in samples):
        raise ValueError("la capture doit contenir au moins deux échantillons")
    stop = pump_stop_index(samples)
    during_pump = None
    if stop is not None:
        volume_ml, average_c = volume_weighted_temperature(samples, stop)
        stop_s = finite_number(samples[stop].get("t_ms"), "t_ms", stop) / 1000
        during_pump = stop_s, volume_ml, average_c
    full_capture = volume_weighted_temperature(samples, len(samples) - 1)
    return during_pump, full_capture


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="capture JSON téléchargée par download_hf_capture.py")
    args = parser.parse_args()
    try:
        capture = json.loads(args.capture.read_text(encoding="utf-8"))
        if not isinstance(capture, dict):
            raise ValueError("objet JSON attendu")
        during_pump, (full_volume_ml, full_average_c) = analyze(capture)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"erreur : {error}", file=sys.stderr)
        return 1

    print(f"Capture : {args.capture}")
    if during_pump is not None:
        stop_s, volume_ml, average_c = during_pump
        print(f"Jusqu'à l'arrêt de la pompe ({stop_s:.2f} s) : {average_c:.2f} °C sur {volume_ml:.2f} ml")
    else:
        print("Arrêt de la pompe absent : moyenne pendant toute la capture seulement")
    print(f"Capture entière : {full_average_c:.2f} °C sur {full_volume_ml:.2f} ml")
    if capture.get("dropped_samples", 0):
        print(f"Attention : {capture['dropped_samples']} échantillon(s) perdu(s) ; interpolation sur les intervalles manquants")
    print("Méthode : somme[Δvolume × moyenne des deux températures NTC] / somme[Δvolume]")
    print("La NTC est dans la chaudière et le débitmètre en amont ; ce n'est pas une mesure directe à la sortie.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

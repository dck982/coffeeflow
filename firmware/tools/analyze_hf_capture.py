#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Calcule la température NTC moyenne d'une capture HF, pondérée par la tasse et par le volume.

La moyenne pondérée par la tasse est le critère de la consigne
(docs/chauffe-infusion.md#objectif) : chaque gramme arrivé en tasse avant
l'arrêt de la pompe compte avec la NTC du même instant.

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


SCALE_VALID = 0x04


def cup_weighted_temperature(samples: list[dict[str, Any]], end_index: int) -> tuple[float, float] | None:
    """Moyenne NTC pondérée par les grammes arrivés en tasse jusqu'à ``end_index``.

    Le poids est pris en maximum courant : la balance oscille de quelques
    dixièmes de gramme et une baisse n'est pas de l'eau qui remonte. Les
    poids invalides ou négatifs sont ignorés. ``None`` sans balance.
    """
    cup_g = 0.0
    temperature_cup = 0.0
    top_g: float | None = None
    top_temperature = 0.0
    for index in range(end_index + 1):
        sample = samples[index]
        weight = sample.get("weight_g")
        if not (int(sample.get("flags", 0)) & SCALE_VALID) or isinstance(weight, bool) \
                or not isinstance(weight, (int, float)) or not math.isfinite(weight) or weight < 0:
            continue
        if sample.get("boiler_temperature_valid") is not True:
            continue
        temperature = finite_number(sample.get("boiler_temperature_c"), "boiler_temperature_c", index)
        if top_g is None:
            top_g, top_temperature = float(weight), temperature
            continue
        if weight > top_g:
            delta = weight - top_g
            temperature_cup += delta * (top_temperature + temperature) / 2
            cup_g += delta
            top_g = float(weight)
        top_temperature = temperature
    if cup_g <= 0:
        return None
    return cup_g, temperature_cup / cup_g


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
        cup = cup_weighted_temperature(capture["samples"], pump_stop_index(capture["samples"]))
        if cup is not None:
            cup_g, cup_c = cup
            print(f"Pondérée par la tasse, jusqu'à l'arrêt de la pompe : {cup_c:.2f} °C sur {cup_g:.1f} g")
        else:
            print("Pondérée par la tasse : pas de balance dans la capture")
        print(f"Pondérée par le volume, jusqu'à l'arrêt de la pompe ({stop_s:.2f} s) : {average_c:.2f} °C sur {volume_ml:.2f} ml")
    else:
        print("Arrêt de la pompe absent : moyenne pendant toute la capture seulement")
    print(f"Pondérée par le volume, capture entière : {full_average_c:.2f} °C sur {full_volume_ml:.2f} ml")
    if capture.get("dropped_samples", 0):
        print(f"Attention : {capture['dropped_samples']} échantillon(s) perdu(s) ; interpolation sur les intervalles manquants")
    print("Méthode : somme[Δ × moyenne des deux températures NTC] / somme[Δ], Δ en grammes (maximum courant) ou en ml")
    print("La NTC est dans la chaudière et le débitmètre en amont ; ce n'est pas une mesure directe à la sortie.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

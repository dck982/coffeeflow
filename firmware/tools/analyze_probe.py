#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Résume une ou plusieurs captures de record_probe.py.

Exemple :
    uv run firmware/tools/analyze_probe.py captures/probe-….json --reference-c 24.6
    uv run firmware/tools/analyze_probe.py captures/probe-….json --start 30 --end 90

Les échantillons qui relisent la même conversion ADS1115 (même instant de
mesure, déduit de `age_ms`) sont écartés. Pour chaque capture : moyenne,
écart-type, min et max des codes, de la résistance et de la température dans
la fenêtre choisie, dérive par régression linéaire, puis un tableau par
tranches pour repérer les plateaux. La température principale est calculée
sur le rapport moyen ; R = R_fixe × (a0/a1 − 1) (voir
docs/ntc_ads1115_calibration.md).
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from statistics import fmean, stdev
from typing import Any, Callable, Sequence

# Constantes de calibration_machine.h (écran 0.3.34).
R_FIXED_OHM = 4676.0
NTC_R0_OHM = 47000.0
NTC_BETA_K = 3930.0
NTC_T0_K = 298.15
# Callendar–Van Dusen IEC 60751, PT1000.
PT_R0_OHM = 1000.0
PT_A = 3.9083e-3
PT_B = -5.775e-7
# Deux lectures dont l'instant de mesure diffère de moins que ceci sont la même
# conversion (période d'acquisition 100 ms).
SAME_CONVERSION_S = 0.03


@dataclass(frozen=True)
class Sample:
    t_s: float  # instant de la conversion, depuis le début de la capture
    a0: int
    a1: int


def ntc_temperature_c(resistance: float, r0: float = NTC_R0_OHM, beta: float = NTC_BETA_K) -> float:
    return 1.0 / (1.0 / NTC_T0_K + math.log(resistance / r0) / beta) - 273.15


def ntc_resistance(temperature_c: float, r0: float = NTC_R0_OHM, beta: float = NTC_BETA_K) -> float:
    return r0 * math.exp(beta * (1.0 / (temperature_c + 273.15) - 1.0 / NTC_T0_K))


def pt1000_temperature_c(resistance: float) -> float:
    # Inverse exacte pour T ≥ 0 °C ; l'écart sous 0 °C reste < 0,001 °C vers −1 °C.
    ratio = resistance / PT_R0_OHM
    return (-PT_A + math.sqrt(PT_A * PT_A - 4.0 * PT_B * (1.0 - ratio))) / (2.0 * PT_B)


def pt1000_resistance(temperature_c: float) -> float:
    return PT_R0_OHM * (1.0 + PT_A * temperature_c + PT_B * temperature_c * temperature_c)


def load_samples(capture: dict[str, Any]) -> tuple[list[Sample], int, int]:
    """Renvoie les conversions distinctes, le nombre de doublons et d'invalides."""
    samples: list[Sample] = []
    duplicates = invalid = 0
    for raw in capture.get("samples", []):
        a0, a1 = raw.get("a0_raw"), raw.get("a1_raw")
        if not isinstance(a0, int) or not isinstance(a1, int) or a1 <= 0 or a1 >= a0:
            invalid += 1
            continue
        age = raw.get("age_ms")
        t = float(raw["elapsed_s"]) - (age / 1000.0 if isinstance(age, (int, float)) else 0.0)
        if samples and abs(t - samples[-1].t_s) < SAME_CONVERSION_S and \
                (a0, a1) == (samples[-1].a0, samples[-1].a1):
            duplicates += 1
            continue
        samples.append(Sample(t, a0, a1))
    return samples, duplicates, invalid


def slope_per_min(times: Sequence[float], values: Sequence[float]) -> float:
    if len(times) < 2:
        return math.nan
    mean_t, mean_v = fmean(times), fmean(values)
    den = sum((t - mean_t) ** 2 for t in times)
    if den == 0:
        return math.nan
    return 60.0 * sum((t - mean_t) * (v - mean_v) for t, v in zip(times, values)) / den


def second_means(samples: Sequence[Sample], values: Sequence[float]) -> list[float]:
    bins: dict[int, list[float]] = {}
    for sample, value in zip(samples, values):
        bins.setdefault(math.floor(sample.t_s), []).append(value)
    return [fmean(v) for v in bins.values()]


def spread(values: Sequence[float]) -> float:
    return stdev(values) if len(values) > 1 else math.nan


def stat_row(label: str, values: Sequence[float], fmt: str) -> str:
    cells = [fmean(values), spread(values), min(values), max(values)]
    return f"  {label:<12}" + "".join(f"{format(v, fmt):>12}" for v in cells)


def analyze(path: Path, capture: dict[str, Any], *, probe: str, r_fixed: float,
            r0: float, beta: float, start: float | None, end: float | None,
            reference_c: float | None, bin_s: float) -> str:
    to_c: Callable[[float], float]
    if probe == "ntc":
        to_c = lambda r: ntc_temperature_c(r, r0, beta)
    else:
        to_c = pt1000_temperature_c
    samples, duplicates, invalid = load_samples(capture)
    lines = [f"{path.name}  ({capture.get('started_at_utc', '?')}, arrêt : "
             f"{capture.get('stop_reason', '?')})"]
    window = [s for s in samples
              if (start is None or s.t_s >= start) and (end is None or s.t_s <= end)]
    if not window:
        lines.append("  aucune conversion valide dans la fenêtre")
        return "\n".join(lines)
    lines.append(f"  {len(window)} conversions de {window[0].t_s:.1f} à {window[-1].t_s:.1f} s"
                 f" ({duplicates} doublons, {invalid} invalides écartés sur la capture)")
    ratios = [s.a0 / s.a1 - 1.0 for s in window]
    resistances = [r_fixed * q for q in ratios]
    temps = [to_c(r) for r in resistances]
    lines.append(f"  {'':<12}{'moyenne':>12}{'écart-type':>12}{'min':>12}{'max':>12}")
    lines.append(stat_row("a0", [s.a0 for s in window], ".1f"))
    lines.append(stat_row("a1", [s.a1 for s in window], ".1f"))
    lines.append(stat_row("a1/a0", [s.a1 / s.a0 for s in window], ".6f"))
    lines.append(stat_row("R (Ω)", resistances, ".1f"))
    lines.append(stat_row("T (°C)", temps, ".3f"))
    mean_r = r_fixed * fmean(ratios)
    mean_t = to_c(mean_r)
    lines.append(f"  T du rapport moyen : {mean_t:.3f} °C ; écart-type des moyennes 1 s : "
                 f"{spread(second_means(window, temps)):.3f} °C ; dérive : "
                 f"{slope_per_min([s.t_s for s in window], temps):+.3f} °C/min")
    if reference_c is not None:
        lines.append(f"  référence {reference_c:.2f} °C : écart {mean_t - reference_c:+.3f} °C")
        if probe == "ntc":
            expected = ntc_resistance(reference_c, r0, beta)
            lines.append(f"    R mesurée {mean_r:.1f} Ω, courbe {expected:.1f} Ω "
                         f"({100.0 * (mean_r / expected - 1.0):+.2f} %)")
        else:
            implied = pt1000_resistance(reference_c) / fmean(ratios)
            lines.append(f"    R_fixe impliquée : {implied:.1f} Ω")
    if bin_s > 0:
        lines.append(f"  tranches de {bin_s:g} s :")
        lines.append(f"  {'début (s)':>10}{'n':>5}{'a1':>10}{'T (°C)':>10}{'σ (°C)':>9}")
        groups: dict[int, list[int]] = {}
        for index, sample in enumerate(window):
            groups.setdefault(math.floor(sample.t_s / bin_s), []).append(index)
        for key, indices in groups.items():
            group_temps = [temps[i] for i in indices]
            group_t = to_c(r_fixed * fmean(ratios[i] for i in indices))
            lines.append(f"  {key * bin_s:>10.0f}{len(indices):>5}"
                         f"{fmean(window[i].a1 for i in indices):>10.1f}"
                         f"{group_t:>10.3f}{spread(group_temps):>9.3f}")
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("captures", nargs="+", type=Path)
    parser.add_argument("--probe", choices=("ntc", "pt1000"), default="ntc")
    parser.add_argument("--r-fixed", type=float, default=R_FIXED_OHM, metavar="OHM")
    parser.add_argument("--r0", type=float, default=NTC_R0_OHM, metavar="OHM",
                        help="NTC : résistance à 25 °C")
    parser.add_argument("--beta", type=float, default=NTC_BETA_K, metavar="K")
    parser.add_argument("--start", type=float, metavar="S", help="début de fenêtre (s)")
    parser.add_argument("--end", type=float, metavar="S", help="fin de fenêtre (s)")
    parser.add_argument("--reference-c", type=float, metavar="C",
                        help="température de référence du bain")
    parser.add_argument("--bin", type=float, default=10.0, metavar="S",
                        help="largeur des tranches (0 : aucune)")
    args = parser.parse_args(argv)
    status = 0
    for index, path in enumerate(args.captures):
        try:
            capture = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"{path} : {error}", file=sys.stderr)
            status = 1
            continue
        if index:
            print()
        print(analyze(path, capture, probe=args.probe, r_fixed=args.r_fixed, r0=args.r0,
                      beta=args.beta, start=args.start, end=args.end,
                      reference_c=args.reference_c, bin_s=args.bin))
    return status


if __name__ == "__main__":
    raise SystemExit(main())

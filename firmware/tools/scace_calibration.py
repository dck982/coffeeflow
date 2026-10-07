#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Calcule les constantes de calibration de la sonde SCACE à partir d'un palier.

Exemples :
    uv run firmware/tools/scace_calibration.py ice --capture captures/scace-….csv --start 200 --end 320
    uv run firmware/tools/scace_calibration.py boil --capture captures/scace-….csv --start 60 --end 180 \\
        --celsius 98.72 --fixed-ohm 999.3
    uv run firmware/tools/scace_calibration.py convert --ratio 32.672 --fixed-ohm 999.3

Le palier vient d'une capture de scace_ble_log.py, entre --start et --end
(secondes depuis la première trame), ou d'un rapport R_ntc / R_fixe donné
avec --ratio.

- ice : point de glace, 0 °C. Donne kFixedOhm, qui absorbe la R fixe réelle
  et la tolérance de R25.
- boil : point d'ébullition, à la température calculée depuis la pression
  atmosphérique. Avec le kFixedOhm de la glace, donne kBetaShiftK.
- convert : température lue par le firmware pour un rapport et des constantes.

La conversion reprend celle de firmware/scace/main/main.cpp : Steinhart–Hart
ajustée sur la table R/T 8016, puis décalage de B référencé à 0 °C.
"""

from __future__ import annotations

import argparse
import csv
import math
import statistics
import sys
from pathlib import Path

from scace_ble_log import slope_per_min


# Repris de firmware/scace/main/main.cpp ; un test vérifie qu'ils n'ont pas
# divergé.
SH_A = 1.12476949e-03
SH_B = 2.34824463e-04
SH_C = 8.50854659e-08
KELVIN_AT_0C = 273.15
# Au-delà, le palier n'en est pas un.
MAX_DRIFT_K_PER_MIN = 0.01


def sh_inv_kelvin(ln_r: float) -> float:
    return SH_A + SH_B * ln_r + SH_C * ln_r ** 3


def celsius_from_ohm(ohm: float, beta_shift_k: float = 0.0) -> float:
    ln_r = math.log(ohm)
    inv_t = sh_inv_kelvin(ln_r)
    for _ in range(3):
        inv_t = sh_inv_kelvin(ln_r - beta_shift_k * (inv_t - 1 / KELVIN_AT_0C))
    return 1 / inv_t - KELVIN_AT_0C


def table_ohm(celsius: float) -> float:
    """R de la courbe nominale (sans décalage de B), par Newton sur ln R."""
    target = 1 / (celsius + KELVIN_AT_0C)
    ln_r = math.log(10000.0)
    for _ in range(20):
        step = (sh_inv_kelvin(ln_r) - target) / (SH_B + 3 * SH_C * ln_r ** 2)
        ln_r -= step
        if abs(step) < 1e-12:
            break
    return math.exp(ln_r)


def fixed_ohm_from_ice(ratio: float) -> float:
    return table_ohm(0.0) / ratio


def beta_shift_from_boil(ratio: float, fixed_ohm: float, celsius: float) -> float:
    inv_t = 1 / (celsius + KELVIN_AT_0C)
    return (math.log(fixed_ohm * ratio) - math.log(table_ohm(celsius))) / (inv_t - 1 / KELVIN_AT_0C)


def plateau(path: Path, start_s: float, end_s: float) -> dict[str, float]:
    with path.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise ValueError(f"capture vide : {path}")
    t0 = int(rows[0]["unix_ms"]) / 1000
    points = []
    for row in rows:
        t = int(row["unix_ms"]) / 1000 - t0
        a0, a1 = int(row["a0"]), int(row["a1"])
        if row["status"] == "ok" and start_s <= t <= end_s and a0 > a1:
            points.append((t, a1 / (a0 - a1), float(row["celsius"])))
    if len(points) < 2:
        raise ValueError(f"moins de deux mesures valides entre {start_s} et {end_s} s")
    ratios = [r for _, r, _ in points]
    return {
        "n": len(points),
        "span_s": points[-1][0] - points[0][0],
        "ratio": statistics.fmean(ratios),
        "ratio_sem": statistics.stdev(ratios) / math.sqrt(len(ratios)),
        "celsius_sd": statistics.stdev([c for _, _, c in points]),
        "drift_k_per_min": slope_per_min([(t, c) for t, _, c in points]),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="mode", required=True)
    for name in ("ice", "boil", "convert"):
        p = sub.add_parser(name)
        p.add_argument("--ratio", type=float, help="rapport R_ntc / R_fixe moyen")
        p.add_argument("--capture", type=Path, help="CSV de scace_ble_log.py")
        p.add_argument("--start", type=float, default=0.0, metavar="SECONDS")
        p.add_argument("--end", type=float, default=math.inf, metavar="SECONDS")
        if name != "ice":
            p.add_argument("--fixed-ohm", type=float, required=True, help="kFixedOhm (du point de glace)")
        if name == "boil":
            p.add_argument("--celsius", type=float, required=True, help="température d'ébullition du jour")
        if name == "convert":
            p.add_argument("--beta-shift", type=float, default=0.0, help="kBetaShiftK")
    args = parser.parse_args()
    if (args.ratio is None) == (args.capture is None):
        parser.error("donner --ratio ou --capture, pas les deux")

    ratio = args.ratio
    if args.capture is not None:
        try:
            stats = plateau(args.capture, args.start, args.end)
        except (OSError, ValueError, KeyError) as error:
            print(f"erreur : {error}", file=sys.stderr)
            return 1
        ratio = stats["ratio"]
        print(f"palier : {stats['n']} mesures sur {stats['span_s']:.1f} s, R/R_fixe {ratio:.5f}"
              f" ± {stats['ratio_sem']:.5f}, σ {stats['celsius_sd']:.3f} K,"
              f" dérive {stats['drift_k_per_min']:+.4f} K/min")
        if abs(stats["drift_k_per_min"]) > MAX_DRIFT_K_PER_MIN:
            print(f"attention : dérive au-delà de {MAX_DRIFT_K_PER_MIN} K/min, ce n'est pas un palier")
    if ratio is None or not math.isfinite(ratio) or ratio <= 0:
        parser.error("rapport invalide")

    if args.mode == "ice":
        fixed = fixed_ohm_from_ice(ratio)
        print(f"kFixedOhm = {fixed:.1f}  (R table à 0 °C : {table_ohm(0.0):.0f} Ω)")
    elif args.mode == "boil":
        shift = beta_shift_from_boil(ratio, args.fixed_ohm, args.celsius)
        print(f"kBetaShiftK = {shift:+.1f}  (lu sans correction : {celsius_from_ohm(args.fixed_ohm * ratio):.3f} °C,"
              f" attendu {args.celsius:.3f} °C ; tolérance de B : ±12 K)")
    else:
        print(f"{celsius_from_ohm(args.fixed_ohm * ratio, args.beta_shift):.3f} °C")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

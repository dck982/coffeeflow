#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Compare tachymètre, Digmesa et balance sur une capture HF de purge.

Calculs, sur la marche stable (consigne au-dessus de 0, après le démarrage) :

- vitesse médiane et fronts parasites du tachymètre : un échantillon bien
  au-dessus de la médiane vient d'un front de trop ; sa position dans le
  demi-tour se déduit de la vitesse lue ;
- débits par régression linéaire : refoulé compté (impulsions du
  tachymètre), refoulé à la vitesse médiane, Digmesa, balance ;
- totaux, K du Digmesa (impulsions par litre) déduit de la balance ;
- arrêt : temps jusqu'à 0 tr/min et pression avant et après.

Exemple : uv run firmware/analysis/pump_capture.py captures/261113-084742.json
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# Les mêmes que PUMP_ML_PER_TURN et PUMP_TACH_PULSES_PER_TURN de screen-core.
PUMP_ML_PER_TURN = 0.1
TACH_PULSES_PER_TURN = 2
# Une vitesse lue au-dessus de la médiane de plus de ce rapport vient d'un
# front parasite.
OUTLIER_RATIO = 1.15


@dataclass
class Slope:
    per_s: float
    points: int


@dataclass
class Report:
    run_start_s: float
    stop_s: float | None
    window: tuple[float, float]
    rpm_median: float
    outliers: list[tuple[float, int, float]]
    displaced_ml_s: Slope
    rpm_ml_s: float
    flow_pulses_s: Slope
    weight_g_s: Slope | None
    displaced_total_ml: float
    pulses_total: int
    weight_final_g: float | None
    first_pulse_s: float | None
    first_weight_s: float | None
    rundown_s: float | None
    displaced_after_stop_ml: float | None
    pressure_before_bar: float | None
    pressure_after_bar: float | None


def slope(points: list[tuple[float, float]]) -> Slope:
    """Pente des moindres carrés, en unités par seconde."""
    if len(points) < 2:
        raise RuntimeError("fenêtre trop courte pour une pente")
    mean_t = statistics.fmean(t for t, _ in points)
    mean_v = statistics.fmean(v for _, v in points)
    den = sum((t - mean_t) ** 2 for t, _ in points)
    num = sum((t - mean_t) * (v - mean_v) for t, v in points)
    return Slope(num / den, len(points))


def weight_valid(sample: dict[str, Any]) -> bool:
    return bool(sample.get("flags", 0) & 0x04) and sample.get("weight_g") is not None


def glitch_position(rpm_read: float, rpm_median: float) -> float:
    """Position du front parasite, en fraction de demi-tour après le vrai front.

    La vitesse se lit sur deux périodes. Un front parasite à la fraction x
    d'un demi-tour donne deux lectures possibles : périodes (x, 1 − x),
    somme d'un demi-tour, soit deux fois la vitesse ; ou (1, x), soit
    2 / (1 + x) fois la vitesse. On rend x, ou 1 − x dans le premier cas
    (indiscernable).
    """
    ratio = rpm_read / rpm_median
    if ratio >= 1.9:
        return 0.5
    return 2.0 / ratio - 1.0


def analyze(capture: dict[str, Any], skip_s: float = 1.0) -> Report:
    samples = capture.get("samples")
    if not isinstance(samples, list) or not samples:
        raise RuntimeError("capture sans échantillons")
    t = [s["t_ms"] / 1000.0 for s in samples]
    running = [i for i, s in enumerate(samples) if s.get("pump_pct_commanded", 0) > 0]
    if not running:
        raise RuntimeError("la pompe n'est jamais commandée")
    start = running[0]
    stop = next((i for i in range(start, len(samples))
                 if samples[i].get("pump_pct_commanded", 0) == 0), None)
    end = stop if stop is not None else len(samples)
    window = [i for i in range(start, end) if t[i] >= t[start] + skip_s]
    if len(window) < 3:
        raise RuntimeError("marche stable trop courte")

    rpms = [samples[i]["pump_rpm"] for i in window if samples[i]["pump_rpm"] > 0]
    rpm_median = statistics.median(rpms)
    # Un STATUS_PUMP repris par deux échantillons ne compte qu'une fois.
    outliers = [(t[i], samples[i]["pump_rpm"],
                 glitch_position(samples[i]["pump_rpm"], rpm_median))
                for i in window if samples[i]["pump_rpm"] > OUTLIER_RATIO * rpm_median
                and samples[i]["pump_displaced_ml"] != samples[i - 1]["pump_displaced_ml"]]

    displaced = slope([(t[i], samples[i]["pump_displaced_ml"]) for i in window])
    pulses = slope([(t[i], samples[i]["flow_pulse_count"]) for i in window])
    weighed = [(t[i], samples[i]["weight_g"]) for i in window if weight_valid(samples[i])]
    weight = slope(weighed) if len(weighed) >= 3 else None

    last = samples[-1]
    weights = [s["weight_g"] for s in samples if weight_valid(s)]
    first_pulse = next((t[i] for i in range(start, len(samples))
                        if samples[i]["flow_pulse_count"] > samples[start]["flow_pulse_count"]),
                       None)
    first_weight = next((t[i] for i in range(start, len(samples))
                         if weight_valid(samples[i]) and samples[i]["weight_g"] >= 0.5), None)

    rundown = displaced_after = before = after = None
    if stop is not None:
        stopped = next((i for i in range(stop, len(samples)) if samples[i]["pump_rpm"] == 0), None)
        if stopped is not None:
            rundown = t[stopped] - t[stop]
            displaced_after = samples[stopped]["pump_displaced_ml"] - samples[stop]["pump_displaced_ml"]
            later = [s["pressure_bar"] for i, s in enumerate(samples)
                     if i >= stopped and s.get("pressure_bar") is not None]
            after = statistics.median(later) if later else None
        earlier = [samples[i]["pressure_bar"] for i in window
                   if t[i] >= t[stop] - 1.0 and samples[i].get("pressure_bar") is not None]
        before = statistics.median(earlier) if earlier else None

    return Report(
        run_start_s=t[start],
        stop_s=t[stop] if stop is not None else None,
        window=(t[window[0]], t[window[-1]]),
        rpm_median=rpm_median,
        outliers=outliers,
        displaced_ml_s=displaced,
        rpm_ml_s=rpm_median * PUMP_ML_PER_TURN / 60.0,
        flow_pulses_s=pulses,
        weight_g_s=weight,
        displaced_total_ml=last["pump_displaced_ml"] - samples[start]["pump_displaced_ml"],
        pulses_total=last["flow_pulse_count"] - samples[start]["flow_pulse_count"],
        weight_final_g=weights[-1] if weights else None,
        first_pulse_s=first_pulse,
        first_weight_s=first_weight,
        rundown_s=rundown,
        displaced_after_stop_ml=displaced_after,
        pressure_before_bar=before,
        pressure_after_bar=after,
    )


def fmt(value: float | None, digits: int = 2, unit: str = "") -> str:
    if value is None:
        return "—"
    text = f"{value:.{digits}f}".replace(".", ",")
    return f"{text} {unit}".rstrip()


def render(r: Report, k_capture: float | None) -> str:
    lines = []
    lines.append(f"Marche : {fmt(r.run_start_s)} s → arrêt {fmt(r.stop_s)} s ; "
                 f"fenêtre stable {fmt(r.window[0])}–{fmt(r.window[1])} s")
    lines.append("")
    lines.append("Tachymètre")
    lines.append(f"  vitesse médiane         {r.rpm_median:.0f} tr/min")
    excess = r.displaced_ml_s.per_s / r.rpm_ml_s - 1.0
    lines.append(f"  refoulé à cette vitesse {fmt(r.rpm_ml_s)} ml/s")
    lines.append(f"  refoulé compté          {fmt(r.displaced_ml_s.per_s)} ml/s "
                 f"({fmt(100 * excess, 1)} % d'impulsions en plus)")
    extra = excess * r.rpm_median * TACH_PULSES_PER_TURN / 60.0
    lines.append(f"  fronts en trop          ≈ {fmt(extra, 1)} par seconde")
    if r.outliers:
        lines.append(f"  lectures hautes         {len(r.outliers)} :")
        for at, rpm, x in r.outliers:
            lines.append(f"    {fmt(at)} s : {rpm} tr/min, ×{fmt(rpm / r.rpm_median)} ; "
                         f"front parasite à {fmt(x)} demi-tour")
    else:
        lines.append("  lectures hautes         aucune")
    lines.append("")
    lines.append("Débits sur la fenêtre stable")
    lines.append(f"  Digmesa                 {fmt(r.flow_pulses_s.per_s, 1)} imp/s"
                 + (f", {fmt(r.flow_pulses_s.per_s * 1000 / k_capture)} ml/s à {k_capture:.0f} imp/L"
                    if k_capture else ""))
    if r.weight_g_s is not None:
        w = r.weight_g_s.per_s
        lines.append(f"  balance                 {fmt(w)} g/s")
        lines.append(f"  balance / vitesse       {fmt(100 * w / r.rpm_ml_s, 1)} %")
        lines.append(f"  K en marche             {r.flow_pulses_s.per_s / w * 1000:.0f} imp/L")
    else:
        lines.append("  balance                 —")
    lines.append("")
    lines.append("Totaux")
    lines.append(f"  refoulé compté          {fmt(r.displaced_total_ml, 1)} ml")
    lines.append(f"  Digmesa                 {r.pulses_total} impulsions"
                 + (f", {fmt(r.pulses_total * 1000 / k_capture, 1)} ml à {k_capture:.0f} imp/L"
                    if k_capture else ""))
    lines.append(f"  balance, finale         {fmt(r.weight_final_g, 1, 'g')}")
    if r.weight_final_g:
        lines.append(f"  K total                 {r.pulses_total / r.weight_final_g * 1000:.0f} imp/L "
                     f"({fmt(r.weight_final_g / r.pulses_total, 3)} g par impulsion)")
    lines.append(f"  premier front Digmesa   {fmt(r.first_pulse_s)} s ; "
                 f"premier 0,5 g {fmt(r.first_weight_s)} s")
    lines.append("")
    lines.append("Arrêt")
    lines.append(f"  jusqu'à 0 tr/min        {fmt(r.rundown_s)} s "
                 f"({fmt(r.displaced_after_stop_ml)} ml refoulés)")
    lines.append(f"  pression avant / après  {fmt(r.pressure_before_bar)} / "
                 f"{fmt(r.pressure_after_bar)} bar")
    return "\n".join(lines)


def capture_k(capture: dict[str, Any]) -> float | None:
    """K du firmware qui a fait la capture, relu de volume_ml / impulsions."""
    for s in reversed(capture["samples"]):
        if s.get("flow_pulse_count", 0) > 0 and s.get("volume_ml", 0) > 0:
            first = capture["samples"][0]
            pulses = s["flow_pulse_count"] - first.get("flow_pulse_count", 0)
            volume = s["volume_ml"] - first.get("volume_ml", 0)
            if pulses > 0 and volume > 0:
                return pulses / volume * 1000
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("capture", type=Path, help="capture JSON de purge")
    parser.add_argument("--skip", type=float, default=1.0,
                        help="secondes ignorées après le départ de la pompe (1 par défaut)")
    args = parser.parse_args()
    try:
        capture = json.loads(args.capture.read_text())
        report = analyze(capture, args.skip)
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"erreur : {error}", file=sys.stderr)
        return 1
    print(render(report, capture_k(capture)))
    return 0


if __name__ == "__main__":
    sys.exit(main())

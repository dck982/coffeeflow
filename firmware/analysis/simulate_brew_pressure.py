#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Simule en boucle fermée la régulation de pression d'infusion.

Le modèle relie la commande de la pompe à engrenage à la pression mesurée :

- retard pur entre la commande calculée par l'écran et son effet (CAN,
  driver, tachymètre, mesure de pression : tout est regroupé) ;
- moteur du premier ordre, consigne = 45 tr/min par % moins 50 tr/min par
  bar de charge (≈ 4 500 tr/min à 100 % à vide, ≈ 4 050 à 9 bar) ;
- débit refoulé de 0,1 ml/tour, moins une fuite interne de 0,33 ml/s par
  bar (≈ 3,2 ml/s à 9,7 bar contre l'OPV, 14/11) ;
- galette linéaire : débit = pression / R ;
- compliance du circuit : dP/dt = (débit net − débit galette) / C.

Retard, constante de temps du moteur et compliance ne se mesurent pas
directement. ``calibrate`` les cherche sur une grille, en rejouant chaque
capture avec le régulateur de sa version d'écran (0.5.4 : PI toutes les
200 ms ; 0.5.5 : PI à chaque mesure et terme dérivé permanent) : le modèle
retenu doit reproduire le pic, le plateau, la plage de commande et la période
d'oscillation des deux infusions. ``compare`` simule ensuite chaque variante de
régulation, montée depuis 3 bar à 100 % comprise, sur plusieurs galettes.

Le régulateur est un portage de ``Machine::tick`` (screen-core) : activation
aux 2/3 de la cible, intégrale initialisée sans à-coup, anti-windup
conditionnel, plancher de 10 %, pente sur deux intervalles de mesure.

Exemples :
    uv run firmware/analysis/simulate_brew_pressure.py calibrate \\
        captures/261115-214106.json captures/261116-112941.json
    uv run firmware/analysis/simulate_brew_pressure.py compare --delay 0.25 0.35 0.4
"""

from __future__ import annotations

import argparse
import itertools
import json
import math
import random
import statistics
import sys
from dataclasses import dataclass, replace
from pathlib import Path


STEP_S = 0.005
SAMPLE_PERIOD_S = 0.1
TARGET_BAR = 9.0
PUMP_FLOOR_PCT = 10.0
PUMP_CEILING_PCT = 100.0
KP_PCT_PER_BAR = 15.0
KI_PCT_PER_BAR_SECOND = 3.0
MAX_INTEGRATION_S = 0.4
SLOPE_MAX_SPAN_S = 0.4
ACTIVATION_FRACTION = 2.0 / 3.0


@dataclass(frozen=True)
class Plant:
    delay_s: float = 0.25
    motor_tau_s: float = 0.05
    compliance_ml_per_bar: float = 0.45
    rpm_per_pct: float = 45.0
    rpm_drop_per_bar: float = 50.0
    slip_ml_s_per_bar: float = 0.33
    ml_per_rev: float = 0.1

    def net_flow_ml_s(self, rpm: float, pressure_bar: float) -> float:
        return rpm / 60.0 * self.ml_per_rev - self.slip_ml_s_per_bar * pressure_bar


@dataclass(frozen=True)
class Regulation:
    """Réglage du régulateur ; ``derivative`` vaut ``always`` ou ``rise``."""

    kd_pct_per_bar_per_s: float = 0.0
    derivative: str = "rise"
    period_s: float | None = None  # None : une commande par mesure


REGULATIONS = {
    "0.5.4": Regulation(kd_pct_per_bar_per_s=0.0, period_s=0.2),
    "0.5.5": Regulation(kd_pct_per_bar_per_s=7.5, derivative="always"),
    "0.5.6": Regulation(kd_pct_per_bar_per_s=3.0, derivative="rise"),
}


class Controller:
    """Portage du PI d'infusion de ``Machine::tick``, appelé à chaque mesure."""

    def __init__(self, regulation: Regulation, start_pct: float = 100.0):
        self.reg = regulation
        self.command_pct = start_pct
        self.integral_pct = start_pct
        self.active = False
        self.derivative_active = True
        self.history: list[tuple[float, float]] = []
        self.last_control_s: float | None = None

    def sample(self, t_s: float, pressure_bar: float) -> float:
        slope = None
        if len(self.history) == 2:
            oldest_s, oldest_bar = self.history[0]
            span_s = t_s - oldest_s
            if 0.0 < span_s <= SLOPE_MAX_SPAN_S + 1e-9:
                slope = (pressure_bar - oldest_bar) / span_s
        self.history = (self.history + [(t_s, pressure_bar)])[-2:]

        if self.last_control_s is None:
            self.last_control_s = t_s - (self.reg.period_s or SAMPLE_PERIOD_S)
        if self.reg.period_s is not None and t_s - self.last_control_s < self.reg.period_s - 1e-9:
            return self.command_pct
        elapsed_s = min(t_s - self.last_control_s, MAX_INTEGRATION_S)
        self.last_control_s = t_s

        if (self.reg.derivative == "rise" and self.active
                and slope is not None and slope <= 0.0):
            self.derivative_active = False
        derivative_pct = (self.reg.kd_pct_per_bar_per_s * (slope or 0.0)
                          if self.derivative_active else 0.0)
        error_bar = TARGET_BAR - pressure_bar
        if not self.active and pressure_bar >= TARGET_BAR * ACTIVATION_FRACTION:
            self.active = True
            self.integral_pct = clamp(self.command_pct - KP_PCT_PER_BAR * error_bar,
                                      PUMP_FLOOR_PCT, PUMP_CEILING_PCT)
        elif self.active:
            candidate = self.integral_pct + KI_PCT_PER_BAR_SECOND * error_bar * elapsed_s
            output = KP_PCT_PER_BAR * error_bar + candidate - derivative_pct
            winds_up_high = output > PUMP_CEILING_PCT and error_bar > 0.0
            winds_up_low = output < PUMP_FLOOR_PCT and error_bar < 0.0
            if not winds_up_high and not winds_up_low:
                self.integral_pct = clamp(candidate, PUMP_FLOOR_PCT, PUMP_CEILING_PCT)
        if self.active:
            output = KP_PCT_PER_BAR * error_bar + self.integral_pct - derivative_pct
            self.command_pct = float(round_half_away(clamp(output, PUMP_FLOOR_PCT, PUMP_CEILING_PCT)))
        return self.command_pct


@dataclass
class Trace:
    t_s: list[float]
    pressure_bar: list[float]
    command_pct: list[float]


def simulate(plant: Plant, regulation: Regulation, resistance: float, *,
             start_bar: float = 3.0, seconds: float = 12.0,
             noise_bar: float = 0.02, seed: int = 0) -> Trace:
    """Montée depuis ``start_bar`` à 100 %, puis régulation ; ``resistance``
    en bar·s/ml de débit net dans la galette."""
    rng = random.Random(seed)
    controller = Controller(regulation)
    delay_steps = max(1, round(plant.delay_s / STEP_S))
    pending = [controller.command_pct] * delay_steps
    rpm = plant.rpm_per_pct * 100.0 - plant.rpm_drop_per_bar * start_bar
    pressure = start_bar
    next_sample_s = 0.0
    trace = Trace([], [], [])
    for i in range(int(seconds / STEP_S)):
        t = i * STEP_S
        if t >= next_sample_s - 1e-9:
            next_sample_s += SAMPLE_PERIOD_S
            controller.sample(t, pressure + rng.gauss(0.0, noise_bar))
        pending.append(controller.command_pct)
        applied = pending.pop(0)
        setpoint = max(plant.rpm_per_pct * applied - plant.rpm_drop_per_bar * pressure, 0.0)
        rpm += (setpoint - rpm) * STEP_S / plant.motor_tau_s
        flow = plant.net_flow_ml_s(rpm, pressure) - pressure / resistance
        pressure = max(pressure + flow / plant.compliance_ml_per_bar * STEP_S, 0.0)
        trace.t_s.append(t)
        trace.pressure_bar.append(pressure)
        trace.command_pct.append(controller.command_pct)
    return trace


@dataclass
class Summary:
    peak_bar: float
    plateau_low_bar: float
    plateau_high_bar: float
    command_low_pct: float
    command_high_pct: float
    period_s: float | None

    @property
    def oscillates(self) -> bool:
        return self.plateau_high_bar - self.plateau_low_bar > 0.6


def summarize(t_s, pressure_bar, command_pct, end_s=None) -> Summary:
    """Plateau : de 2 s après le premier passage à 8 bar jusqu'à ``end_s``."""
    end_s = t_s[-1] if end_s is None else end_s
    first_8 = next((t for t, p in zip(t_s, pressure_bar) if p >= 8.0), t_s[0])
    plateau = [(t, p, c) for t, p, c in zip(t_s, pressure_bar, command_pct)
               if first_8 + 2.0 <= t <= end_s]
    if len(plateau) < 3:
        raise ValueError("plateau trop court")
    ts, ps, cs = zip(*plateau)
    mean = statistics.fmean(ps)
    ups = [ts[i] for i in range(1, len(ps)) if ps[i - 1] < mean <= ps[i]]
    period = (ups[-1] - ups[0]) / (len(ups) - 1) if len(ups) >= 4 else None
    peak = max(p for t, p in zip(t_s, pressure_bar) if t <= end_s)
    return Summary(peak, min(ps), max(ps), min(cs), max(cs), period)


def summarize_trace(trace: Trace) -> Summary:
    return summarize(trace.t_s, trace.pressure_bar, trace.command_pct)


@dataclass
class Observed:
    name: str
    regulation: Regulation
    resistance: float
    summary: Summary


def load_observed(path: Path, plant: Plant) -> Observed:
    """Plateau d'une capture d'infusion, et résistance équivalente de la
    galette dans le modèle : pression / (débit net du modèle à la vitesse et
    à la pression moyennes du plateau)."""
    capture = json.loads(path.read_text())
    version = capture.get("firmware", {}).get("screen", "")
    if version not in REGULATIONS:
        raise SystemExit(f"{path.name} : écran {version!r}, régulateur inconnu ({', '.join(REGULATIONS)})")
    samples = [s for s in capture["samples"] if s["mode"] == "infusion"]
    t = [s["t_ms"] / 1000.0 for s in samples]
    p = [float(s["pressure_bar"]) for s in samples]
    c = [float(s["pump_pct_commanded"]) for s in samples]
    summary = summarize(t, p, c)
    first_8 = next(tt for tt, pp in zip(t, p) if pp >= 8.0)
    plateau = [s for s, tt in zip(samples, t) if tt >= first_8 + 2.0]
    mean_bar = statistics.fmean(float(s["pressure_bar"]) for s in plateau)
    mean_rpm = statistics.fmean(float(s["pump_rpm"]) for s in plateau)
    resistance = mean_bar / plant.net_flow_ml_s(mean_rpm, mean_bar)
    return Observed(path.stem, REGULATIONS[version], resistance, summary)


def calibration_error(sim: Summary, obs: Summary) -> float:
    error = (abs(sim.peak_bar - obs.peak_bar) / 0.3
             + abs(sim.plateau_low_bar - obs.plateau_low_bar) / 0.3
             + abs(sim.plateau_high_bar - obs.plateau_high_bar) / 0.3
             + abs(sim.command_low_pct - obs.command_low_pct) / 20.0)
    if obs.period_s is not None and obs.oscillates:
        error += abs(sim.period_s - obs.period_s) / 0.1 if sim.period_s else 10.0
    return error


def calibrate(paths: list[Path]) -> int:
    base = Plant()
    observed = [load_observed(p, base) for p in paths]
    for o in observed:
        s = o.summary
        period = f", période {s.period_s:.2f} s" if s.oscillates and s.period_s else ""
        print(f"{o.name} : R ≈ {o.resistance:.2f} bar·s/ml ; pic {s.peak_bar:.2f} bar, plateau "
              f"{s.plateau_low_bar:.2f}–{s.plateau_high_bar:.2f} bar, commande "
              f"{s.command_low_pct:.0f}–{s.command_high_pct:.0f} %{period}")
    rows = []
    for delay, tau, comp in itertools.product((0.15, 0.2, 0.25, 0.3, 0.35, 0.4),
                                              (0.05, 0.1, 0.15),
                                              (0.35, 0.45, 0.55, 0.65)):
        plant = replace(base, delay_s=delay, motor_tau_s=tau, compliance_ml_per_bar=comp)
        sims = [summarize_trace(simulate(plant, o.regulation, o.resistance)) for o in observed]
        error = sum(calibration_error(s, o.summary) for s, o in zip(sims, observed))
        rows.append((error, plant, sims))
    rows.sort(key=lambda r: r[0])
    print("\nmeilleurs modèles (retard, constante du moteur, compliance) :")
    for error, plant, sims in rows[:5]:
        parts = []
        for o, s in zip(observed, sims):
            period = f" T {s.period_s:.2f} s" if s.oscillates and s.period_s else ""
            parts.append(f"{o.name} pic {s.peak_bar:.2f} plateau {s.plateau_low_bar:.2f}–"
                         f"{s.plateau_high_bar:.2f} cmd {s.command_low_pct:.0f}–{s.command_high_pct:.0f}{period}")
        print(f"  écart {error:5.2f}  {plant.delay_s:.2f} s / {plant.motor_tau_s:.2f} s / "
              f"{plant.compliance_ml_per_bar:.2f} ml/bar : " + " | ".join(parts))
    return 0


VARIANTS = {
    "0.5.4 PI 200 ms": REGULATIONS["0.5.4"],
    "0.5.5 Kd 7,5 permanent": REGULATIONS["0.5.5"],
    "Kd 0, PI à chaque mesure": Regulation(0.0),
    "Kd 7,5 montée seulement": Regulation(7.5, "rise"),
    "Kd 2 montée seulement": Regulation(2.0, "rise"),
    "0.5.6 Kd 3 montée seulement": REGULATIONS["0.5.6"],
    "Kd 2 permanent": Regulation(2.0, "always"),
    "Kd 3 permanent": Regulation(3.0, "always"),
}


def compare(delays: list[float], resistances: list[float], seeds: int) -> int:
    for delay in delays:
        plant = Plant(delay_s=delay)
        print(f"\nretard {delay:.2f} s, moteur {plant.motor_tau_s:.2f} s, "
              f"compliance {plant.compliance_ml_per_bar:.2f} ml/bar")
        for resistance in resistances:
            print(f"  R = {resistance:.2f} bar·s/ml")
            for name, regulation in VARIANTS.items():
                sums = [summarize_trace(simulate(plant, regulation, resistance, seed=s))
                        for s in range(seeds)]
                mean = lambda f: statistics.fmean(f(s) for s in sums)
                low, high = mean(lambda s: s.plateau_low_bar), mean(lambda s: s.plateau_high_bar)
                periods = [s.period_s for s in sums if s.oscillates and s.period_s]
                tail = f"  oscille, {statistics.fmean(periods):.2f} s" if periods else ""
                print(f"    {name:30s} pic {mean(lambda s: s.peak_bar):5.2f} bar  plateau "
                      f"{low:5.2f}–{high:5.2f} bar  commande {mean(lambda s: s.command_low_pct):3.0f}–"
                      f"{mean(lambda s: s.command_high_pct):3.0f} %{tail}")
    return 0


def clamp(value: float, low: float, high: float) -> float:
    return min(max(value, low), high)


def round_half_away(value: float) -> int:
    """``libm::roundf`` : demi arrondi en s'éloignant de zéro."""
    return int(math.floor(value + 0.5)) if value >= 0 else -int(math.floor(-value + 0.5))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    cal = sub.add_parser("calibrate", help="cale retard, moteur et compliance sur des captures")
    cal.add_argument("captures", type=Path, nargs="+")
    cmp_ = sub.add_parser("compare", help="simule les variantes de régulation")
    cmp_.add_argument("--delay", type=float, nargs="+", default=[0.25, 0.35, 0.4])
    cmp_.add_argument("--resistance", type=float, nargs="+", default=[4.0, 5.2, 6.4, 8.0],
                      help="bar·s/ml de débit net (défaut : 4,0 5,2 6,4 8,0)")
    cmp_.add_argument("--seeds", type=int, default=3)
    args = parser.parse_args(argv)
    if args.command == "calibrate":
        return calibrate(args.captures)
    return compare(args.delay, args.resistance, args.seeds)


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Rejoue le PI de pression d'une capture haute fréquence CoffeeFlow.

Seuls les échantillons dont ``mode`` vaut ``infusion`` sont employés. Le
replay reproduit la logique de ``core::machine::Machine`` : échantillonnage
de la pression maintenu entre deux captures, période de contrôle, activation
à ``target - margin``, saturation et anti-windup.

Exemples :
    uv run firmware/tools/replay_brew_pi.py captures/260921-081442.json
    uv run firmware/tools/replay_brew_pi.py captures/260921-135930.json --kp 15 --ki 6
    uv run firmware/tools/replay_brew_pi.py captures/260921-135930.json --csv /tmp/pi.csv

Le CSV contient la pression enregistrée et la commande contrefactuelle. Il
ne prédit pas la pression avec de nouveaux gains : cela nécessiterait aussi
un modèle du groupe café/pompe.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


PRESSURE_VALID_FLAG = 0x01


@dataclass(frozen=True)
class Parameters:
    target_bar: float = 9.0
    pump_initial_pct: float = 100.0
    pump_floor_pct: float = 50.0
    kp_pct_per_bar: float = 15.0
    ki_pct_per_bar_second: float = 3.0
    activation_margin_bar: float = 3.0
    control_period_ms: int = 200
    max_integration_period_ms: int = 400
    machine_tick_ms: int = 50
    pressure_hold_ms: int = 300


@dataclass
class Controller:
    params: Parameters
    command_pct: int
    integral_pct: float
    active: bool = False

    @classmethod
    def initial(cls, params: Parameters) -> "Controller":
        command = round(params.pump_initial_pct)
        return cls(params=params, command_pct=command, integral_pct=float(command))

    def update(self, pressure_bar: float, elapsed_ms: int) -> None:
        """Port Python direct du bloc PI de Machine::tick()."""
        p = self.params
        error_bar = p.target_bar - pressure_bar
        if not self.active and pressure_bar >= p.target_bar - p.activation_margin_bar:
            self.active = True
            self.integral_pct = clamp(
                self.command_pct - p.kp_pct_per_bar * error_bar,
                p.pump_floor_pct,
                p.pump_initial_pct,
            )
        elif self.active:
            proportional_pct = p.kp_pct_per_bar * error_bar
            candidate_integral_pct = self.integral_pct + (
                p.ki_pct_per_bar_second * error_bar * elapsed_ms / 1000.0
            )
            candidate_output_pct = proportional_pct + candidate_integral_pct
            winds_up_high = candidate_output_pct > p.pump_initial_pct and error_bar > 0.0
            winds_up_low = candidate_output_pct < p.pump_floor_pct and error_bar < 0.0
            if not winds_up_high and not winds_up_low:
                self.integral_pct = clamp(candidate_integral_pct, p.pump_floor_pct, p.pump_initial_pct)

        if self.active:
            output_pct = clamp(
                p.kp_pct_per_bar * error_bar + self.integral_pct,
                p.pump_floor_pct,
                p.pump_initial_pct,
            )
            # std::lround() arrondit les demi-valeurs en s'éloignant de zéro.
            self.command_pct = math.floor(output_pct + 0.5)


def clamp(value: float, low: float, high: float) -> float:
    return max(low, min(value, high))


def load_infusion(path: Path) -> list[dict[str, Any]]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("schema") != "coffeeflow.hf_capture.v1":
        raise RuntimeError("capture inconnue ou incompatible")
    samples = [sample for sample in payload.get("samples", []) if sample.get("mode") == "infusion"]
    if not samples:
        raise RuntimeError("la capture ne contient aucun échantillon mode=infusion")
    return samples


def replay(samples: list[dict[str, Any]], params: Parameters) -> list[dict[str, Any]]:
    """Rejoue la tâche machine à 50 ms, avec maintien de dernière mesure."""
    controller = Controller.initial(params)
    first_ms = int(samples[0]["t_ms"])
    last_ms = int(samples[-1]["t_ms"])
    sample_index = 0
    current = samples[0]
    last_valid = current if int(current.get("flags", 0)) & PRESSURE_VALID_FLAG else None
    last_valid_ms = first_ms if last_valid else None
    last_control_ms = first_ms
    rows: list[dict[str, Any]] = []

    # La tâche firmware tourne toutes les 50 ms. Les timestamps de capture
    # sont légèrement jitterés ; le dernier échantillon disponible est donc
    # maintenu jusqu'au suivant, comme Snapshot dans le firmware.
    for now_ms in range(first_ms, last_ms + 1, params.machine_tick_ms):
        while sample_index + 1 < len(samples) and int(samples[sample_index + 1]["t_ms"]) <= now_ms:
            sample_index += 1
            current = samples[sample_index]
            if int(current.get("flags", 0)) & PRESSURE_VALID_FLAG:
                last_valid = current
                last_valid_ms = int(current["t_ms"])
        if now_ms - last_control_ms >= params.control_period_ms and last_valid_ms is not None and \
                now_ms - last_valid_ms <= params.pressure_hold_ms:
            controller.update(
                float(last_valid["pressure_bar"]),
                min(now_ms - last_control_ms, params.max_integration_period_ms),
            )
            last_control_ms = now_ms

        # Emit one row for each original capture point, rather than each task tick.
        while len(rows) < len(samples) and int(samples[len(rows)]["t_ms"]) <= now_ms:
            observed = samples[len(rows)]
            rows.append({
                "t_ms": int(observed["t_ms"]),
                "pressure_bar": float(observed["pressure_bar"]),
                "pressure_valid": bool(int(observed.get("flags", 0)) & PRESSURE_VALID_FLAG),
                "observed_command_pct": int(observed["pump_pct_commanded"]),
                "simulated_command_pct": controller.command_pct,
                "integral_pct": controller.integral_pct,
                "pi_active": controller.active,
            })
    # A capture timestamp can fall in the final <50 ms interval.
    while len(rows) < len(samples):
        observed = samples[len(rows)]
        rows.append({
            "t_ms": int(observed["t_ms"]), "pressure_bar": float(observed["pressure_bar"]),
            "pressure_valid": bool(int(observed.get("flags", 0)) & PRESSURE_VALID_FLAG),
            "observed_command_pct": int(observed["pump_pct_commanded"]),
            "simulated_command_pct": controller.command_pct, "integral_pct": controller.integral_pct,
            "pi_active": controller.active,
        })
    return rows


def report(rows: list[dict[str, Any]]) -> str:
    active = [row for row in rows if row["pi_active"]]
    errors = [abs(row["simulated_command_pct"] - row["observed_command_pct"]) for row in active]
    valid = sum(row["pressure_valid"] for row in active)
    return (
        f"PI actif : {len(active)} échantillons, pression valide : {valid}/{len(active)}\n"
        f"Commande simulée finale : {active[-1]['simulated_command_pct']} % "
        f"(observée : {active[-1]['observed_command_pct']} %)\n"
        f"Écart absolu moyen commande (PI actif) : {sum(errors) / len(errors):.2f} point(s)"
    )


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--target", type=float, default=9.0, help="consigne en bar (défaut : 9)")
    parser.add_argument("--kp", type=float, default=15.0, help="Kp en %%/bar (défaut : 15)")
    parser.add_argument("--ki", type=float, default=3.0, help="Ki en %%/(bar·s) (défaut : 3)")
    parser.add_argument("--activation-margin", type=float, default=3.0, help="marge d'activation en bar")
    parser.add_argument("--csv", type=Path, help="écrit les séries observée/simulée en CSV")
    args = parser.parse_args()
    try:
        params = Parameters(target_bar=args.target, kp_pct_per_bar=args.kp,
                            ki_pct_per_bar_second=args.ki,
                            activation_margin_bar=args.activation_margin)
        rows = replay(load_infusion(args.capture), params)
        print(report(rows))
        if args.csv:
            write_csv(args.csv, rows)
            print(f"CSV écrit dans {args.csv}")
    except (OSError, ValueError, json.JSONDecodeError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["bleak>=0.22"]
# ///
"""Enregistre les codes bruts A0/A1 de la sonde chaudière et A2 du XDB401
analogique pour les étalonner.

Exemple : COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
    uv run firmware/tools/record_probe.py

Interroge /telemetry à 5 Hz jusqu'à Ctrl-C (ou --duration). Plonger la sonde
dans les bains pendant l'enregistrement ; retrouver ensuite les plateaux sur
un tracé de a0_raw / a1_raw. Le rapport a0/a1 − 1 vaut R_sonde / R_fixe.
a2_raw vaut null si A2 n'a pas pu être lu ; un code vaut 125 µV. La pression
convertie (pressure_bar, pressure_valid) est relevée en même temps, pour lire
l'état au repos, hors des captures HF qui démarrent avec la pompe.

Avec --scace, la sonde SCACE est lue en BLE directement par le Mac (le Core2
accepte l'écran et le Mac à la fois) : chaque mesure porte la dernière
température reçue (scace_c, null si absente ou plus vieille que 300 ms) et
son âge (scace_age_ms). L'écran ne la donne pas dans /telemetry : en mode
Wi-Fi, son BLE est éteint.

Les captures sont écrites par défaut dans captures/ (ignoré par Git).
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

from record_heating import request_json
from scace_ble_log import LatestTemperature, follow_in_background


PROJECT_ROOT = Path(__file__).resolve().parents[2]
CAPTURE_DIR = PROJECT_ROOT / "captures"
POLL_INTERVAL_S = 0.2
STATUS_INTERVAL_S = 2.0


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds")


def is_number(value: Any) -> bool:
    return not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)


def probe_sample(telemetry: dict[str, Any]) -> dict[str, Any]:
    boiler = telemetry.get("temperature", {}).get("boiler", {})
    a0, a1 = boiler.get("ntc_a0_raw"), boiler.get("ntc_a1_raw")
    if not is_number(a0) or not is_number(a1):
        raise RuntimeError("GET /telemetry: codes ntc_a0_raw / ntc_a1_raw absents")
    return {
        "a0_raw": a0,
        "a1_raw": a1,
        "c": boiler.get("c"),
        "valid": boiler.get("valid"),
        "freshness": boiler.get("freshness"),
        "age_ms": boiler.get("age_ms"),
        "a2_raw": telemetry.get("pressure", {}).get("a2_raw"),
        "pressure_bar": telemetry.get("pressure", {}).get("bar"),
        "pressure_valid": telemetry.get("pressure", {}).get("valid"),
    }


def record_probe(output: Path,
                 client: Callable[[str, str, dict[str, Any] | None], dict[str, Any]],
                 *, clock: Callable[[], float] = time.monotonic,
                 sleep: Callable[[float], None] = time.sleep,
                 duration_s: float | None = None,
                 scace: LatestTemperature | None = None) -> dict[str, Any]:
    if duration_s is not None and (not math.isfinite(duration_s) or duration_s <= 0):
        raise ValueError("duration doit être un nombre de secondes positif")
    if output.exists():
        raise FileExistsError(f"le fichier existe déjà : {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    capture: dict[str, Any] = {
        "started_at_utc": utc_now(),
        "interval_s": POLL_INTERVAL_S,
        "max_duration_s": duration_s,
        "samples": [],
        "stop_reason": "error",
    }
    try:
        started = clock()
        next_poll = started
        next_status = started
        while duration_s is None or clock() < started + duration_s:
            sleep(max(0.0, next_poll - clock()))
            if duration_s is not None and clock() >= started + duration_s:
                break
            sample = probe_sample(client("GET", "/telemetry", None))
            if scace is not None:
                sample["scace_c"], sample["scace_age_ms"] = scace.read()
            received = clock()
            capture["samples"].append({
                "elapsed_s": round(received - started, 3),
                "received_at_utc": utc_now(),
                **sample,
            })
            if received >= next_status:
                a0, a1 = sample["a0_raw"], sample["a1_raw"]
                ratio = f"{a0 / a1 - 1:.5f}" if a1 > 0 else "indisponible"
                a2 = sample["a2_raw"]
                a2_text = f"{a2} ({a2 * 0.125:.1f} mV)" if is_number(a2) else "indisponible"
                bar = sample["pressure_bar"]
                bar_text = f"{bar:.3f} bar" if is_number(bar) and sample["pressure_valid"] else "indisponible"
                scace_text = ""
                if scace is not None:
                    scace_c = sample["scace_c"]
                    scace_text = f" / SCACE {scace_c:.2f} °C" if is_number(scace_c) else " / SCACE indisponible"
                print(f"{received - started:6.1f} s : a0 {a0} / a1 {a1} / a0/a1−1 {ratio}"
                      f" / {sample['c']} °C ({sample['freshness']}) / a2 {a2_text}"
                      f" / pression {bar_text}{scace_text}", flush=True)
                while next_status <= received:
                    next_status += STATUS_INTERVAL_S
            next_poll += POLL_INTERVAL_S
            while next_poll < received:
                next_poll += POLL_INTERVAL_S
        capture["stop_reason"] = "duration"
    except KeyboardInterrupt:
        capture["stop_reason"] = "interrupted"
    except (OSError, RuntimeError, ValueError, TypeError) as error:
        capture["stop_reason"] = "error"
        capture["error"] = str(error)
    finally:
        capture["ended_at_utc"] = utc_now()
        with output.open("x", encoding="utf-8") as handle:
            json.dump(capture, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
    return capture


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="fichier JSON de sortie")
    parser.add_argument("--duration", type=float, metavar="SECONDS",
                        help="durée en secondes (défaut : jusqu'à Ctrl-C)")
    parser.add_argument("--scace", action="store_true", help="lire aussi la sonde SCACE en BLE")
    parser.add_argument("--scace-address", help="adresse BLE de la sonde (sinon par service)")
    args = parser.parse_args()
    if args.duration is not None and (not math.isfinite(args.duration) or args.duration <= 0):
        parser.error("--duration doit être un nombre de secondes positif")
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    address = os.environ.get("COFFEEFLOW_IP")
    if not token or not address:
        parser.error("COFFEEFLOW_HTTP_TOKEN et COFFEEFLOW_IP sont requis")
    url = f"http://{address.rstrip('/')}"
    output = args.output or CAPTURE_DIR / f"probe-{datetime.now():%Y%m%d-%H%M%S-%f}.json"
    client = lambda method, path, body: request_json(url, token, method, path, body)
    scace = None
    if args.scace:
        scace = LatestTemperature()
        follow_in_background(scace, address=args.scace_address)
    try:
        capture = record_probe(output, client, duration_s=args.duration, scace=scace)
    except (OSError, ValueError) as error:
        print(f"erreur du relevé : {error}", file=sys.stderr)
        return 1
    print(f"relevé écrit dans {output} ({len(capture['samples'])} mesures, {capture['stop_reason']})")
    if capture["stop_reason"] == "error":
        print(f"erreur : {capture.get('error', 'inconnue')}", file=sys.stderr)
        return 1
    # Ctrl-C est la façon normale d'arrêter cet enregistrement.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

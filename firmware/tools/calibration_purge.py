#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Exécute une purge HTTP bornée et archive les instantanés capteurs.

Le poids est volontairement demandé après la purge : le mode Wi-Fi coupe le
BLE de la balance Acaia sur l'image actuelle.
"""

import argparse
import json
import math
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


def request(base_url: str, token: str, method: str, path: str,
            body: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Authorization": f"Bearer {token}", "Accept": "application/json"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    payload = "" if body is None else f" --data {json.dumps(body, separators=(',', ':'))!r}"
    print(f"running curl -X {method} {base_url}{path}{payload}", flush=True)
    req = Request(base_url + path, data=data, headers=headers, method=method)
    try:
        with urlopen(req, timeout=10) as response:
            print(f"got HTTP status {response.status}", flush=True)
            return json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        print(f"got HTTP status {error.code}", flush=True)
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path}: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"{method} {path}: {error.reason}") from error


def telemetry_delta(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key in ("flow_pulse_count", "volume_ml"):
        if isinstance(before.get(key), (int, float)) and isinstance(after.get(key), (int, float)):
            result[key] = after[key] - before[key]
    return result


def safe_max_seconds(duration_s: float) -> int:
    """Leave two seconds between the requested duration and the firmware cutoff."""
    return min(60, max(5, math.ceil((duration_s + 2) / 5) * 5))


def file_number(value: float) -> str:
    """Stable filename fragment without a decimal point."""
    return f"{value:g}".replace(".", "p")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--http", help="base URL (sinon http://$COFFEEFLOW_IP)")
    parser.add_argument("--token", help="bearer token (or COFFEEFLOW_HTTP_TOKEN)")
    parser.add_argument("--pump-pct", type=int, required=True, help="pump power: 20..100, multiple of 5")
    parser.add_argument("--duration-s", type=float, required=True, help="purge duration: 0.1..55 s")
    parser.add_argument("--settle-s", type=float, default=5.0,
                        help="wait after the actuator TTL expires before final telemetry (default: 5 s)")
    parser.add_argument("--weight-g", type=float, help="weight manually measured after the purge")
    parser.add_argument("--output", type=Path,
                        help="JSON output file (default: ./calibration/purge-…json)")
    args = parser.parse_args()

    token = args.token or os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("--token ou COFFEEFLOW_HTTP_TOKEN est requis")
    if args.pump_pct < 20 or args.pump_pct > 100 or args.pump_pct % 5:
        parser.error("--pump-pct doit être entre 20 et 100, par pas de 5")
    if not 0.1 <= args.duration_s <= 55:
        parser.error("--duration-s doit être entre 0.1 et 55 secondes")
    if not 0 <= args.settle_s <= 20:
        parser.error("--settle-s doit être entre 0 et 20 secondes")

    base_url = args.http or (f"http://{os.environ['COFFEEFLOW_IP']}"
                             if os.environ.get("COFFEEFLOW_IP") else None)
    if base_url is None:
        parser.error("--http ou COFFEEFLOW_IP est requis")
    base_url = base_url.rstrip("/")
    timestamp = datetime.now(timezone.utc)
    output = args.output or Path("calibration") / (
        f"purge-{args.pump_pct}pct-{file_number(args.duration_s)}s-"
        f"{timestamp:%Y%m%dT%H%M%SZ}.json")
    record: dict[str, Any] = {
        "schema_version": 1,
        "started_at_utc": timestamp.isoformat(),
        "request": {"pump_pct": args.pump_pct, "duration_s": args.duration_s,
                    "settle_s": args.settle_s},
    }
    config: dict[str, Any] | None = None
    started = False

    try:
        config = request(base_url, token, "GET", "/config")
        record["telemetry_before"] = request(base_url, token, "GET", "/telemetry")
        max_s = safe_max_seconds(args.duration_s)
        record["firmware_safety_max_s"] = max_s
        request(base_url, token, "POST", "/config", {
            "version": config["version"],
            "purge": {"pump_pct": args.pump_pct, "max_s": max_s},
        })
        ttl_ms = max(1, math.ceil(args.duration_s * 1000))
        record["request"]["ttl_ms"] = ttl_ms
        response = request(base_url, token, "POST", "/action", {
            "action": "set_actuators",
            "dimmer": args.pump_pct,
            "ttl_ms": ttl_ms,
        })
        if not response.get("ok"):
            raise RuntimeError(f"actionneurs refusés: {response.get('reason', 'raison inconnue')}")
        started = True
        time.sleep(args.duration_s)
    except (KeyError, RuntimeError) as error:
        record["error"] = str(error)
        print(f"calibration error: {error}", flush=True)
    finally:
        if started:
            time.sleep(args.settle_s)
        try:
            record["telemetry_after"] = request(base_url, token, "GET", "/telemetry")
            if "telemetry_before" in record:
                record["delta"] = telemetry_delta(record["telemetry_before"], record["telemetry_after"])
        except RuntimeError as error:
            record["telemetry_after_error"] = str(error)
        if config is not None:
            try:
                record["config_restore"] = request(base_url, token, "POST", "/config", {
                    "version": config["version"], "purge": config["purge"],
                })
            except (KeyError, RuntimeError) as error:
                record["config_restore_error"] = str(error)

        if "error" not in record and args.weight_g is not None:
            if args.weight_g < 0:
                record["weight_input_error"] = "--weight-g must be positive"
            else:
                record["measured_weight_g"] = args.weight_g
        if "error" not in record and args.weight_g is None and sys.stdin.isatty():
            entered = input("Poids d'eau mesuré après purge (g, Entrée pour ignorer) : ").strip()
            if entered:
                try:
                    measured_weight = float(entered.replace(",", "."))
                    if measured_weight < 0:
                        raise ValueError
                    record["measured_weight_g"] = measured_weight
                except ValueError:
                    record["weight_input_error"] = "invalid manual weight"
        if (record.get("measured_weight_g", 0) > 0 and
                record.get("delta", {}).get("flow_pulse_count", 0) > 0):
            record["derived"] = {
                "pulses_per_gram": record["delta"]["flow_pulse_count"] / record["measured_weight_g"],
                "ml_per_pulse_approx": record["measured_weight_g"] / record["delta"]["flow_pulse_count"],
            }

        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"Résultat écrit dans {output}")

    if "error" in record:
        return 1
    if "measured_weight_g" not in record:
        print("Saisir ensuite le poids mesuré dans le JSON ou passer --weight-g au prochain essai.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

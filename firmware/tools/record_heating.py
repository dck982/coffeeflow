#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Surveille une chauffe déjà active et enregistre les mesures.

Exemple : COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
    uv run firmware/tools/record_heating.py --monitor-time 120

La chauffe doit être activée manuellement avant de lancer le script.

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
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


PROJECT_ROOT = Path(__file__).resolve().parents[2]
CAPTURE_DIR = PROJECT_ROOT / "captures"
POLL_INTERVAL_S = 0.5
STATUS_INTERVAL_S = 10.0


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds")


def request_json(url: str, token: str, method: str, path: str,
                 body: dict[str, Any] | None = None) -> dict[str, Any]:
    data = json.dumps(body).encode("utf-8") if body is not None else None
    request = Request(
        f"{url}{path}", data=data,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/json",
            **({"Content-Type": "application/json"} if data is not None else {}),
        },
        method=method,
    )
    try:
        with urlopen(request, timeout=2 if path == "/telemetry" else 5) as response:
            payload = json.load(response)
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path}: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"{method} {path}: {error.reason}") from error
    if not isinstance(payload, dict):
        raise RuntimeError(f"{method} {path}: réponse JSON inattendue")
    return payload


def target_from_config(config: dict[str, Any]) -> tuple[int, float]:
    version = config.get("version")
    heating = config.get("heating")
    target = heating.get("brew_temperature_c") if isinstance(heating, dict) else None
    if type(version) is not int or isinstance(target, bool) or not isinstance(target, (int, float)):
        raise RuntimeError("GET /config: version ou consigne chaudière absente")
    target = float(target)
    if not math.isfinite(target):
        raise RuntimeError("GET /config: consigne chaudière invalide")
    return version, target


def valid_temperature(sample: dict[str, Any]) -> float | None:
    boiler = sample.get("temperature", {}).get("boiler", {})
    temperature = boiler.get("c")
    if (boiler.get("valid") is True and
            boiler.get("freshness") == "fresh" and
            not isinstance(temperature, bool) and isinstance(temperature, (int, float)) and
            math.isfinite(temperature)):
        return float(temperature)
    return None


def confirmed_heating(response: dict[str, Any], enabled: bool) -> bool:
    heating = response.get("heating")
    return isinstance(heating, dict) and heating.get("enabled") is enabled


def record_heating(output: Path,
                   client: Callable[[str, str, dict[str, Any] | None], dict[str, Any]],
                   *, clock: Callable[[], float] = time.monotonic,
                   sleep: Callable[[float], None] = time.sleep,
                   monitor_time_s: float = 60.0) -> dict[str, Any]:
    if not math.isfinite(monitor_time_s) or monitor_time_s <= 0:
        raise ValueError("monitor-time doit être un nombre de secondes positif")
    if output.exists():
        raise FileExistsError(f"le fichier existe déjà : {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    capture: dict[str, Any] = {
        "started_at_utc": utc_now(),
        "mode": "monitor",
        "interval_s": POLL_INTERVAL_S,
        "max_duration_s": monitor_time_s,
        "samples": [],
        "stop_reason": "error",
    }
    try:
        config = client("GET", "/config", None)
        _version, target_c = target_from_config(config)
        capture["target_c"] = target_c

        started = clock()
        deadline = started + monitor_time_s
        next_poll = started
        next_status = started
        while clock() < deadline:
            sleep(max(0.0, next_poll - clock()))
            if clock() >= deadline:
                break
            telemetry = client("GET", "/telemetry", None)
            received = clock()
            capture["samples"].append({
                "elapsed_s": round(received - started, 3),
                "received_at_utc": utc_now(),
                "telemetry": telemetry,
            })
            temperature = valid_temperature(telemetry)
            if received >= next_status:
                current = f"{temperature:.1f} °C" if temperature is not None else "indisponible"
                power = telemetry.get("heating", {}).get("power_pct")
                requested = (f"{power:.1f} %" if isinstance(power, (int, float)) and
                             not isinstance(power, bool) and math.isfinite(power) else "indisponible")
                print(f"{received - started:5.1f} s : chaudière {current} / cible {target_c:.1f} °C"
                      f" / puissance demandée {requested}", flush=True)
                while next_status <= received:
                    next_status += STATUS_INTERVAL_S
            next_poll += POLL_INTERVAL_S
            while next_poll < received:
                next_poll += POLL_INTERVAL_S
        else:
            capture["stop_reason"] = "monitor_duration"
        if capture["stop_reason"] == "error":
            capture["stop_reason"] = "monitor_duration"
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
    parser.add_argument("--monitor-time", type=float, default=60.0, metavar="SECONDS",
                        help="durée de la surveillance en secondes (défaut : 60)")
    args = parser.parse_args()
    if not math.isfinite(args.monitor_time) or args.monitor_time <= 0:
        parser.error("--monitor-time doit être un nombre de secondes positif")
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    address = os.environ.get("COFFEEFLOW_IP")
    if not token or not address:
        parser.error("COFFEEFLOW_HTTP_TOKEN et COFFEEFLOW_IP sont requis")
    url = f"http://{address.rstrip('/')}"
    output = args.output or CAPTURE_DIR / f"monitor-heating-{datetime.now():%Y%m%d-%H%M%S-%f}.json"
    client = lambda method, path, body: request_json(url, token, method, path, body)
    try:
        capture = record_heating(output, client, monitor_time_s=args.monitor_time)
    except (OSError, ValueError) as error:
        print(f"erreur du relevé : {error}", file=sys.stderr)
        return 1
    print(f"relevé écrit dans {output} ({len(capture['samples'])} mesures, {capture['stop_reason']})")
    if capture["stop_reason"] == "error":
        print(f"erreur : {capture.get('error', 'inconnue')}", file=sys.stderr)
        return 1
    if capture["stop_reason"] == "interrupted":
        return 130
    return 0 if capture["stop_reason"] == "monitor_duration" else 2


if __name__ == "__main__":
    raise SystemExit(main())

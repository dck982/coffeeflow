#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Effectue une purge Wi-Fi de la durée demandée, en secondes.

Exemple : COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
    uv run firmware/tools/purge.py 10

La purge s'arrête aussi automatiquement à purge.max_s dans le firmware.
"""

from __future__ import annotations

import argparse
import math
import os
import sys
import time
from typing import Any, Callable

from record_heating import request_json


Client = Callable[[str, str, dict[str, Any] | None], dict[str, Any]]


def describe_telemetry(label: str, telemetry: dict[str, Any]) -> None:
    temperature = telemetry.get("sensors", {}).get("boiler_temperature_c")
    temperature_text = f"{temperature:.2f} °C" if isinstance(temperature, (int, float)) else "indisponible"
    power = telemetry.get("actuators", {}).get("heating_power_pct")
    power_text = f"{power:.1f} %" if isinstance(power, (int, float)) else "indisponible"
    print(f"{label} : chaudière {temperature_text}, chauffe {power_text}, cycle {telemetry.get('cycle', {}).get('state')}", flush=True)


def purge(duration_s: float, client: Client, *, sleep: Callable[[float], None] = time.sleep) -> None:
    if not math.isfinite(duration_s) or duration_s <= 0:
        raise ValueError("la durée doit être un nombre de secondes positif")

    config = client("GET", "/config", None)
    purge_config = config.get("purge")
    max_s = purge_config.get("max_s") if isinstance(purge_config, dict) else None
    if isinstance(max_s, bool) or not isinstance(max_s, (int, float)) or not math.isfinite(max_s):
        raise RuntimeError("GET /config: purge.max_s absent ou invalide")
    if duration_s >= max_s:
        raise ValueError(f"la durée doit être inférieure à purge.max_s ({max_s:g} s)")

    before = client("GET", "/telemetry", None)
    cycle = before.get("cycle", {}).get("state")
    if cycle not in ("idle", "finished"):
        raise RuntimeError(f"purge impossible : cycle actuel {cycle!r}")
    describe_telemetry("Avant", before)

    # Le POST peut être appliqué même si la réponse Wi-Fi se perd. Dans ce cas,
    # envoyer quand même purge_release pour ne pas attendre le timeout firmware.
    press_attempted = False
    try:
        press_attempted = True
        response = client("POST", "/action", {"action": "purge_press"})
        if response.get("ok") is not True:
            raise RuntimeError(f"purge_press refusé : {response.get('reason')}")
        print(f"Purge démarrée pour {duration_s:g} s", flush=True)
        sleep(duration_s)
    finally:
        if press_attempted:
            response = client("POST", "/action", {"action": "purge_release"})
            if response.get("ok") is not True:
                raise RuntimeError(f"purge_release non confirmé : {response.get('reason')}")
            print("Purge arrêtée", flush=True)

    describe_telemetry("Après", client("GET", "/telemetry", None))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("seconds", type=float, help="durée de la purge en secondes")
    parser.add_argument("--http", help="URL de base (sinon http://$COFFEEFLOW_IP)")
    args = parser.parse_args()
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        parser.error("seconds doit être un nombre de secondes positif")
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("COFFEEFLOW_HTTP_TOKEN est requis")
    if args.http:
        url = args.http.rstrip("/")
    elif address := os.environ.get("COFFEEFLOW_IP"):
        url = f"http://{address.rstrip('/')}"
    else:
        parser.error("--http ou COFFEEFLOW_IP est requis")

    client = lambda method, path, body: request_json(url, token, method, path, body)
    try:
        purge(args.seconds, client)
    except (OSError, RuntimeError, ValueError, TypeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

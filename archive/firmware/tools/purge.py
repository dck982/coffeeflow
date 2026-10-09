#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Effectue une purge Wi-Fi de la durée demandée, en secondes.

Exemple : COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
    uv run firmware/tools/purge.py 15

L'arrêt est confié au firmware : purge.max_s, qui n'accepte que des multiples
de 5 s de 5 à 60 s, est réglé sur la durée arrondie au multiple supérieur avant
purge_press, puis rétabli. Pour un multiple de 5, la purge s'arrête donc à la
durée exacte sans dépendre d'un purge_release qui pourrait se perdre en Wi-Fi.
Sinon, purge_release l'arrête à la durée demandée, et le firmware au plus tard
au multiple suivant.
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

POLL_INTERVAL_S = 0.5
# Marge laissée au firmware après la durée avant d'envoyer purge_release.
STOP_GRACE_S = 3.0


def describe_telemetry(label: str, telemetry: dict[str, Any]) -> None:
    temperature = telemetry.get("temperature", {}).get("boiler", {}).get("c")
    temperature_text = f"{temperature:.2f} °C" if isinstance(temperature, (int, float)) else "indisponible"
    power = telemetry.get("heating", {}).get("power_pct")
    power_text = f"{power:.1f} %" if isinstance(power, (int, float)) else "indisponible"
    print(f"{label} : chaudière {temperature_text}, chauffe {power_text}, cycle {telemetry.get('brew', {}).get('state')}", flush=True)


def cycle_state(client: Client) -> str | None:
    return client("GET", "/telemetry", None).get("brew", {}).get("state")


def set_purge_max_s(client: Client, version: int, max_s: int) -> None:
    response = client("POST", "/config", {"version": version, "purge": {"max_s": max_s}})
    if response.get("purge", {}).get("max_s") != max_s:
        raise RuntimeError(f"POST /config: purge.max_s non appliqué ({max_s} s)")


def press(client: Client) -> None:
    try:
        response = client("POST", "/action", {"action": "purge_press"})
    except RuntimeError as error:
        # La commande a pu être appliquée malgré la réponse perdue. Ne pas la
        # renvoyer : une purge courte déjà terminée serait relancée.
        if cycle_state(client) == "purge":
            return
        raise RuntimeError(f"purge non démarrée après une réponse perdue : {error}") from error
    if response.get("ok") is not True:
        raise RuntimeError(f"purge_press refusé : {response.get('reason')}")


def release_until_stopped(client: Client, backstop: float, *, clock: Callable[[], float],
                          sleep: Callable[[float], None]) -> None:
    """Renvoie purge_release jusqu'à l'arrêt, au plus tard jusqu'à l'arrêt firmware."""
    while True:
        try:
            client("POST", "/action", {"action": "purge_release"})
            if cycle_state(client) != "purge":
                return
        except RuntimeError:
            pass
        if clock() >= backstop:
            raise RuntimeError("purge toujours active après purge_release, l'arrêter depuis la machine")
        sleep(POLL_INTERVAL_S)


def wait_for_stop(client: Client, duration_s: float, backstop: float, firmware_timed: bool, *,
                  clock: Callable[[], float], sleep: Callable[[float], None]) -> None:
    deadline = clock() + duration_s + (STOP_GRACE_S if firmware_timed else 0.0)
    while clock() < deadline:
        sleep(min(POLL_INTERVAL_S, max(0.0, deadline - clock())))
        try:
            if cycle_state(client) != "purge":
                return
        except RuntimeError:
            continue  # télémétrie perdue : le firmware arrête la purge seul
    if firmware_timed:
        print("Purge encore active après sa durée, envoi de purge_release", file=sys.stderr, flush=True)
    release_until_stopped(client, backstop, clock=clock, sleep=sleep)


def purge(duration_s: float, client: Client, *, sleep: Callable[[float], None] = time.sleep,
          clock: Callable[[], float] = time.monotonic) -> None:
    if not math.isfinite(duration_s) or not 0 < duration_s <= 60:
        raise ValueError("la durée doit être comprise entre 0 et 60 s (plafond de purge.max_s)")
    timeout_s = max(5, 5 * math.ceil(duration_s / 5))
    firmware_timed = timeout_s == duration_s

    config = client("GET", "/config", None)
    version = config.get("version")
    purge_config = config.get("purge")
    max_s = purge_config.get("max_s") if isinstance(purge_config, dict) else None
    if type(version) is not int or type(max_s) is not int:
        raise RuntimeError("GET /config: version ou purge.max_s absent ou invalide")

    before = client("GET", "/telemetry", None)
    cycle = before.get("brew", {}).get("state")
    if cycle not in ("idle", "finished"):
        raise RuntimeError(f"purge impossible : cycle actuel {cycle!r}")
    describe_telemetry("Avant", before)

    try:
        if max_s != timeout_s:
            set_purge_max_s(client, version, timeout_s)
        try:
            press(client)
            # Arrêt firmware attendu au plus tard ici, marge comprise.
            backstop = clock() + timeout_s + STOP_GRACE_S
            stop_text = "arrêt par le firmware" if firmware_timed else f"arrêt par purge_release, firmware à {timeout_s} s"
            print(f"Purge démarrée pour {duration_s:g} s ({stop_text})", flush=True)
            wait_for_stop(client, duration_s, backstop, firmware_timed, clock=clock, sleep=sleep)
        except KeyboardInterrupt:
            release_until_stopped(client, clock() + timeout_s + STOP_GRACE_S, clock=clock, sleep=sleep)
            raise
        print("Purge arrêtée", flush=True)
    finally:
        if max_s != timeout_s:
            try:
                set_purge_max_s(client, version, max_s)
            except RuntimeError as error:
                print(f"purge.max_s reste à {timeout_s} s, le remettre à {max_s} s : {error}",
                      file=sys.stderr, flush=True)

    describe_telemetry("Après", client("GET", "/telemetry", None))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("seconds", type=float, help="durée de la purge en secondes (60 au plus) ; exacte si multiple de 5")
    parser.add_argument("--http", help="URL de base (sinon http://$COFFEEFLOW_IP)")
    args = parser.parse_args()
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

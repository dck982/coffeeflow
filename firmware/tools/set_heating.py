#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Active ou désactive la chauffe de la chaudière dans /config.

Exemple :
    COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
        uv run firmware/tools/set_heating.py true
"""

from __future__ import annotations

import argparse
import os
import sys
from typing import Any, Callable

from record_heating import confirmed_heating, request_json


def set_heating(enabled: bool,
                client: Callable[[str, str, dict[str, Any] | None], dict[str, Any]]) -> None:
    config = client("GET", "/config", None)
    version = config.get("version")
    if type(version) is not int:
        raise RuntimeError("GET /config: version absente ou invalide")
    response = client("POST", "/config", {"version": version, "heating": {"enabled": enabled}})
    if not confirmed_heating(response, enabled):
        raise RuntimeError("POST /config: état de chauffe non confirmé")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("enabled", choices=("true", "false"), help="état souhaité de heating.enabled")
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

    enabled = args.enabled == "true"
    client = lambda method, path, body: request_json(url, token, method, path, body)
    try:
        set_heating(enabled, client)
    except (OSError, RuntimeError, ValueError, TypeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"heating.enabled={'true' if enabled else 'false'} confirmé")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

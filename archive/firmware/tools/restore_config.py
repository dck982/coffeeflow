#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Restaure une configuration CoffeeFlow depuis un fichier JSON."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


def base_url(value: str | None, parser: argparse.ArgumentParser) -> str:
    if value:
        return value.rstrip("/")
    if address := os.environ.get("COFFEEFLOW_IP"):
        return f"http://{address}"
    parser.error("--http ou COFFEEFLOW_IP est requis")
    raise AssertionError


def read_config(path: Path) -> dict[str, Any]:
    try:
        payload: Any = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise RuntimeError(f"JSON invalide dans {path}: {error}") from error
    if not isinstance(payload, dict) or not isinstance(payload.get("version"), int):
        raise RuntimeError(f"configuration inconnue ou incompatible: {path}")
    return payload


def restore_config(url: str, token: str, config: dict[str, Any]) -> dict[str, Any]:
    data = json.dumps(config).encode("utf-8")
    request = Request(
        f"{url}/config",
        data=data,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/json",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urlopen(request, timeout=20) as response:
            payload: Any = json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"POST /config: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"POST /config: {error.reason}") from error
    if not isinstance(payload, dict) or not isinstance(payload.get("version"), int):
        raise RuntimeError("réponse /config inconnue ou incompatible")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path, help="fichier JSON produit par save_config.py")
    parser.add_argument("--http", help="base URL (sinon http://$COFFEEFLOW_IP)")
    args = parser.parse_args()
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("COFFEEFLOW_HTTP_TOKEN est requis")

    try:
        response = restore_config(base_url(args.http, parser), token, read_config(args.config))
    except (OSError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"configuration restaurée (version {response['version']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

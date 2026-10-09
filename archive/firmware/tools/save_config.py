#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Sauvegarde la configuration CoffeeFlow dans configs/.

Exemple :
    COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
        uv run firmware/tools/save_config.py
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CONFIG_DIR = PROJECT_ROOT / "configs"


def base_url(value: str | None, parser: argparse.ArgumentParser) -> str:
    if value:
        return value.rstrip("/")
    if address := os.environ.get("COFFEEFLOW_IP"):
        return f"http://{address}"
    parser.error("--http ou COFFEEFLOW_IP est requis")
    raise AssertionError


def load_config(url: str, token: str) -> dict[str, Any]:
    request = Request(
        f"{url}/config",
        headers={"Authorization": f"Bearer {token}", "Accept": "application/json"},
        method="GET",
    )
    try:
        with urlopen(request, timeout=20) as response:
            payload: Any = json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"GET /config: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"GET /config: {error.reason}") from error
    if not isinstance(payload, dict) or not isinstance(payload.get("version"), int):
        raise RuntimeError("réponse /config inconnue ou incompatible")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--http", help="base URL (sinon http://$COFFEEFLOW_IP)")
    parser.add_argument("--config-dir", type=Path, default=DEFAULT_CONFIG_DIR,
                        help=f"répertoire de sortie (défaut : {DEFAULT_CONFIG_DIR})")
    args = parser.parse_args()
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("COFFEEFLOW_HTTP_TOKEN est requis")

    try:
        config = load_config(base_url(args.http, parser), token)
        output = args.config_dir / f"{datetime.now():%y%m%d-%H%M%S}.json"
        if output.exists():
            raise RuntimeError(f"le fichier existe déjà : {output}")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    except (OSError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"configuration écrite dans {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

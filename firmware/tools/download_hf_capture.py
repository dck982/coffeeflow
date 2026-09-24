#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Télécharge la dernière capture haute fréquence CoffeeFlow.

Exemple :
    COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
        uv run firmware/tools/download_hf_capture.py
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
DEFAULT_CAPTURE_DIR = PROJECT_ROOT / "captures"


def base_url(value: str | None, parser: argparse.ArgumentParser) -> str:
    if value:
        return value.rstrip("/")
    if address := os.environ.get("COFFEEFLOW_IP"):
        return f"http://{address}"
    parser.error("--http ou COFFEEFLOW_IP est requis")


def download_capture(url: str, token: str) -> dict[str, Any]:
    request = Request(
        f"{url}/hf-capture?view=both",
        headers={"Authorization": f"Bearer {token}", "Accept": "application/json"},
        method="GET",
    )
    try:
        with urlopen(request, timeout=20) as response:
            payload: Any = json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"GET /hf-capture: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"GET /hf-capture: {error.reason}") from error
    if not isinstance(payload, dict) or payload.get("schema") not in ("coffeeflow.hf_capture.v1", "coffeeflow.hf_capture.v2") or not isinstance(payload.get("samples"), list):
        raise RuntimeError("réponse /hf-capture inconnue ou incompatible")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--http", help="base URL (sinon http://$COFFEEFLOW_IP)")
    parser.add_argument("--capture-dir", type=Path, default=DEFAULT_CAPTURE_DIR, help=f"répertoire de sortie (défaut : {DEFAULT_CAPTURE_DIR})")
    args = parser.parse_args()
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("COFFEEFLOW_HTTP_TOKEN est requis")

    try:
        capture = download_capture(base_url(args.http, parser), token)
        output = args.capture_dir / f"{datetime.now():%y%m%d-%H%M%S}.json"
        if output.exists():
            raise RuntimeError(f"le fichier existe déjà : {output}")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(capture, indent=2) + "\n", encoding="utf-8")
    except (OSError, RuntimeError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"capture écrite dans {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

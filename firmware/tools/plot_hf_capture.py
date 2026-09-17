#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["matplotlib>=3.8"]
# ///
"""Télécharge et trace la dernière capture haute fréquence CoffeeFlow.

Exemple :
    COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=192.168.2.196 \
        uv run firmware/tools/plot_hf_capture.py --output capture.png
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

import matplotlib.pyplot as plt


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
            payload = json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"GET /hf-capture: HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"GET /hf-capture: {error.reason}") from error
    if payload.get("schema") != "coffeeflow.hf_capture.v1" or not isinstance(payload.get("samples"), list):
        raise RuntimeError("réponse /hf-capture inconnue ou incompatible")
    return payload


def values(samples: list[dict[str, Any]], key: str) -> list[float]:
    return [float(sample.get(key, 0.0)) for sample in samples]


def plot(capture: dict[str, Any]) -> plt.Figure:
    samples: list[dict[str, Any]] = capture["samples"]
    if not samples:
        raise RuntimeError("la capture ne contient aucun échantillon")
    elapsed_s = [float(sample["t_ms"]) / 1000.0 for sample in samples]
    pressure, temperature = values(samples, "pressure_bar"), values(samples, "temperature_c")
    flow, volume, weight = values(samples, "flow_ml_s"), values(samples, "volume_ml"), values(samples, "weight_g")
    commanded, reported = values(samples, "pump_pct_commanded"), values(samples, "pump_pct_reported")

    figure, axes = plt.subplots(4, 1, figsize=(13, 10), sharex=True, layout="constrained")
    duration_s = (int(capture["ended_at_us"]) - int(capture["started_at_us"])) / 1_000_000.0
    figure.suptitle(f"CoffeeFlow · {capture['origin']} · {duration_s:.1f} s · {capture['sample_count']} échantillons")

    axes[0].plot(elapsed_s, pressure, color="tab:red", label="pression")
    axes[0].set_ylabel("bar")
    pump_axis = axes[0].twinx()
    pump_axis.step(elapsed_s, commanded, where="post", color="tab:blue", linestyle="--", label="pompe demandée")
    pump_axis.step(elapsed_s, reported, where="post", color="tab:blue", label="pompe rapportée")
    pump_axis.set_ylabel("pompe (%)")
    axes[0].legend(loc="upper left")
    pump_axis.legend(loc="upper right")

    axes[1].plot(elapsed_s, flow, color="tab:green", label="débit")
    axes[1].set_ylabel("ml/s")
    volume_axis = axes[1].twinx()
    volume_axis.plot(elapsed_s, volume, color="tab:olive", label="volume")
    volume_axis.set_ylabel("ml")
    axes[1].legend(loc="upper left")
    volume_axis.legend(loc="upper right")

    axes[2].plot(elapsed_s, temperature, color="tab:orange")
    axes[2].set_ylabel("température (°C)")
    axes[3].plot(elapsed_s, weight, color="tab:purple")
    axes[3].set_ylabel("poids (g)")
    axes[3].set_xlabel("temps depuis SET dimmer>0 (s)")
    for axis in axes:
        axis.grid(True, alpha=0.25)
    return figure


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--http", help="base URL (sinon http://$COFFEEFLOW_IP)")
    parser.add_argument("--output", type=Path, help="fichier image de sortie (PNG, PDF, SVG…)")
    parser.add_argument("--record-json", type=Path, help="copie du JSON téléchargé")
    parser.add_argument("--no-show", action="store_true", help="ne pas ouvrir la fenêtre matplotlib")
    args = parser.parse_args()
    token = os.environ.get("COFFEEFLOW_HTTP_TOKEN")
    if not token:
        parser.error("COFFEEFLOW_HTTP_TOKEN est requis")

    try:
        capture = download_capture(base_url(args.http, parser), token)
        if args.record_json:
            args.record_json.parent.mkdir(parents=True, exist_ok=True)
            args.record_json.write_text(json.dumps(capture, indent=2) + "\n", encoding="utf-8")
        figure = plot(capture)
    except RuntimeError as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        figure.savefig(args.output, dpi=160)
        print(f"graphe écrit dans {args.output}")
    if not args.no_show:
        plt.show()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

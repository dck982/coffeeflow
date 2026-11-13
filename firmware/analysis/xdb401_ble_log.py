#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["bleak>=0.22"]
# ///
"""Enregistre en BLE la pression de la sonde XDB401 (Core2, firmware/xdb401).

Exemple :
    uv run firmware/analysis/xdb401_ble_log.py
    uv run firmware/analysis/xdb401_ble_log.py --duration 120

Écrit une ligne CSV par trame (10 Hz) dans captures/xdb401-<date>.csv jusqu'à
Ctrl-C ou --duration, et une ligne d'état par seconde sur la console. Se
reconnecte seul si la liaison tombe.

Colonnes : unix_ms (réception sur le Mac), seq, probe_ms (temps sonde modulo
65 536), raw (code signé 24 bits), bar (raw × 16 / 2^23, sans zéro), celsius
(corps de la sonde), status (0 sans mesure, 1 mesure).
"""

from __future__ import annotations

import argparse
import asyncio
import csv
import struct
import sys
import time
from datetime import datetime
from pathlib import Path

from bleak import BleakClient, BleakScanner

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CAPTURE_DIR = PROJECT_ROOT / "captures"

# Repris de firmware/xdb401/main/xdb401_ble.hpp.
SERVICE_UUID = "33ebc0eb-99ef-44b7-bc7a-482d582acac4"
FRAME_UUID = "29f79e86-5891-4811-9310-56d44f05ce02"
FRAME = struct.Struct("<HHihBB")
FULL_SCALE_BAR = 16.0
RECONNECT_DELAY_S = 2.0


def decode(data: bytes) -> tuple[int, int, int, float, float, int]:
    seq, probe_ms, raw, centi_c, status, _ = FRAME.unpack_from(data)
    return seq, probe_ms, raw, raw * FULL_SCALE_BAR / 2**23, centi_c / 100, status


async def run(output: Path, duration: float | None) -> None:
    deadline = None if duration is None else time.monotonic() + duration
    with output.open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["unix_ms", "seq", "probe_ms", "raw", "bar", "celsius", "status"])
        last_print = 0.0

        def on_frame(_: object, data: bytearray) -> None:
            nonlocal last_print
            if len(data) < FRAME.size:
                return
            seq, probe_ms, raw, bar, celsius, status = decode(bytes(data))
            writer.writerow([int(time.time() * 1000), seq, probe_ms, raw, f"{bar:.4f}", f"{celsius:.2f}", status])
            now = time.monotonic()
            if now - last_print >= 1.0:
                last_print = now
                f.flush()
                state = f"{bar:7.3f} bar  {celsius:5.2f} °C" if status == 1 else "sans mesure"
                print(f"{datetime.now():%H:%M:%S}  {state}", flush=True)

        while deadline is None or time.monotonic() < deadline:
            device = await BleakScanner.find_device_by_filter(
                lambda _, adv: SERVICE_UUID in adv.service_uuids, timeout=10.0
            )
            if device is None:
                print("sonde XDB401 introuvable, nouvel essai", file=sys.stderr)
                continue
            try:
                async with BleakClient(device) as client:
                    print(f"connecté à {device.name or device.address}", file=sys.stderr)
                    await client.start_notify(FRAME_UUID, on_frame)
                    while client.is_connected and (deadline is None or time.monotonic() < deadline):
                        await asyncio.sleep(0.2)
            except Exception as exc:  # noqa: BLE001 — toute coupure mène à une reconnexion
                print(f"liaison perdue : {exc}", file=sys.stderr)
            if deadline is None or time.monotonic() < deadline:
                await asyncio.sleep(RECONNECT_DELAY_S)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--duration", type=float, help="durée en secondes (défaut : jusqu'à Ctrl-C)")
    parser.add_argument("--output", type=Path, help="fichier CSV (défaut : captures/xdb401-<date>.csv)")
    args = parser.parse_args()
    CAPTURE_DIR.mkdir(exist_ok=True)
    output = args.output or CAPTURE_DIR / f"xdb401-{datetime.now():%y%m%d-%H%M%S}.csv"
    print(f"enregistrement dans {output}", file=sys.stderr)
    try:
        asyncio.run(run(output, args.duration))
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()

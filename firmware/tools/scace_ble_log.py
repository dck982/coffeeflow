#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["bleak>=0.22"]
# ///
"""Enregistre en BLE les mesures de la sonde SCACE (Core2), pour la calibration.

Exemple :
    uv run firmware/tools/scace_ble_log.py
    uv run firmware/tools/scace_ble_log.py --duration 600 --window 120

Cherche le périphérique qui annonce le service SCACE, s'abonne à la trame de
mesure (10 Hz) et écrit une ligne CSV par trame jusqu'à Ctrl-C (ou
--duration). Se reconnecte seul si la liaison tombe. Le 5 V du pont s'allume
avec la touche A du Core2 ; sans pont, la sonde envoie une trame « sans
mesure » par seconde.

Colonnes : unix_ms (réception sur le Mac), seq, probe_ms (temps sonde modulo
65 536), a0, a1 (codes bruts), celsius (constantes de la sonde, vide sans
température), status. Le rapport a1 / (a0 − a1) vaut R_ntc / R_fixe.

Toutes les 2 s, une ligne d'état donne la moyenne de ce rapport et de la
température sur la fenêtre glissante, l'écart-type et la dérive : le point de
glace est atteint quand la dérive reste sous 0,01 K/min.

Les captures sont écrites par défaut dans captures/ (ignoré par Git).
"""

from __future__ import annotations

import argparse
import asyncio
import csv
import math
import struct
import sys
import time
from collections import deque
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import TextIO


PROJECT_ROOT = Path(__file__).resolve().parents[2]
CAPTURE_DIR = PROJECT_ROOT / "captures"

# Repris de firmware/common/include/common/scace_ble.hpp ; un test vérifie
# qu'ils n'ont pas divergé.
SERVICE_UUID = "5017120a-7fa7-46d8-a22e-61b24b1201fe"
FRAME_UUID = "84fef574-1171-4c9a-b221-6f17aea79a38"

FRAME = struct.Struct("<HHhhhBB")
NO_TEMPERATURE = -32768
STATUS_NAMES = {0: "sans-mesure", 1: "ok", 2: "ouverte", 3: "court-circuit"}
STATUS_OK = 1

STATUS_INTERVAL_S = 2.0
RECONNECT_DELAY_S = 2.0
CSV_HEADER = ["unix_ms", "seq", "probe_ms", "a0", "a1", "celsius", "status"]


@dataclass(frozen=True)
class Frame:
    seq: int
    probe_ms: int
    a0: int
    a1: int
    celsius: float | None
    status: int

    @property
    def ratio(self) -> float | None:
        """R_ntc / R_fixe, si la NTC est lue."""
        if self.status != STATUS_OK or self.a0 <= self.a1:
            return None
        return self.a1 / (self.a0 - self.a1)


def decode_frame(data: bytes) -> Frame:
    if len(data) < FRAME.size:
        raise ValueError(f"trame de {len(data)} octets, {FRAME.size} attendus")
    seq, probe_ms, a0, a1, centi_c, status, _ = FRAME.unpack_from(data)
    if status not in STATUS_NAMES:
        raise ValueError(f"état inconnu : {status}")
    celsius = None if centi_c == NO_TEMPERATURE else centi_c / 100
    return Frame(seq, probe_ms, a0, a1, celsius, status)


def lost_frames(previous_seq: int | None, seq: int) -> int:
    if previous_seq is None:
        return 0
    return (seq - previous_seq - 1) & 0xFFFF


def slope_per_min(points: list[tuple[float, float]]) -> float | None:
    """Pente par régression linéaire, en unités par minute."""
    if len(points) < 2:
        return None
    n = len(points)
    mean_t = sum(t for t, _ in points) / n
    mean_v = sum(v for _, v in points) / n
    var_t = sum((t - mean_t) ** 2 for t, _ in points)
    if var_t == 0:
        return None
    cov = sum((t - mean_t) * (v - mean_v) for t, v in points)
    return cov / var_t * 60


def stdev(values: list[float]) -> float | None:
    if len(values) < 2:
        return None
    mean = sum(values) / len(values)
    return math.sqrt(sum((v - mean) ** 2 for v in values) / (len(values) - 1))


class Recorder:
    def __init__(self, output: TextIO, window_s: float) -> None:
        self.writer = csv.writer(output)
        self.output = output
        self.window_s = window_s
        self.window: deque[tuple[float, float, float]] = deque()
        self.previous_seq: int | None = None
        self.frames = 0
        self.lost = 0
        self.rejected = 0
        self.last: Frame | None = None
        self.writer.writerow(CSV_HEADER)
        output.flush()

    def reconnected(self) -> None:
        # Le trou pendant la déconnexion n'est pas une perte de notification.
        self.previous_seq = None

    def on_data(self, received_s: float, data: bytes) -> None:
        try:
            frame = decode_frame(data)
        except ValueError:
            self.rejected += 1
            return
        self.lost += lost_frames(self.previous_seq, frame.seq)
        self.previous_seq = frame.seq
        self.frames += 1
        self.last = frame
        celsius = "" if frame.celsius is None else f"{frame.celsius:.2f}"
        self.writer.writerow([round(received_s * 1000), frame.seq, frame.probe_ms,
                              frame.a0, frame.a1, celsius, STATUS_NAMES[frame.status]])
        self.output.flush()
        ratio = frame.ratio
        if ratio is not None and frame.celsius is not None:
            self.window.append((received_s, frame.celsius, ratio))
        while self.window and received_s - self.window[0][0] > self.window_s:
            self.window.popleft()

    def status_line(self) -> str:
        frame = self.last
        if frame is None:
            return "en attente de trames"
        parts = [f"{STATUS_NAMES[frame.status]}"]
        if frame.celsius is not None:
            parts[0] = f"{frame.celsius:.2f} °C"
        parts.append(f"a0 {frame.a0} / a1 {frame.a1}")
        if len(self.window) >= 2:
            t0 = self.window[0][0]
            temps = [c for _, c, _ in self.window]
            ratios = [r for _, _, r in self.window]
            mean_c = sum(temps) / len(temps)
            mean_r = sum(ratios) / len(ratios)
            drift = slope_per_min([(t - t0, c) for t, c, _ in self.window])
            span = self.window[-1][0] - t0
            parts.append(f"{span:.0f} s : moy {mean_c:.3f} °C, σ {stdev(temps):.3f} K,"
                         f" dérive {drift:+.3f} K/min, R/R_fixe {mean_r:.5f}")
        parts.append(f"{self.lost} perdue(s)")
        return " / ".join(parts)


async def find_device(address: str | None, timeout_s: float):
    from bleak import BleakScanner

    if address:
        return await BleakScanner.find_device_by_address(address, timeout=timeout_s)
    return await BleakScanner.find_device_by_filter(
        lambda _device, adv: SERVICE_UUID in (uuid.lower() for uuid in adv.service_uuids),
        timeout=timeout_s,
    )


async def record(recorder: Recorder, *, address: str | None, duration_s: float | None,
                 scan_timeout_s: float) -> str:
    from bleak import BleakClient
    from bleak.exc import BleakError

    loop = asyncio.get_running_loop()
    deadline = None if duration_s is None else loop.time() + duration_s

    def remaining() -> float | None:
        return None if deadline is None else max(0.0, deadline - loop.time())

    def on_notify(_sender, data: bytearray) -> None:
        recorder.on_data(time.time(), bytes(data))

    async def report() -> None:
        while True:
            await asyncio.sleep(STATUS_INTERVAL_S)
            print(recorder.status_line(), flush=True)

    while remaining() != 0.0:
        print("recherche de la sonde SCACE…", flush=True)
        device = await find_device(address, scan_timeout_s)
        if device is None:
            print(f"aucune sonde trouvée en {scan_timeout_s:.0f} s ; Core2 allumé ?", flush=True)
            continue
        disconnected = asyncio.Event()
        try:
            async with BleakClient(device, disconnected_callback=lambda _c: disconnected.set()) as client:
                recorder.reconnected()
                await client.start_notify(FRAME_UUID, on_notify)
                print(f"connecté à {device.name or device.address}", flush=True)
                reporter = asyncio.create_task(report())
                try:
                    await asyncio.wait_for(disconnected.wait(), timeout=remaining())
                except asyncio.TimeoutError:
                    return "duration"
                finally:
                    reporter.cancel()
        except (BleakError, OSError, asyncio.TimeoutError) as error:
            print(f"liaison BLE : {error}", flush=True)
        if remaining() != 0.0:
            print("liaison perdue, reconnexion…", flush=True)
            await asyncio.sleep(RECONNECT_DELAY_S)
    return "duration"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", type=Path, help="fichier CSV de sortie")
    parser.add_argument("--duration", type=float, metavar="SECONDS",
                        help="durée en secondes (défaut : jusqu'à Ctrl-C)")
    parser.add_argument("--window", type=float, default=60.0, metavar="SECONDS",
                        help="fenêtre des moyennes et de la dérive (défaut : 60 s)")
    parser.add_argument("--address", help="adresse BLE (identifiant CoreBluetooth sur Mac) ; sinon par service")
    parser.add_argument("--scan-timeout", type=float, default=10.0, metavar="SECONDS")
    args = parser.parse_args()
    for name in ("duration", "window", "scan_timeout"):
        value = getattr(args, name)
        if value is not None and (not math.isfinite(value) or value <= 0):
            parser.error(f"--{name.replace('_', '-')} doit être un nombre de secondes positif")

    output = args.output or CAPTURE_DIR / f"scace-{datetime.now():%Y%m%d-%H%M%S}.csv"
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        handle = output.open("x", encoding="utf-8", newline="")
    except OSError as error:
        print(f"erreur : {error}", file=sys.stderr)
        return 1
    with handle:
        recorder = Recorder(handle, args.window)
        try:
            reason = asyncio.run(record(recorder, address=args.address, duration_s=args.duration,
                                        scan_timeout_s=args.scan_timeout))
        except KeyboardInterrupt:
            reason = "interrupted"
    print(f"relevé écrit dans {output} ({recorder.frames} trames, {recorder.lost} perdue(s),"
          f" {recorder.rejected} rejetée(s), {reason})")
    # Ctrl-C est la façon normale d'arrêter cet enregistrement.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

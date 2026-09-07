"""Enregistrement et relecture de trames — ce format devient la fixture des
tests d'algorithme en phase 6 : on enregistre un vrai shot, on le rejoue sur
le Mac (docs/firmware-implementation.md, phase 1).

Un fichier est du JSON Lines, une trame par ligne :
    {"t": <secondes écoulées depuis la première trame>, "id": <int>, "data": "<hex>"}
Le `t` relatif (pas un timestamp absolu) est ce qui permet de rejouer un
shot à son rythme d'origine indépendamment de la date d'enregistrement.
"""

from __future__ import annotations

import json
import time
from collections.abc import Iterator
from pathlib import Path

from .framing import RawFrame
from .transport import Transport


def record(transport: Transport, path: Path, on_frame=None) -> None:
    """Enregistre indéfiniment jusqu'à KeyboardInterrupt (Ctrl-C)."""
    start = None
    with open(path, "w", encoding="utf-8") as f:
        while True:
            result = transport.recv(timeout=1.0)
            if result is None:
                continue
            ts, frame = result
            if start is None:
                start = ts
            entry = {"t": ts - start, "id": frame.can_id, "data": frame.data.hex()}
            f.write(json.dumps(entry) + "\n")
            f.flush()
            if on_frame is not None:
                on_frame(ts, frame)


def replay(path: Path) -> Iterator[tuple[float, RawFrame]]:
    """Relit un fichier enregistré, sans matériel. Renvoie (t_relatif, trame)."""
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            frame = RawFrame(can_id=entry["id"], data=bytes.fromhex(entry["data"]))
            yield entry["t"], frame


def replay_to_transport(path: Path, transport: Transport, realtime: bool = True) -> None:
    """Rejoue un enregistrement sur un vrai transport, au rythme d'origine
    par défaut — utile pour rejouer un shot vers une carte réelle."""
    previous_t = 0.0
    for t, frame in replay(path):
        if realtime:
            delay = t - previous_t
            if delay > 0:
                time.sleep(delay)
            previous_t = t
        transport.send(frame)

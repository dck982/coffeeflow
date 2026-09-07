"""Deux transports, un seul décodeur (docs/firmware-implementation.md,
phase 1) : l'USB série (phases 2-3) cadre en COBS, le WebSocket (phase 6)
transporte le PDU nu, un message = une trame. Le reste de l'outil ne
travaille qu'avec RawFrame, jamais avec les octets du fil.

Les dépendances (`pyserial`, `websockets`) sont importées à la demande :
décoder un fichier enregistré ou tester les modules protocole/messages ne
doit pas exiger un transport installé.
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod

from .framing import RawFrame, StreamDecoder, encode_framed, encode_pdu, decode_pdu


class Transport(ABC):
    @abstractmethod
    def send(self, frame: RawFrame) -> None: ...

    @abstractmethod
    def recv(self, timeout: float | None = None) -> tuple[float, RawFrame] | None:
        """Renvoie (horodatage, trame), ou None si `timeout` s'écoule sans trame."""

    def close(self) -> None:
        pass

    def __enter__(self) -> "Transport":
        return self

    def __exit__(self, *exc_info) -> None:
        self.close()


class SerialTransport(Transport):
    """USB CDC — cadrage COBS + 0x00, voir framing.py."""

    def __init__(self, port: str, baudrate: int = 115200):
        import serial  # pyserial

        # CDC ACM ignore le débit : la valeur est cosmétique, ne pas
        # dimensionner un flash comme si le lien était vraiment à ce débit.
        self._ser = serial.Serial(port, baudrate, timeout=0.05)
        self._decoder = StreamDecoder()

    def send(self, frame: RawFrame) -> None:
        self._ser.write(encode_framed(frame))

    def recv(self, timeout: float | None = None) -> tuple[float, RawFrame] | None:
        deadline = None if timeout is None else time.monotonic() + timeout
        while True:
            frame = self._decoder.pop_frame()
            if frame is not None:
                return time.time(), frame
            chunk = self._ser.read(256)
            if not chunk:
                if deadline is not None and time.monotonic() >= deadline:
                    return None
                continue
            self._decoder.push_bytes(chunk)

    @property
    def dropped_count(self) -> int:
        return self._decoder.dropped_count

    def close(self) -> None:
        self._ser.close()


class WebSocketTransport(Transport):
    """Miroir du trafic CAN par WebSocket (phase 6) — même PDU qu'en série,
    sans COBS : un message WebSocket est déjà une trame complète."""

    def __init__(self, url: str):
        from websockets.sync.client import connect

        self._ws = connect(url)

    def send(self, frame: RawFrame) -> None:
        self._ws.send(encode_pdu(frame))

    def recv(self, timeout: float | None = None) -> tuple[float, RawFrame] | None:
        try:
            message = self._ws.recv(timeout=timeout)
        except TimeoutError:
            return None
        if isinstance(message, str):
            return None  # trafic CAN attendu en binaire uniquement
        frame = decode_pdu(message)
        if frame is None:
            return None
        return time.time(), frame

    def close(self) -> None:
        self._ws.close()

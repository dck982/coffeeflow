"""Ligne de commande de l'outil Mac — docs/firmware-implementation.md,
phase 1. Un seul décodeur, deux transports (--port pour l'USB série,
--ws pour le WebSocket de phase 6).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .decoder import decode_frame
from .flash_client import FlashProgress, flash
from .framing import RawFrame
from .messages import PongPayload, ReqStatusPayload, SetPayload
from .protocol import CanId, Dest, MessageType, Node, encode_can_id
from .recorder import record, replay, replay_to_transport
from .transport import SerialTransport, Transport, WebSocketTransport


def _open_transport(args: argparse.Namespace) -> Transport:
    if args.ws:
        return WebSocketTransport(args.ws)
    if args.port:
        return SerialTransport(args.port, args.baudrate)
    raise SystemExit("préciser --port (USB série) ou --ws (WebSocket)")


def _add_transport_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--port", help="port série, ex. /dev/tty.usbmodemXXXX")
    parser.add_argument("--baudrate", type=int, default=115200)
    parser.add_argument("--ws", help="URL WebSocket, ex. ws://coffeeflow.local/ws")


def cmd_monitor(args: argparse.Namespace) -> int:
    with _open_transport(args) as transport:
        try:
            while True:
                result = transport.recv(timeout=1.0)
                if result is None:
                    continue
                ts, frame = result
                print(decode_frame(ts, frame))
        except KeyboardInterrupt:
            return 0


def cmd_send(args: argparse.Namespace) -> int:
    src = Node[args.src.upper()]
    dest = Dest[args.dest.upper()]

    if args.message == "ping":
        can_id = encode_can_id(CanId(MessageType.PING, dest, src))
        payload = b""
    elif args.message == "pong":
        can_id = encode_can_id(CanId(MessageType.PONG, dest, src))
        payload = PongPayload(node=src, version_major=0, version_minor=1, version_patch=0, uptime_s=0).pack()
    elif args.message == "stop":
        can_id = encode_can_id(CanId(MessageType.STOP, dest, src))
        payload = b""
    elif args.message == "reset":
        can_id = encode_can_id(CanId(MessageType.RESET, dest, src))
        payload = b""
    elif args.message == "set":
        can_id = encode_can_id(CanId(MessageType.SET, dest, src))
        payload = SetPayload(
            set_ssr=args.ssr is not None,
            set_dimmer=args.dimmer is not None,
            ssr=bool(args.ssr),
            dimmer=args.dimmer or 0,
            ttl_ms=args.ttl_ms,
        ).pack()
    elif args.message == "reqstatus":
        can_id = encode_can_id(CanId(MessageType.REQSTATUS, dest, src))
        payload = ReqStatusPayload(target_type=MessageType[args.target.upper()], period_ms=args.period_ms).pack()
    else:
        raise SystemExit(f"message inconnu: {args.message}")

    with _open_transport(args) as transport:
        transport.send(RawFrame(can_id, payload))
    print(f"envoyé {args.message} id={can_id:#05x} data={payload.hex()}")
    return 0


def cmd_record(args: argparse.Namespace) -> int:
    with _open_transport(args) as transport:
        print(f"enregistrement vers {args.out} — Ctrl-C pour arrêter")
        try:
            record(transport, Path(args.out), on_frame=lambda ts, f: print(decode_frame(ts, f)))
        except KeyboardInterrupt:
            print("\narrêté")
    return 0


def cmd_replay(args: argparse.Namespace) -> int:
    path = Path(args.file)
    if args.port or args.ws:
        with _open_transport(args) as transport:
            replay_to_transport(path, transport, realtime=not args.fast)
        print("relecture envoyée")
        return 0

    for t, frame in replay(path):
        print(decode_frame(t, frame))
    return 0


def cmd_flash(args: argparse.Namespace) -> int:
    image = Path(args.image).read_bytes()
    dest = Dest[args.dest.upper()]
    src = Node[args.src.upper()]

    def on_progress(p: FlashProgress) -> None:
        pct = 100 * p.bytes_sent / p.image_size
        print(f"\rbloc {p.block_number}/{p.block_count} ({pct:5.1f}%)", end="", flush=True)

    with _open_transport(args) as transport:
        try:
            flash(transport, image, src=src, dest=dest, on_progress=on_progress)
        except Exception as exc:  # noqa: BLE001 — rapporté tel quel à l'utilisateur
            print()
            print(f"échec: {exc}", file=sys.stderr)
            return 1
    print("\nfait")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="coffeetool", description="Outil Mac — décodeur/encodeur du protocole CAN coffeeflow")
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("monitor", help="décode le flux en direct")
    _add_transport_args(p)
    p.set_defaults(func=cmd_monitor)

    p = sub.add_parser("send", help="envoie un message à la main")
    _add_transport_args(p)
    p.add_argument("message", choices=["ping", "pong", "stop", "reset", "set", "reqstatus"])
    p.add_argument("--src", default="screen", choices=["screen", "sensors"])
    p.add_argument("--dest", default="broadcast", choices=["broadcast", "screen", "sensors"])
    p.add_argument("--ssr", type=int, choices=[0, 1], default=None, help="SET: 0 ou 1")
    p.add_argument("--dimmer", type=int, default=None, help="SET: 0..100")
    p.add_argument("--ttl-ms", type=int, default=0, help="SET: 0 = défaut (500 ms)")
    p.add_argument("--target", default="status_pressure", choices=["status_pressure", "status_flow", "status_actuators"])
    p.add_argument("--period-ms", type=int, default=0, help="REQSTATUS: 0 = arrêt")
    p.set_defaults(func=cmd_send)

    p = sub.add_parser("record", help="enregistre le flux dans un fichier")
    _add_transport_args(p)
    p.add_argument("out", help="fichier de sortie (JSON Lines)")
    p.set_defaults(func=cmd_record)

    p = sub.add_parser("replay", help="rejoue un fichier enregistré")
    _add_transport_args(p)
    p.add_argument("file", help="fichier à rejouer")
    p.add_argument("--fast", action="store_true", help="sans respecter le rythme d'origine")
    p.set_defaults(func=cmd_replay)

    p = sub.add_parser("flash", help="flashe une image via FLASH_CTRL/FLASH_DATA")
    _add_transport_args(p)
    p.add_argument("image", help="fichier .bin")
    p.add_argument("--dest", default="sensors", choices=["screen", "sensors"])
    p.add_argument("--src", default="screen", choices=["screen", "sensors"])
    p.set_defaults(func=cmd_flash)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())

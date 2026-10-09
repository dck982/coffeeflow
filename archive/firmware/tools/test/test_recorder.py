from coffeetool.framing import RawFrame
from coffeetool.protocol import CanId, Dest, MessageType, Node, encode_can_id
from coffeetool.recorder import replay, replay_to_transport


class _FakeTransport:
    def __init__(self):
        self.sent = []

    def send(self, frame):
        self.sent.append(frame)

    def recv(self, timeout=None):
        return None


def test_replay_roundtrip(tmp_path):
    path = tmp_path / "shot.jsonl"
    frame = RawFrame(can_id=encode_can_id(CanId(MessageType.STATUS_FLOW, Dest.SCREEN, Node.SENSORS)), data=b"\x01\x02\x03\x04\x05\x06\x00")
    path.write_text('{"t": 0.0, "id": %d, "data": "%s"}\n' % (frame.can_id, frame.data.hex()))

    entries = list(replay(path))
    assert len(entries) == 1
    t, out_frame = entries[0]
    assert t == 0.0
    assert out_frame == frame


def test_replay_to_transport_sends_in_order(tmp_path):
    path = tmp_path / "shot.jsonl"
    frame_a = RawFrame(can_id=1, data=b"\x00")
    frame_b = RawFrame(can_id=2, data=b"\x01")
    path.write_text(
        '{"t": 0.0, "id": %d, "data": "%s"}\n{"t": 0.01, "id": %d, "data": "%s"}\n'
        % (frame_a.can_id, frame_a.data.hex(), frame_b.can_id, frame_b.data.hex())
    )

    transport = _FakeTransport()
    replay_to_transport(path, transport, realtime=False)
    assert transport.sent == [frame_a, frame_b]

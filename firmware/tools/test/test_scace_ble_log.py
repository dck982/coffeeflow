import csv
import io
import re
import struct
import unittest
from pathlib import Path

from scace_ble_log import (FRAME_UUID, SERVICE_UUID, LatestTemperature, Recorder, decode_frame,
                           lost_frames, slope_per_min)


HEADER = Path(__file__).resolve().parents[2] / "common" / "include" / "common" / "scace_ble.hpp"


def frame_bytes(seq=0, probe_ms=0, a0=26416, a1=25600, centi_c=3, status=1):
    return struct.pack("<HHhhhBB", seq, probe_ms, a0, a1, centi_c, status, 0)


class ScaceBleLogTest(unittest.TestCase):
    def test_uuids_match_firmware_header(self):
        header = HEADER.read_text(encoding="utf-8")
        self.assertIn(f'kServiceUuid[] = "{SERVICE_UUID}"', header)
        self.assertIn(f'kFrameUuid[] = "{FRAME_UUID}"', header)

    def test_frame_size_matches_firmware_header(self):
        size = int(re.search(r"kFrameSize = (\d+);", HEADER.read_text(encoding="utf-8")).group(1))
        self.assertEqual(len(frame_bytes()), size)

    def test_decode_frame(self):
        frame = decode_frame(frame_bytes(seq=7, probe_ms=65535, a0=26416, a1=-2, centi_c=-150, status=1))
        self.assertEqual((frame.seq, frame.probe_ms, frame.a0, frame.a1), (7, 65535, 26416, -2))
        self.assertEqual(frame.celsius, -1.5)

    def test_decode_frame_without_temperature(self):
        frame = decode_frame(frame_bytes(a0=26416, a1=26415, centi_c=-32768, status=2))
        self.assertIsNone(frame.celsius)
        self.assertIsNone(frame.ratio)

    def test_decode_frame_rejects_short_and_unknown_status(self):
        with self.assertRaises(ValueError):
            decode_frame(frame_bytes()[:11])
        with self.assertRaises(ValueError):
            decode_frame(frame_bytes(status=4))

    def test_decode_frame_ignores_trailing_bytes(self):
        self.assertEqual(decode_frame(frame_bytes(seq=3) + b"\x00\x00").seq, 3)

    def test_ratio_is_r_ntc_over_r_fixed(self):
        self.assertAlmostEqual(decode_frame(frame_bytes(a0=26416, a1=24012)).ratio, 24012 / 2404)

    def test_lost_frames_wraps(self):
        self.assertEqual(lost_frames(None, 5), 0)
        self.assertEqual(lost_frames(4, 5), 0)
        self.assertEqual(lost_frames(65534, 1), 2)

    def test_slope_per_min(self):
        points = [(t, 0.5 * t / 60) for t in range(0, 120, 10)]
        self.assertAlmostEqual(slope_per_min(points), 0.5)
        self.assertIsNone(slope_per_min([(0.0, 1.0)]))

    def test_recorder_writes_csv_and_counts_losses(self):
        output = io.StringIO()
        recorder = Recorder(output, window_s=60)
        recorder.on_data(1000.0, frame_bytes(seq=1, centi_c=3))
        recorder.on_data(1000.1, frame_bytes(seq=3, centi_c=4))
        recorder.on_data(1000.2, frame_bytes(seq=4, a0=0, a1=0, centi_c=-32768, status=0))
        recorder.on_data(1000.3, b"\x00")
        rows = list(csv.reader(io.StringIO(output.getvalue())))
        self.assertEqual(rows[0], ["unix_ms", "seq", "probe_ms", "a0", "a1", "celsius", "status"])
        self.assertEqual(rows[1], ["1000000", "1", "0", "26416", "25600", "0.03", "ok"])
        self.assertEqual(rows[3][5:], ["", "sans-mesure"])
        self.assertEqual((recorder.frames, recorder.lost, recorder.rejected), (3, 1, 1))
        self.assertEqual(len(recorder.window), 2)

    def test_reconnection_gap_is_not_a_loss(self):
        recorder = Recorder(io.StringIO(), window_s=60)
        recorder.on_data(0.0, frame_bytes(seq=1))
        recorder.reconnected()
        recorder.on_data(9.0, frame_bytes(seq=90))
        self.assertEqual(recorder.lost, 0)

    def test_window_drops_old_points_and_reports(self):
        recorder = Recorder(io.StringIO(), window_s=10)
        for i in range(30):
            recorder.on_data(float(i), frame_bytes(seq=i, centi_c=i))
        self.assertLessEqual(recorder.window[-1][0] - recorder.window[0][0], 10)
        line = recorder.status_line()
        self.assertIn("0.29 °C", line)
        self.assertIn("dérive +0.600 K/min", line)


class LatestTemperatureTest(unittest.TestCase):
    def setUp(self):
        self.now = 100.0
        self.latest = LatestTemperature(clock=lambda: self.now)

    def test_nothing_received(self):
        self.assertEqual(self.latest.read(), (None, None))

    def test_fresh_temperature_and_age(self):
        self.latest.on_data(frame_bytes(centi_c=9123))
        self.now += 0.1
        self.assertEqual(self.latest.read(), (91.23, 100))

    def test_stale_temperature_is_dropped_but_age_kept(self):
        self.latest.on_data(frame_bytes(centi_c=9123))
        self.now += 0.5
        self.assertEqual(self.latest.read(), (None, 500))

    def test_frame_without_measurement_clears_temperature(self):
        self.latest.on_data(frame_bytes(centi_c=9123))
        self.latest.on_data(frame_bytes(centi_c=-32768, status=0))
        self.assertEqual(self.latest.read(), (None, 0))

    def test_invalid_frame_is_ignored(self):
        self.latest.on_data(frame_bytes(centi_c=9123))
        self.latest.on_data(b"\x00" * 4)
        self.assertEqual(self.latest.read()[0], 91.23)


if __name__ == "__main__":
    unittest.main()

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from record_probe import record_probe


class Clock:
    def __init__(self):
        self.now = 0.0

    def monotonic(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


def telemetry(a0=26305, a1=1151):
    return {"temperature": {"boiler": {
        "c": 20, "valid": True, "freshness": "fresh", "age_ms": 120,
        "ntc_a0_raw": a0, "ntc_a1_raw": a1}}}


class ProbeTests(unittest.TestCase):
    def run_probe(self, client, **kwargs):
        clock = Clock()
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "probe.json"
            with contextlib.redirect_stdout(io.StringIO()):
                result = record_probe(output, client, clock=clock.monotonic,
                                      sleep=clock.sleep, **kwargs)
            self.assertEqual(json.loads(output.read_text()), result)
        return result

    def test_records_raw_codes_at_two_hertz(self):
        result = self.run_probe(lambda m, p, b: telemetry(), duration_s=2)
        self.assertEqual(result["stop_reason"], "duration")
        self.assertEqual(len(result["samples"]), 4)
        first = result["samples"][0]
        self.assertEqual((first["a0_raw"], first["a1_raw"]), (26305, 1151))
        self.assertEqual(first["age_ms"], 120)

    def test_only_telemetry_is_requested(self):
        calls = []

        def client(method, path, body):
            calls.append((method, path))
            return telemetry()

        self.run_probe(client, duration_s=1)
        self.assertEqual(set(calls), {("GET", "/telemetry")})

    def test_interrupt_keeps_samples(self):
        count = 0

        def client(method, path, body):
            nonlocal count
            count += 1
            if count > 3:
                raise KeyboardInterrupt
            return telemetry()

        result = self.run_probe(client)
        self.assertEqual(result["stop_reason"], "interrupted")
        self.assertEqual(len(result["samples"]), 3)

    def test_missing_codes_are_an_error(self):
        result = self.run_probe(lambda m, p, b: {"temperature": {"boiler": {}}},
                                duration_s=2)
        self.assertEqual(result["stop_reason"], "error")
        self.assertIn("ntc_a0_raw", result["error"])

    def test_refuses_existing_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "probe.json"
            output.write_text("{}")
            with self.assertRaises(FileExistsError):
                record_probe(output, lambda m, p, b: telemetry(), duration_s=1)


if __name__ == "__main__":
    unittest.main()

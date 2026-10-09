import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from record_heating import record_heating


class Clock:
    def __init__(self):
        self.now = 0.0

    def monotonic(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


class MonitorTests(unittest.TestCase):
    def run_monitor(self, *, enabled, monitor_time_s=2, fail_telemetry=False):
        calls = []
        clock = Clock()

        def client(method, path, body):
            calls.append((method, path, body))
            if path == "/config":
                return {"version": 7, "heating": {
                    "enabled": enabled, "brew_temperature_c": 90}}
            if fail_telemetry:
                raise RuntimeError("réseau perdu")
            return {
                "temperature": {"boiler": {
                    "c": 90, "valid": True, "freshness": "fresh"}},
                "heating": {"power_pct": 37.5},
            }

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "heating.json"
            with contextlib.redirect_stdout(io.StringIO()):
                result = record_heating(
                    output, client, monitor_time_s=monitor_time_s,
                    clock=clock.monotonic, sleep=clock.sleep)
            self.assertEqual(json.loads(output.read_text()), result)
        return result, calls

    def test_records_for_full_duration_without_changing_heating(self):
        result, calls = self.run_monitor(enabled=True, monitor_time_s=2)
        self.assertEqual(result["mode"], "monitor")
        self.assertEqual(result["stop_reason"], "monitor_duration")
        self.assertEqual(result["max_duration_s"], 2)
        self.assertEqual(result["target_c"], 90)
        self.assertEqual(len(result["samples"]), 4)
        self.assertEqual(calls[0], ("GET", "/config", None))
        self.assertFalse(any(method == "POST" for method, _, _ in calls))

    def test_records_while_heating_is_disabled(self):
        result, calls = self.run_monitor(enabled=False, monitor_time_s=2)
        self.assertEqual(result["stop_reason"], "monitor_duration")
        self.assertEqual(len(result["samples"]), 4)
        self.assertEqual(calls[0], ("GET", "/config", None))
        self.assertFalse(any(method == "POST" for method, _, _ in calls))

    def test_displays_temperature_every_ten_seconds(self):
        clock = Clock()

        def client(method, path, body):
            if path == "/config":
                return {"version": 7, "heating": {
                    "enabled": False, "brew_temperature_c": 90}}
            return {"temperature": {"boiler": {
                "c": 20, "valid": True, "freshness": "fresh"}},
                "heating": {"power_pct": 37.5}}

        output = io.StringIO()
        with tempfile.TemporaryDirectory() as directory:
            with contextlib.redirect_stdout(output):
                record_heating(
                    Path(directory) / "heating.json", client,
                    monitor_time_s=11, clock=clock.monotonic, sleep=clock.sleep)
        lines = output.getvalue().splitlines()
        self.assertEqual(len(lines), 2)
        self.assertIn("0.0 s : chaudière 20.0 °C / cible 90.0 °C / puissance demandée 37.5 %", lines[0])
        self.assertIn("10.0 s : chaudière 20.0 °C / cible 90.0 °C / puissance demandée 37.5 %", lines[1])

    def test_telemetry_error_keeps_partial_file(self):
        result, calls = self.run_monitor(
            enabled=False, monitor_time_s=2, fail_telemetry=True)
        self.assertEqual(result["stop_reason"], "error")
        self.assertEqual(result["samples"], [])
        self.assertEqual(result["error"], "réseau perdu")
        self.assertEqual(calls, [
            ("GET", "/config", None), ("GET", "/telemetry", None)])


if __name__ == "__main__":
    unittest.main()

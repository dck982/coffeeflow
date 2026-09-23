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


class CaptureTests(unittest.TestCase):
    def run_capture(self, temperatures, *, fail_telemetry=False, max_duration_s=2):
        calls = []
        clock = Clock()
        readings = iter(temperatures)

        def client(method, path, body):
            calls.append((method, path, body))
            if path == "/config" and method == "GET":
                return {"version": 6, "heating": {"enabled": False, "brew_temperature_c": 90}}
            if path == "/config":
                return {"heating": {"enabled": body["heating"]["enabled"]}}
            if fail_telemetry:
                raise RuntimeError("réseau perdu")
            return {"boiler_temperature_c": next(readings),
                    "boiler_temperature_valid": True,
                    "boiler_temperature_freshness": "fresh"}

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "heating.json"
            result = record_heating(output, client, clock=clock.monotonic,
                                    sleep=clock.sleep, max_duration_s=max_duration_s)
            self.assertEqual(json.loads(output.read_text()), result)
        return result, calls

    def test_stops_at_target_and_disables_heating(self):
        result, calls = self.run_capture([20, 89, 90, 120])
        self.assertEqual(result["stop_reason"], "target_reached")
        self.assertEqual(len(result["samples"]), 3)
        self.assertEqual([s["elapsed_s"] for s in result["samples"]], [0, 0.5, 1.0])
        self.assertEqual(calls[-1][2]["heating"]["enabled"], False)
        self.assertTrue(result["heating_disabled"])

    def test_timeout_still_disables_heating(self):
        result, calls = self.run_capture([20] * 10, max_duration_s=1)
        self.assertEqual(result["stop_reason"], "timeout")
        self.assertEqual(len(result["samples"]), 2)
        self.assertEqual(calls[-1][2]["heating"]["enabled"], False)

    def test_telemetry_error_keeps_partial_file_and_disables(self):
        result, calls = self.run_capture([], fail_telemetry=True)
        self.assertEqual(result["stop_reason"], "error")
        self.assertTrue(result["heating_disabled"])
        self.assertEqual(calls[-1][2]["heating"]["enabled"], False)

    def test_disable_failure_is_recorded_after_three_attempts(self):
        attempts = 0
        clock = Clock()

        def client(method, path, body):
            nonlocal attempts
            if method == "GET" and path == "/config":
                return {"version": 6, "heating": {"enabled": False, "brew_temperature_c": 90}}
            if method == "POST" and body["heating"]["enabled"] is True:
                return {"heating": {"enabled": True}}
            if method == "POST":
                attempts += 1
                raise RuntimeError("réseau perdu")
            return {"boiler_temperature_c": 90, "boiler_temperature_valid": True,
                    "boiler_temperature_freshness": "fresh"}

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "heating.json"
            result = record_heating(output, client, clock=clock.monotonic, sleep=clock.sleep)
            self.assertEqual(json.loads(output.read_text()), result)
        self.assertEqual(attempts, 3)
        self.assertFalse(result["heating_disabled"])
        self.assertEqual(result["disable_error"], "réseau perdu")


if __name__ == "__main__":
    unittest.main()

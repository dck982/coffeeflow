import contextlib
import io
import unittest

from purge import purge


class Machine:
    """Écran simulé : la purge s'arrête seule à purge.max_s, figé à l'appui."""

    def __init__(self, max_s=20):
        self.max_s = max_s
        self.now = 0.0
        self.purge_until = None
        self.calls = []
        self.lose = set()  # (méthode, action ou chemin) dont la réponse se perd
        self.release_ignored = False

    def clock(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds

    def state(self):
        if self.purge_until is not None and self.now >= self.purge_until:
            self.purge_until = None
        return "purge" if self.purge_until is not None else "finished"

    def client(self, method, path, body):
        key = body.get("action", path) if body else path
        self.calls.append((method, key, body))
        if path == "/config" and method == "GET":
            return {"version": 2, "purge": {"max_s": self.max_s}}
        if path == "/config":
            self.max_s = body["purge"]["max_s"]
            response = {"version": 2, "purge": {"max_s": self.max_s}}
        elif path == "/telemetry":
            response = {"brew": {"state": self.state()}}
        elif key == "purge_press":
            self.state()
            if self.purge_until is None:
                self.purge_until = self.now + self.max_s
            response = {"ok": True}
        else:
            if not self.release_ignored:
                self.purge_until = None
            response = {"ok": True}
        if (method, key) in self.lose:
            self.lose.discard((method, key))
            raise RuntimeError("réponse Wi-Fi perdue")
        return response

    def actions(self):
        return [key for method, key, _ in self.calls if method == "POST"]

    def run(self, duration):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            purge(duration, self.client, sleep=self.sleep, clock=self.clock)


class PurgeTests(unittest.TestCase):
    def test_firmware_times_the_purge_and_max_s_is_restored(self):
        machine = Machine(max_s=20)
        machine.run(15)
        self.assertEqual(machine.actions(), ["/config", "purge_press", "/config"])
        self.assertEqual([body["purge"]["max_s"] for m, k, body in machine.calls if k == "/config" and m == "POST"],
                         [15, 20])
        self.assertEqual(machine.max_s, 20)
        self.assertGreaterEqual(machine.now, 15)

    def test_config_untouched_when_max_s_already_matches(self):
        machine = Machine(max_s=15)
        machine.run(15)
        self.assertEqual(machine.actions(), ["purge_press"])

    def test_lost_press_response_is_checked_on_telemetry(self):
        machine = Machine()
        machine.lose.add(("POST", "purge_press"))
        machine.run(15)
        self.assertEqual(machine.actions().count("purge_press"), 1)
        self.assertEqual(machine.max_s, 20)

    def test_unapplied_press_is_not_resent(self):
        machine = Machine()
        original = machine.client

        def client(method, path, body):
            if body and body.get("action") == "purge_press":
                machine.calls.append((method, "purge_press", body))
                raise RuntimeError("requête perdue")
            return original(method, path, body)

        machine.client = client
        with self.assertRaisesRegex(RuntimeError, "non démarrée"):
            machine.run(15)
        self.assertEqual(machine.actions().count("purge_press"), 1)
        self.assertEqual(machine.max_s, 20)

    def test_release_is_a_fallback_when_purge_outlives_its_duration(self):
        machine = Machine()
        original = machine.client

        def client(method, path, body):
            if method == "POST" and path == "/config":
                return original(method, path, {"version": 2, "purge": {"max_s": 60}}) | {
                    "purge": {"max_s": body["purge"]["max_s"]}}
            return original(method, path, body)

        machine.client = client
        machine.run(15)
        self.assertIn("purge_release", machine.actions())
        self.assertEqual(machine.state(), "finished")

    def test_stuck_purge_is_reported(self):
        machine = Machine()
        machine.release_ignored = True
        original = machine.client

        def client(method, path, body):
            if method == "POST" and path == "/config":
                return original(method, path, {"version": 2, "purge": {"max_s": 60}}) | {
                    "purge": {"max_s": body["purge"]["max_s"]}}
            return original(method, path, body)

        machine.client = client
        with self.assertRaisesRegex(RuntimeError, "toujours active"):
            machine.run(15)

    def test_interruption_releases(self):
        machine = Machine()

        def interrupt(_seconds):
            raise KeyboardInterrupt

        machine.sleep = interrupt
        with self.assertRaises(KeyboardInterrupt):
            machine.run(15)
        self.assertEqual(machine.actions(), ["/config", "purge_press", "purge_release", "/config"])
        self.assertEqual(machine.max_s, 20)

    def test_other_durations_are_released_with_a_firmware_backstop(self):
        machine = Machine(max_s=20)
        machine.run(8)
        self.assertEqual(machine.actions(), ["/config", "purge_press", "purge_release", "/config"])
        self.assertEqual([body["purge"]["max_s"] for m, k, body in machine.calls if k == "/config" and m == "POST"],
                         [10, 20])
        self.assertAlmostEqual(machine.now, 8)

    def test_lost_release_still_stops_at_the_backstop(self):
        machine = Machine(max_s=20)
        machine.release_ignored = True
        machine.run(8)
        self.assertGreaterEqual(machine.actions().count("purge_release"), 2)
        self.assertEqual(machine.state(), "finished")
        self.assertLessEqual(machine.now, 10 + 1)

    def test_rejects_duration_outside_firmware_range(self):
        for duration in (0, -1, 65, float("nan")):
            with self.assertRaisesRegex(ValueError, "entre 0 et 60"):
                purge(duration, lambda m, p, b: self.fail("aucune requête attendue"))

    def test_refuses_active_cycle(self):
        machine = Machine()
        machine.purge_until = 100
        with self.assertRaisesRegex(RuntimeError, "cycle actuel"):
            machine.run(15)
        self.assertEqual(machine.actions(), [])


if __name__ == "__main__":
    unittest.main()

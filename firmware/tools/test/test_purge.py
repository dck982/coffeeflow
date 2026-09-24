import unittest

from purge import purge


class PurgeTests(unittest.TestCase):
    def test_sends_press_then_release_and_reads_telemetry(self):
        calls = []
        sleeps = []

        def client(method, path, body):
            calls.append((method, path, body))
            if path == "/config":
                return {"purge": {"max_s": 20}}
            if path == "/telemetry":
                return {"cycle": "idle", "boiler_temperature_c": 90, "heating_power_pct": 0}
            return {"ok": True}

        purge(10, client, sleep=sleeps.append)
        self.assertEqual(sleeps, [10])
        self.assertEqual(calls, [
            ("GET", "/config", None),
            ("GET", "/telemetry", None),
            ("POST", "/action", {"action": "purge_press"}),
            ("POST", "/action", {"action": "purge_release"}),
            ("GET", "/telemetry", None),
        ])

    def test_releases_on_interruption(self):
        actions = []

        def client(method, path, body):
            if path == "/config":
                return {"purge": {"max_s": 20}}
            if path == "/telemetry":
                return {"cycle": "idle"}
            actions.append(body["action"])
            return {"ok": True}

        def interrupt(_seconds):
            raise KeyboardInterrupt

        with self.assertRaises(KeyboardInterrupt):
            purge(10, client, sleep=interrupt)
        self.assertEqual(actions, ["purge_press", "purge_release"])

    def test_releases_when_press_response_is_lost(self):
        actions = []

        def client(method, path, body):
            if path == "/config":
                return {"purge": {"max_s": 20}}
            if path == "/telemetry":
                return {"cycle": "idle"}
            actions.append(body["action"])
            if body["action"] == "purge_press":
                raise RuntimeError("réponse Wi-Fi perdue")
            return {"ok": True}

        with self.assertRaisesRegex(RuntimeError, "réponse Wi-Fi perdue"):
            purge(10, client)
        self.assertEqual(actions, ["purge_press", "purge_release"])

    def test_rejects_firmware_timeout(self):
        calls = []

        def client(method, path, body):
            calls.append(path)
            return {"purge": {"max_s": 20}}

        with self.assertRaisesRegex(ValueError, "purge.max_s"):
            purge(20, client)
        self.assertEqual(calls, ["/config"])


if __name__ == "__main__":
    unittest.main()

import unittest

from set_scace import set_scace


class SetScaceTests(unittest.TestCase):
    def test_reads_version_and_sets_both_states(self):
        for enabled in (True, False):
            calls = []

            def client(method, path, body):
                calls.append((method, path, body))
                if method == "GET":
                    return {"version": 10, "scace": {"enabled": not enabled}}
                return {"scace": {"enabled": body["scace"]["enabled"]}}

            set_scace(enabled, client)
            self.assertEqual(calls, [
                ("GET", "/config", None),
                ("POST", "/config", {"version": 10, "scace": {"enabled": enabled}}),
            ])

    def test_rejects_unconfirmed_change(self):
        def client(method, path, body):
            return {"version": 10, "scace": {"enabled": False}} if method == "GET" else {"scace": {"enabled": False}}

        with self.assertRaisesRegex(RuntimeError, "non confirmé"):
            set_scace(True, client)

    def test_rejects_firmware_without_scace(self):
        def client(method, path, body):
            self.assertEqual(method, "GET")
            return {"version": 9}

        with self.assertRaisesRegex(RuntimeError, "0.3.47"):
            set_scace(True, client)


if __name__ == "__main__":
    unittest.main()

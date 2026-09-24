import unittest

from set_heating import set_heating


class SetHeatingTests(unittest.TestCase):
    def test_reads_version_and_sets_both_states(self):
        for enabled in (True, False):
            calls = []

            def client(method, path, body):
                calls.append((method, path, body))
                if method == "GET":
                    return {"version": 6}
                return {"heating": {"enabled": body["heating"]["enabled"]}}

            set_heating(enabled, client)
            self.assertEqual(calls, [
                ("GET", "/config", None),
                ("POST", "/config", {"version": 6, "heating": {"enabled": enabled}}),
            ])

    def test_rejects_unconfirmed_change(self):
        def client(method, path, body):
            return {"version": 6} if method == "GET" else {"heating": {"enabled": False}}

        with self.assertRaisesRegex(RuntimeError, "non confirmé"):
            set_heating(True, client)

    def test_rejects_invalid_version(self):
        def client(method, path, body):
            self.assertEqual(method, "GET")
            return {"version": True}

        with self.assertRaisesRegex(RuntimeError, "version absente"):
            set_heating(True, client)

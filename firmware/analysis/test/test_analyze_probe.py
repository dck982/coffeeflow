import math
import unittest
from pathlib import Path

from analyze_probe import (analyze, load_samples, ntc_resistance, ntc_temperature_c,
                           pt1000_resistance, pt1000_temperature_c)


def sample(elapsed, a1, age=50, a0=26304):
    return {"elapsed_s": elapsed, "a0_raw": a0, "a1_raw": a1, "age_ms": age}


def new_sample(elapsed, a1, age=50, a0=26304):
    return {"elapsed_s": elapsed, "received_at_utc": "2026-10-09T10:00:00.000+00:00",
            "telemetry": {"temperature": {"boiler": {
                "ntc_a0_raw": a0, "ntc_a1_raw": a1, "age_ms": age}}}}


class ConversionTests(unittest.TestCase):
    def test_ntc_matches_firmware_reading(self):
        # Relevé à froid du 2026-10-02 après flash de 0.3.33.
        resistance = 4676.0 * (26303 / 2382 - 1.0)
        self.assertAlmostEqual(ntc_temperature_c(resistance), 25.02, places=2)
        self.assertAlmostEqual(ntc_resistance(ntc_temperature_c(resistance)), resistance, places=6)

    def test_pt1000_inverse(self):
        self.assertAlmostEqual(pt1000_temperature_c(1000.0), 0.0, places=9)
        self.assertAlmostEqual(pt1000_temperature_c(pt1000_resistance(98.68)), 98.68, places=6)


class SampleTests(unittest.TestCase):
    def test_drops_repeated_conversions_and_invalid_codes(self):
        capture = {"samples": [
            sample(0.10, 2600, age=40),
            sample(0.30, 2600, age=240),  # même conversion relue 200 ms plus tard
            sample(0.50, 2601, age=40),
            sample(0.70, 0),
            sample(0.90, 2601, age=40),   # même code, nouvelle conversion
        ]}
        samples, duplicates, invalid = load_samples(capture)
        self.assertEqual([s.a1 for s in samples], [2600, 2601, 2601])
        self.assertEqual((duplicates, invalid), (1, 1))

    def test_new_files_read_codes_from_the_telemetry(self):
        capture = {"samples": [
            new_sample(0.10, 2600, age=40),
            new_sample(0.30, 2600, age=240),  # même conversion relue
            new_sample(0.50, 2601, age=40),
            {"elapsed_s": 0.7, "telemetry": {}},  # télémétrie sans sonde
            {"elapsed_s": 0.8, "telemetry": {"temperature": {"boiler": {"ntc_a0_raw": None}}}},
            new_sample(0.90, 0),
        ]}
        samples, duplicates, invalid = load_samples(capture)
        self.assertEqual([s.a1 for s in samples], [2600, 2601])
        self.assertEqual((duplicates, invalid), (1, 3))

    def test_new_files_ignore_root_level_codes(self):
        raw = new_sample(0.1, 2600)
        raw["a0_raw"], raw["a1_raw"] = 1, 1  # sans effet : la télémétrie fait foi
        samples, _, invalid = load_samples({"samples": [raw]})
        self.assertEqual((samples[0].a0, samples[0].a1, invalid), (26304, 2600, 0))

    def test_both_formats_give_the_same_report(self):
        old = {"samples": [sample(0.1 * i, 2600 + (i % 3)) for i in range(1, 80)]}
        new = {"samples": [new_sample(0.1 * i, 2600 + (i % 3)) for i in range(1, 80)]}
        kwargs = dict(probe="ntc", r_fixed=4676.0, r0=47000.0, beta=3950.0,
                      start=None, end=None, reference_c=None, bin_s=2.0)
        self.assertEqual(analyze(Path("a.json"), old, **kwargs),
                         analyze(Path("a.json"), new, **kwargs))

    def test_report_window_and_reference(self):
        capture = {"samples": [sample(0.1 * i, 2600 + (i % 2)) for i in range(1, 300)]}
        report = analyze(Path("probe.json"), capture, probe="ntc", r_fixed=4676.0,
                         r0=47000.0, beta=3950.0, start=10.0, end=20.0,
                         reference_c=27.0, bin_s=5.0)
        self.assertIn("conversions de 10.1 à 19.9 s", report)
        self.assertIn("référence 27.00 °C", report)
        bins = report.split("σ (°C)\n")[1].splitlines()
        self.assertEqual([int(line.split()[0]) for line in bins], [10, 15])
        self.assertFalse(math.isnan(float(report.split("dérive : ")[1].split()[0])))


if __name__ == "__main__":
    unittest.main()

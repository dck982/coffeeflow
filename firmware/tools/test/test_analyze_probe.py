import math
import unittest
from pathlib import Path

from analyze_probe import (analyze, load_samples, ntc_resistance, ntc_temperature_c,
                           pt1000_resistance, pt1000_temperature_c)


def sample(elapsed, a1, age=50, a0=26304):
    return {"elapsed_s": elapsed, "a0_raw": a0, "a1_raw": a1, "age_ms": age}


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

import csv
import re
import tempfile
import unittest
from pathlib import Path

import scace_calibration as cal


MAIN_CPP = Path(__file__).resolve().parents[2] / "scace" / "main" / "main.cpp"

# Extrait de la table R/T 8016 (fiche TDK S861) : °C, R/R25.
TABLE_8016 = [(-10, 5.533), (0, 3.265), (25, 1.0), (60, 0.2488), (90, 0.09177), (100, 0.068), (110, 0.05112)]


class ScaceCalibrationTest(unittest.TestCase):
    def test_coefficients_match_firmware(self):
        source = MAIN_CPP.read_text(encoding="utf-8")
        for name, value in (("kShA", cal.SH_A), ("kShB", cal.SH_B), ("kShC", cal.SH_C)):
            match = re.search(rf"{name} = ([0-9.e+-]+);", source)
            self.assertIsNotNone(match, name)
            self.assertEqual(float(match.group(1)), value, name)

    def test_model_follows_table(self):
        for celsius, ratio in TABLE_8016:
            self.assertAlmostEqual(cal.celsius_from_ohm(ratio * 10000), celsius, delta=0.025)

    def test_table_ohm_inverts_model(self):
        for celsius in (-5.0, 0.0, 37.0, 98.7):
            self.assertAlmostEqual(cal.celsius_from_ohm(cal.table_ohm(celsius)), celsius, places=6)

    def test_ice_then_boil_roundtrip(self):
        fixed = cal.fixed_ohm_from_ice(32.672)
        self.assertAlmostEqual(cal.celsius_from_ohm(fixed * 32.672), 0.0, places=6)
        # Une NTC dont B dépasse le nominal de 10 K, à 98,72 °C.
        ohm = cal.table_ohm(98.72) * 2.718281828 ** (10 * (1 / (98.72 + 273.15) - 1 / 273.15))
        shift = cal.beta_shift_from_boil(ohm / fixed, fixed, 98.72)
        self.assertAlmostEqual(shift, 10.0, places=3)
        self.assertAlmostEqual(cal.celsius_from_ohm(ohm, shift), 98.72, places=4)
        # Le décalage de B ne déplace pas le point de glace.
        self.assertAlmostEqual(cal.celsius_from_ohm(fixed * 32.672, shift), 0.0, places=6)

    def test_plateau_keeps_ok_frames_in_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "capture.csv"
            with path.open("w", newline="") as handle:
                writer = csv.writer(handle)
                writer.writerow(["unix_ms", "seq", "probe_ms", "a0", "a1", "celsius", "status"])
                writer.writerow([1000, 0, 0, 0, 0, "", "sans-mesure"])
                for i in range(1, 21):
                    writer.writerow([1000 + 100 * i, i, 0, 26420, 25640, "0.10", "ok"])
                writer.writerow([4000, 21, 0, 26420, 20000, "9.00", "ok"])
            stats = cal.plateau(path, 0.0, 2.5)
        self.assertEqual(stats["n"], 20)
        self.assertAlmostEqual(stats["ratio"], 25640 / 780)
        self.assertAlmostEqual(stats["drift_k_per_min"], 0.0)


if __name__ == "__main__":
    unittest.main()

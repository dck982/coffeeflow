import json
import math
import re

from describe_hf_capture import cup_flow_series, interactive_plot, report_weight_series, weight_flow_g_s


def test_weight_flow_does_not_bridge_a_hidden_weight():
    flow = weight_flow_g_s([0, 1, 2, 3, 4], [0, 1, math.nan, 3, 4], 4)
    assert all(math.isnan(value) for value in flow)


def test_cup_flow_ignores_negative_weights_and_cooldown():
    samples = [
        {"t_ms": 0, "weight_g": 0.0, "flags": 0x04, "mode": "infusion"},
        {"t_ms": 1000, "weight_g": 1.0, "flags": 0x04, "mode": "infusion"},
        {"t_ms": 2000, "weight_g": -100.0, "flags": 0x04, "mode": "infusion"},
        {"t_ms": 3000, "weight_g": 100.0, "flags": 0x04, "mode": "cooldown"},
        {"t_ms": 4000, "weight_g": 2.0, "flags": 0x04, "mode": "cooldown"},
    ]
    assert all(math.isnan(value) for value in cup_flow_series(samples, 4))


def test_report_weight_stops_after_negative_cooldown_measurement():
    samples = [
        {"weight_g": 20.0, "flags": 0x04, "mode": "infusion"},
        {"weight_g": 21.0, "flags": 0x04, "mode": "cooldown"},
        {"weight_g": -100.0, "flags": 0x04, "mode": "cooldown"},
        {"weight_g": 400.0, "flags": 0x04, "mode": "cooldown"},
    ]
    weights = report_weight_series(samples)
    assert weights[:2] == [20.0, 21.0]
    assert all(math.isnan(value) for value in weights[2:])


def test_ntc_slider_targets_temperature_and_drop_marker_not_ssr():
    samples = []
    for t_ms, temperature, heater_on in [(0, 90, False), (1000, 89, True),
                                          (2000, 88, True), (3000, 90, False)]:
        samples.append({
            "t_ms": t_ms, "flags": 15, "boiler_temperature_c": temperature,
            "boiler_temperature_valid": True, "pressure_bar": 1, "flow_ml_s": 0,
            "weight_g": 0, "pump_pct_commanded": 0, "pump_pct_reported": 0,
            "heating_power_pct": 50, "heater_on": heater_on, "volume_ml": 0,
            "mode": "thermal_preheat",
        })
    html = interactive_plot({"samples": samples, "started_at_us": 0,
                             "ended_at_us": 3_000_000}, 2)
    plot = json.loads(re.search(r"const plot = (\{.*?\});", html).group(1))
    traces = plot["traces"]
    controls = plot["controls"]

    assert traces[controls["temperatureTrace"]]["name"] == "température chaudière"
    assert traces[controls["dropTrace"]]["name"] == "baisse thermique maximale"
    assert controls["dropTrace"] != next(index for index, trace in enumerate(traces)
                                          if trace["name"] == "SSR chaudière actif")


def test_cup_flow_continues_after_pump_stop_until_first_zero():
    from describe_hf_capture import drip_truncated

    flows = [1.2, 1.1, 0.6, 0.2, -0.1, 0.3, -5.0]
    assert drip_truncated(flows, 2)[:4] == [1.2, 1.1, 0.6, 0.2]
    assert drip_truncated(flows, 2)[4] == 0.0
    assert all(math.isnan(value) for value in drip_truncated(flows, 2)[5:])
    hidden = drip_truncated([1.0, 0.5, math.nan, 0.4], 1)
    assert hidden[:2] == [1.0, 0.5] and all(math.isnan(value) for value in hidden[2:])


def test_shot_summary_matches_screen_definitions():
    # Même infusion synthétique que firmware/screen/test/test_shot_summary.cpp.
    from describe_hf_capture import shot_summary

    def sample(t):
        mode = ("thermal_preheat" if t < 5 else "filling" if t < 8 else "preinfusion" if t < 12
                else "infusion" if t < 40 else "cooldown")
        weight = 10 + max(0.0, min(t, 40) - 16) + (min(t - 40, 2) * 0.25 if t > 40 else 0)
        return {"t_ms": round(t * 1000), "mode": mode, "flags": 0x0F,
                "pump_pct_reported": 60 if 5 <= t < 40 else 0,
                "pressure_bar": 0.3 if t < 12 else min(9.0, 0.3 + (t - 12) * 2),
                "weight_g": weight, "volume_ml": max(0.0, min(t, 40) - 5) * 1.5,
                "boiler_temperature_c": 90 - max(0.0, t - 5) * 0.1 if t < 16 else 88.9 + (t - 16) * 0.1}

    shot = shot_summary([sample(i / 10) for i in range(601)])
    assert abs(shot["segments"]["infusion"][0] - 15.9) < 0.01
    assert abs(shot["infusion_s"] - 24.1) < 0.01
    assert abs(shot["infusion_cup_g_s"] - 23.9 / 24.1) < 0.01
    assert abs(shot["infusion_ml_s"] - 1.5) < 0.02
    assert abs(shot["cup_weight_g"] - 24.5) < 0.06 and shot["drip_done"]
    assert abs(shot["drip_gain_g"] - 0.5) < 0.06
    assert abs(shot["cup_mean_c"] - 90.1) < 0.05
    assert abs(shot["min_temperature_c"] - 88.9) < 0.02 and abs(shot["max_temperature_c"] - 91.3) < 0.02


def test_text_phase_table_integrates_heating_per_phase():
    from describe_hf_capture import text_events, text_phase_table

    samples = [{"t_ms": t * 1000, "mode": "thermal_preheat" if t < 2 else "filling", "flags": 0x0F,
                "heating_power_pct": 90 if t < 2 else 50, "heater_on": t < 2,
                "pressure_bar": 0.3 + max(0, t - 2), "volume_ml": max(0, t - 2) * 3.0,
                "weight_g": 0.0, "boiler_temperature_c": 90.0} for t in range(5)]
    lines = text_phase_table(samples, 5.0).splitlines()
    assert lines[1].split()[-3:] == ["180", "2.16", "2.40"]
    assert lines[2].split()[-3:] == ["150", "1.80", "0.00"]
    assert "pression ≥ 2 bar                    2.00     6.0    2.30  remplissage" in text_events(samples)


def _guided_capture(**extra):
    samples = [{"t_ms": t * 1000, "mode": "thermal_preheat" if t < 2 else "filling", "flags": 0x0F,
                "heating_power_pct": 0, "heater_on": False, "pressure_bar": 1.0,
                "volume_ml": 0.0, "weight_g": 0.0, "boiler_temperature_c": 90.0}
               for t in range(6)]
    return {"schema": "coffeeflow.hf_capture.v2", "origin": "brew", "samples": samples,
            "started_at_us": 0, "ended_at_us": 5_000_000, **extra}


RINSE = {"start_ms": -279400, "duration_ms": 5000, "volume_ml": 24.6, "weight_g": 23.1,
         "ntc_start_c": 85.9, "ntc_min_c": 74.2, "ntc_end_c": 74.8, "pressure_max_bar": 0.5}

GUIDED_SESSION = {
    "opened_ms": -412300, "opened_at_unix_s": 0, "setpoint_c": 92.0, "ntc_open_c": 88.1,
    "cup_g": 41.3, "rinse": RINSE,
    "wait": {"ntc_min_c": 74.0, "setpoint_reached_ms": -233800, "heater_on_ms": 31200,
             "early_removals": 0},
    "cup_removed_ms": -118900, "cup_returned_ms": -21400, "start_pressed_ms": 0,
    "scale_lost_ms": 0,
}

# Format d'avant le 15 octobre 2026 : purge sans porte-filtre puis rinçage
# du porte-filtre, sans poids de tasse.
OLD_SESSION = {
    **{k: v for k, v in GUIDED_SESSION.items() if k not in ("cup_g", "rinse")},
    "purges": [
        {"start_ms": -301200, "duration_ms": 5000, "volume_ml": 24.9, "weight_g": None,
         "ntc_start_c": 91.2, "ntc_min_c": 84.0, "ntc_end_c": 84.6, "pressure_max_bar": 0.4},
        RINSE,
    ],
}


def test_session_durations_follow_the_recorded_instants():
    from describe_hf_capture import session_durations

    for session in (GUIDED_SESSION, OLD_SESSION):
        durations = session_durations(session)
        assert abs(durations["setpoint_return_s"] - 40.6) < 1e-9
        assert abs(durations["holder_in_place_s"] - 114.9) < 1e-9
        assert abs(durations["preparation_s"] - 118.9) < 1e-9


def test_report_shows_the_guided_session_block():
    from pathlib import Path
    from describe_hf_capture import render_report

    page = render_report(_guided_capture(session=GUIDED_SESSION), Path("brew.json"))
    assert "Infusion guidée" in page
    assert "Rinçage du porte-filtre" in page and "85.9 → 74.2 → 74.8 °C" in page
    assert "Purge sans porte-filtre" not in page
    assert "40.6 s" in page and "114.9 s" in page and "118.9 s" in page
    assert "41.3 g" in page


def test_report_reads_the_two_purges_of_older_captures():
    from pathlib import Path
    from describe_hf_capture import render_report

    page = render_report(_guided_capture(session=OLD_SESSION), Path("brew.json"))
    assert "Purge sans porte-filtre" in page and "91.2 → 84.0 → 84.6 °C" in page
    assert "Rinçage du porte-filtre" in page and "40.6 s" in page
    assert "Tasse" not in page


def test_report_has_no_guided_block_without_a_session():
    from pathlib import Path
    from describe_hf_capture import render_report

    assert "Infusion guidée" not in render_report(_guided_capture(), Path("brew.json"))
    assert "Infusion guidée" not in render_report(_guided_capture(session=None), Path("brew.json"))


def test_guided_block_tolerates_unknown_values():
    from pathlib import Path
    from describe_hf_capture import render_report

    session = {**GUIDED_SESSION, "cup_g": None, "rinse": {**RINSE, "weight_g": None},
               "wait": {**GUIDED_SESSION["wait"], "setpoint_reached_ms": None,
                        "ntc_min_c": None},
               "cup_removed_ms": None}
    page = render_report(_guided_capture(session=session), Path("brew.json"))
    assert "Infusion guidée" in page and "—" in page

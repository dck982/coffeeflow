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

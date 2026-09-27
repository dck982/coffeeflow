import math

from describe_hf_capture import cup_flow_series, report_weight_series, weight_flow_g_s


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

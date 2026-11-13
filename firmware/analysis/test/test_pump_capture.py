import pytest

from pump_capture import analyze, capture_k, glitch_position


def capture(rpm=4500, glitches=(), seconds=6.0, stop_after=5.0, weight_ratio=0.8):
    """Purge synthétique à 100 ms : pompe à `rpm`, balance à `weight_ratio`
    du refoulé, Digmesa à 2 500 imp/L de l'eau pesée, arrêt à `stop_after`."""
    ml_s = rpm * 0.1 / 60
    samples = []
    displaced = pulses = weight = 0.0
    for n in range(int(seconds * 10) + 1):
        t = n / 10
        on = t < stop_after
        if on and n > 0:
            displaced += ml_s * 0.1
            weight += ml_s * weight_ratio * 0.1
            pulses += ml_s * weight_ratio * 0.1 * 2.5
        read = rpm if on and n > 0 else 0
        if n in glitches:
            read = 2 * rpm
            displaced += 0.05
        samples.append({
            "t_ms": int(t * 1000), "pump_pct_commanded": 100 if on else 0,
            "pump_rpm": read, "pump_displaced_ml": round(displaced, 2),
            "flow_pulse_count": int(pulses), "volume_ml": int(pulses) / 3.45,
            "weight_g": weight, "pressure_bar": 0.1 if on else 0.2, "flags": 0x0F,
        })
    return {"samples": samples}


def test_steady_rates_and_k():
    r = analyze(capture())
    assert r.rpm_median == 4500
    assert r.rpm_ml_s == pytest.approx(7.5)
    assert r.displaced_ml_s.per_s == pytest.approx(7.5, rel=0.01)
    assert r.weight_g_s.per_s == pytest.approx(6.0, rel=0.01)
    assert r.flow_pulses_s.per_s == pytest.approx(15.0, rel=0.05)
    assert r.outliers == []
    assert r.pressure_before_bar == pytest.approx(0.1)
    assert r.pressure_after_bar == pytest.approx(0.2)
    assert r.rundown_s == pytest.approx(0.0)


def test_glitches_are_listed_once_with_their_position():
    r = analyze(capture(glitches=(20, 30)))
    assert [rpm for _, rpm, _ in r.outliers] == [9000, 9000]
    assert all(x == pytest.approx(0.5) for _, _, x in r.outliers)


def test_glitch_position_from_a_one_and_a_third_reading():
    assert glitch_position(6138, 4605) == pytest.approx(0.5, abs=0.01)
    assert glitch_position(9209, 4605) == 0.5


def test_capture_k_reads_the_firmware_constant_back():
    assert capture_k(capture()) == pytest.approx(3450, rel=0.01)


def test_pump_never_commanded_is_an_error():
    c = capture()
    for s in c["samples"]:
        s["pump_pct_commanded"] = 0
    with pytest.raises(RuntimeError):
        analyze(c)

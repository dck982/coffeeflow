import pytest

from analyze_hf_capture import cup_weighted_temperature


def sample(weight_g, temperature_c, flags=0x0F):
    return {"weight_g": weight_g, "boiler_temperature_c": temperature_c,
            "boiler_temperature_valid": True, "flags": flags}


def test_cup_mean_weights_each_gram_with_the_ntc_of_its_arrival():
    samples = [sample(0.0, 90.0), sample(1.0, 90.0), sample(3.0, 94.0)]
    cup_g, average_c = cup_weighted_temperature(samples, 2)
    assert cup_g == pytest.approx(3.0)
    assert average_c == pytest.approx((1 * 90 + 2 * 92) / 3)


def test_cup_mean_ignores_scale_dips_and_invalid_weights():
    samples = [sample(0.0, 90.0), sample(2.0, 90.0), sample(1.5, 90.0),
               sample(-100.0, 90.0), sample(50.0, 90.0, flags=0x0B), sample(3.0, 92.0)]
    cup_g, average_c = cup_weighted_temperature(samples, 5)
    assert cup_g == pytest.approx(3.0)
    assert average_c == pytest.approx((2 * 90 + 1 * 91) / 3)


def test_cup_mean_without_scale_is_none():
    assert cup_weighted_temperature([sample(0.0, 90.0, flags=0x0B)] * 3, 2) is None


def test_scace_cup_mean_uses_the_basket_probe_and_skips_missing_frames():
    samples = [dict(sample(0.0, 90.0), scace_temperature_c=88.0),
               dict(sample(1.0, 90.0), scace_temperature_c=None),
               dict(sample(3.0, 90.0), scace_temperature_c=92.0)]
    cup_g, average_c = cup_weighted_temperature(samples, 2, "scace")
    assert cup_g == pytest.approx(3.0)
    assert average_c == pytest.approx(90.0)


def test_scace_cup_mean_without_probe_is_none():
    assert cup_weighted_temperature([sample(0.0, 90.0), sample(2.0, 90.0)], 1, "scace") is None

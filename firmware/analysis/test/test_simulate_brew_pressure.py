from simulate_brew_pressure import (
    REGULATIONS, Controller, Plant, Regulation, simulate, summarize_trace,
)


def run(controller, points):
    return [controller.sample(t, p) for t, p in points]


def test_controller_matches_machine_rs_pressure_regulation():
    # Même suite que `pressure_regulation` dans screen-core/tests/machine.rs.
    c = Controller(REGULATIONS["0.5.6"])
    points = [(2.0, 6.9), (2.1, 7.0), (2.2, 8.0), (2.3, 9.0), (2.4, 9.0),
              (2.5, 9.5), (2.6, 9.5), (2.7, 9.5)]
    assert run(c, points) == [100, 99, 68, 39, 54, 54, 54, 61]


def test_derivative_stops_at_the_first_peak():
    # Même suite que `derivative_stops_at_the_first_pressure_peak`.
    c = Controller(REGULATIONS["0.5.6"])
    points = [(2.0, 8.0), (2.1, 9.0), (2.2, 8.8), (2.3, 8.6), (2.4, 9.0), (2.5, 9.6)]
    assert run(c, points)[2:] == [76, 91, 85, 76]


def test_permanent_derivative_oscillates_and_rise_only_does_not():
    plant = Plant()
    always = summarize_trace(simulate(plant, REGULATIONS["0.5.5"], 6.4))
    rise = summarize_trace(simulate(plant, REGULATIONS["0.5.6"], 6.4))
    assert always.oscillates
    assert not rise.oscillates
    assert rise.peak_bar < summarize_trace(simulate(plant, Regulation(0.0), 6.4)).peak_bar

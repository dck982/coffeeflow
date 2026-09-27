#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# ///
"""Génère un rapport HTML interactif à partir d'une capture HF CoffeeFlow.

Le rapport contient le graphe de la capture, un résumé textuel, une chronologie
des phases et les mesures aux points clés. Il est conçu pour rester lisible
sans devoir interpréter visuellement le graphe.

Exemple :
    uv run firmware/tools/describe_hf_capture.py captures/260927-144713.json
"""

from __future__ import annotations

import argparse
import html
import json
import math
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Iterable

PHASE_STYLE = {
    "thermal_preheat": ("précharge thermique", "#4a9bb5"),
    "filling": ("remplissage", "#8c6bb1"),
    "preinfusion": ("pré-infusion", "#6baed6"),
    "infusion": ("infusion", "#fdae6b"),
    "ramp_down": ("descente", "#74c476"),
    "cooldown": ("récupération", "#bdbdbd"),
    "purge": ("purge", "#fb6a4a"),
}
MODE_LABELS = {mode: label for mode, (label, _color) in PHASE_STYLE.items()}
ORIGIN_LABELS = {"brew": "infusion", "purge": "purge"}
VALIDITY_FLAGS = {
    "pressure": (0x01, "Pression"),
    "flow": (0x02, "Débitmètre"),
    "scale": (0x04, "Balance"),
    "boiler": (0x08, "Température chaudière"),
}


def validate_capture(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise RuntimeError("le fichier JSON ne contient pas un objet")
    if payload.get("schema") not in ("coffeeflow.hf_capture.v1", "coffeeflow.hf_capture.v2") \
            or not isinstance(payload.get("samples"), list):
        raise RuntimeError("capture inconnue ou incompatible")
    return payload


def weight_flow_g_s(times: list[float], weight_g: list[float], window_s: float) -> list[float]:
    if window_s <= 0:
        raise RuntimeError("la fenêtre de débit balance doit être strictement positive")
    half_window_s = window_s / 2
    flow = [float("nan")] * len(times)
    left = right = 0
    for index, time_s in enumerate(times):
        while left < len(times) and times[left] < time_s - half_window_s:
            left += 1
        while right < len(times) and times[right] <= time_s + half_window_s:
            right += 1
        right_index = right - 1
        # Une mesure de poids masquée (NaN) invalide toute la fenêtre. Cela
        # évite de relier artificiellement les deux côtés d'un retrait de tasse.
        window_is_valid = all(math.isfinite(weight_g[position])
                              for position in range(left, right))
        if left < index < right_index and window_is_valid:
            elapsed_s = times[right_index] - times[left]
            if elapsed_s > 0:
                flow[index] = (weight_g[right_index] - weight_g[left]) / elapsed_s
    return flow


def number(value: Any) -> float:
    """Convertit une mesure en nombre, ou NaN si elle est absente/invalide."""
    if isinstance(value, bool):
        return float("nan")
    try:
        result = float(value)
    except (TypeError, ValueError):
        return float("nan")
    return result if math.isfinite(result) else float("nan")


def fmt(value: Any, digits: int = 2, suffix: str = "") -> str:
    value_f = number(value)
    if not math.isfinite(value_f):
        return "—"
    return f"{value_f:.{digits}f}{suffix}"


def mode_label(mode: Any) -> str:
    text = str(mode or "inconnu")
    return MODE_LABELS.get(text, text.replace("_", " "))


def valid(sample: dict[str, Any], signal: str) -> bool:
    if signal == "boiler":
        if "boiler_temperature_valid" in sample:
            return sample.get("boiler_temperature_valid") is True
        if "temperature_c" in sample:  # schéma v1 : XDB401, sans bit chaudière dédié
            return math.isfinite(number(sample.get("temperature_c")))
    bit, _label = VALIDITY_FLAGS[signal]
    try:
        return bool(int(sample.get("flags", 0)) & bit)
    except (TypeError, ValueError):
        return False


def pressure_fmt(sample: dict[str, Any], digits: int = 2) -> str:
    return fmt(sample.get("pressure_bar"), digits) if valid(sample, "pressure") else "invalide"


def has_weight(samples: list[dict[str, Any]]) -> bool:
    return any(valid(sample, "scale") and number(sample.get("weight_g")) >= 0
               for sample in samples)


def report_weight_series(samples: list[dict[str, Any]]) -> list[float]:
    """Masque les poids négatifs et la suite d'un cooldown perturbé."""
    result: list[float] = []
    cooldown_disturbed = False
    for sample in samples:
        value = number(sample.get("weight_g"))
        if sample.get("mode") == "cooldown" and value < 0:
            cooldown_disturbed = True
        if (not valid(sample, "scale") or value < 0 or
                (sample.get("mode") == "cooldown" and cooldown_disturbed)):
            result.append(float("nan"))
        else:
            result.append(value)
    return result


def cup_flow_series(samples: list[dict[str, Any]], window_s: float) -> list[float]:
    times = [number(sample.get("t_ms")) / 1000 for sample in samples]
    # Le débit tasse n'est défini que pendant le cycle hydraulique. Les poids
    # négatifs signalent notamment une tasse retirée et sont exclus du rapport,
    # sans modifier la capture brute.
    weights = [number(sample.get("weight_g"))
               if valid(sample, "scale") and sample.get("mode") != "cooldown" and
               number(sample.get("weight_g")) >= 0 else float("nan")
               for sample in samples]
    if not has_weight(samples):
        return [float("nan")] * len(samples)
    return weight_flow_g_s(times, weights, window_s)


def capture_duration_s(capture: dict[str, Any]) -> float:
    started = number(capture.get("started_at_us"))
    ended = number(capture.get("ended_at_us"))
    if math.isfinite(started) and math.isfinite(ended) and ended >= started:
        return (ended - started) / 1_000_000
    samples = capture["samples"]
    return number(samples[-1].get("t_ms")) / 1000 if samples else 0.0


def capture_datetime(capture: dict[str, Any], source: Path) -> datetime | None:
    unix_s = number(capture.get("started_at_unix_s"))
    if math.isfinite(unix_s) and unix_s > 0:
        return datetime.fromtimestamp(unix_s).astimezone()
    match = re.fullmatch(r"(\d{6})-(\d{6})", source.stem)
    if match:
        try:
            return datetime.strptime("".join(match.groups()), "%y%m%d%H%M%S").astimezone()
        except ValueError:
            pass
    return None


def first_pump_stop(samples: list[dict[str, Any]]) -> int | None:
    was_running = False
    for index, sample in enumerate(samples):
        pump = number(sample.get("pump_pct_reported"))
        if was_running and pump == 0:
            return index
        was_running = was_running or pump > 0
    return None


def closest_index(samples: list[dict[str, Any]], target_ms: float) -> int:
    return min(range(len(samples)), key=lambda index: abs(number(samples[index].get("t_ms")) - target_ms))


def transition_indices(samples: list[dict[str, Any]]) -> list[int]:
    return [index for index in range(1, len(samples))
            if samples[index].get("mode") != samples[index - 1].get("mode")]


def invalid_ranges(samples: list[dict[str, Any]], signal: str,
                   duration_s: float) -> list[tuple[float, float]]:
    """Regroupe les échantillons invalides en intervalles temporels."""
    ranges: list[tuple[float, float]] = []
    start_s: float | None = None
    for sample in samples:
        time_s = number(sample.get("t_ms")) / 1000
        if time_s >= duration_s:
            if start_s is not None:
                ranges.append((start_s, duration_s))
            return ranges
        if not valid(sample, signal) and start_s is None:
            start_s = time_s
        if valid(sample, signal) and start_s is not None:
            ranges.append((start_s, time_s))
            start_s = None
    if start_s is not None:
        ranges.append((start_s, duration_s))
    return ranges


def invalid_duration_s(samples: list[dict[str, Any]], signal: str, duration_s: float) -> float:
    return sum(end_s - start_s for start_s, end_s in invalid_ranges(samples, signal, duration_s))


def key_points(samples: list[dict[str, Any]]) -> list[tuple[int, str, bool]]:
    """Retourne début, repère 1 s et premier sample de chaque nouveau mode."""
    events: dict[int, list[str]] = {0: ["Début de capture"]}
    one_second = closest_index(samples, 1000)
    events.setdefault(one_second, []).append("Repère t = 1 s")
    transitions = set(transition_indices(samples))
    for index in transitions:
        before = mode_label(samples[index - 1].get("mode"))
        after = mode_label(samples[index].get("mode"))
        events.setdefault(index, []).append(f"Transition {before} → {after}")
    return [(index, " · ".join(labels), index in transitions)
            for index, labels in sorted(events.items())]


def extrema(samples: list[dict[str, Any]], field: str, maximum: bool = True,
            signal: str | None = None) -> tuple[float, float] | None:
    candidates = [(number(sample.get(field)), number(sample.get("t_ms")) / 1000) for sample in samples
                  if signal is None or valid(sample, signal)]
    candidates = [(value, time_s) for value, time_s in candidates
                  if math.isfinite(value) and math.isfinite(time_s)]
    if not candidates:
        return None
    return (max if maximum else min)(candidates, key=lambda item: item[0])


def phase_ranges(samples: list[dict[str, Any]], duration_s: float) -> list[tuple[int, int, float, float, str]]:
    transitions = transition_indices(samples)
    starts = [0, *transitions]
    ends = [index - 1 for index in transitions] + [len(samples) - 1]
    ranges = []
    for position, (start, end) in enumerate(zip(starts, ends)):
        start_s = number(samples[start].get("t_ms")) / 1000
        end_s = (number(samples[starts[position + 1]].get("t_ms")) / 1000
                 if position + 1 < len(starts) else duration_s)
        ranges.append((start, end, start_s, end_s, str(samples[start].get("mode", ""))))
    return ranges


def delta(before: dict[str, Any], after: dict[str, Any], field: str) -> float:
    return number(after.get(field)) - number(before.get(field))


def phase_table(samples: list[dict[str, Any]], duration_s: float) -> str:
    rows = []
    for start, end, start_s, end_s, mode in phase_ranges(samples, duration_s):
        first, last = samples[start], samples[end]
        pressure_values = [number(sample.get("pressure_bar")) for sample in samples[start:end + 1]
                           if valid(sample, "pressure")]
        pressure_values = [value for value in pressure_values if math.isfinite(value)]
        max_pressure = max(pressure_values) if pressure_values else float("nan")
        weight_cell = ("ignoré pendant le cooldown"
                       if mode == "cooldown"
                       else f"{fmt(first.get('weight_g'), 1)} → {fmt(last.get('weight_g'), 1)} g"
                            f"<br><small>Δ {fmt(delta(first, last, 'weight_g'), 1)} g</small>")
        rows.append(f"""
          <tr class="{'cooldown' if mode == 'cooldown' else ''}">
            <th scope="row">{html.escape(mode_label(mode))}</th>
            <td>{start_s:.2f}–{end_s:.2f} s</td>
            <td>{end_s - start_s:.2f} s</td>
            <td>{pressure_fmt(first)} → {pressure_fmt(last)} bar<br><small>max. valide {fmt(max_pressure)} bar</small></td>
            <td>{weight_cell}</td>
            <td>{fmt(first.get('volume_ml'), 1)} → {fmt(last.get('volume_ml'), 1)} ml<br><small>Δ {fmt(delta(first, last, 'volume_ml'), 1)} ml</small></td>
            <td>{fmt(first.get('boiler_temperature_c', first.get('temperature_c')))} → {fmt(last.get('boiler_temperature_c', last.get('temperature_c')))} °C</td>
          </tr>""")
    return "\n".join(rows)


def key_points_table(samples: list[dict[str, Any]], weight_flow_window_s: float) -> str:
    cup_flows = cup_flow_series(samples, weight_flow_window_s)

    def value(index: int, field: str, digits: int, unit: str) -> str:
        current = fmt(samples[index].get(field), digits)
        if index in transitions:
            previous = fmt(samples[index - 1].get(field), digits)
            return f"{previous} → {current} {unit}"
        return f"{current} {unit}"

    def temperature(index: int) -> str:
        field = "boiler_temperature_c" if "boiler_temperature_c" in samples[index] else "temperature_c"
        return value(index, field, 2, "°C")

    def pressure(index: int) -> str:
        current = pressure_fmt(samples[index])
        if index in transitions:
            return f"{pressure_fmt(samples[index - 1])} → {current} bar"
        return f"{current} bar"

    def cup_flow(index: int) -> str:
        current = fmt(cup_flows[index])
        if index in transitions:
            return f"{fmt(cup_flows[index - 1])} → {current} g/s"
        return f"{current} g/s"

    transitions = set(transition_indices(samples))
    rows = []
    for index, event, is_transition in key_points(samples):
        sample = samples[index]
        time_s = number(sample.get("t_ms")) / 1000
        if is_transition:
            mode = f"{mode_label(samples[index - 1].get('mode'))} → {mode_label(sample.get('mode'))}"
            pump = (f"{fmt(samples[index - 1].get('pump_pct_commanded'), 0)}/{fmt(samples[index - 1].get('pump_pct_reported'), 0)}"
                    f" → {fmt(sample.get('pump_pct_commanded'), 0)}/{fmt(sample.get('pump_pct_reported'), 0)} %")
        else:
            mode = mode_label(sample.get("mode"))
            pump = f"{fmt(sample.get('pump_pct_commanded'), 0)} / {fmt(sample.get('pump_pct_reported'), 0)} %"
        rows.append(f"""
          <tr class="{'transition' if is_transition else ''}">
            <th scope="row">{html.escape(event)}</th>
            <td>{time_s:.2f} s</td>
            <td><span class="mode">{html.escape(mode)}</span></td>
            <td>{pressure(index)}</td>
            <td>{cup_flow(index)}</td>
            <td>{value(index, 'volume_ml', 1, 'ml')}</td>
            <td>{value(index, 'weight_g', 1, 'g')}</td>
            <td>{temperature(index)}</td>
            <td>{pump}</td>
            <td>{value(index, 'heating_power_pct', 0, '%')}</td>
          </tr>""")
    return "\n".join(rows)


def textual_reading(samples: list[dict[str, Any]], duration_s: float,
                    weight_flow_window_s: float) -> str:
    cup_flows = cup_flow_series(samples, weight_flow_window_s)
    sentences: list[str] = []
    for index in transition_indices(samples):
        previous, current = samples[index - 1], samples[index]
        time_s = number(current.get("t_ms")) / 1000
        sentences.append(
            f"À {time_s:.2f} s, passage de {mode_label(previous.get('mode'))} à "
            f"{mode_label(current.get('mode'))}. Entre le dernier échantillon avant et le premier après, "
            f"la pression vaut {pressure_fmt(previous)} → {pressure_fmt(current)} bar, "
            f"le poids {fmt(previous.get('weight_g'), 1)} → {fmt(current.get('weight_g'), 1)} g, "
            f"le volume {fmt(previous.get('volume_ml'), 1)} → {fmt(current.get('volume_ml'), 1)} ml, "
            f"le débit tasse {fmt(cup_flows[index - 1])} → {fmt(cup_flows[index])} g/s et la pompe "
            f"demandée/rapportée {fmt(previous.get('pump_pct_commanded'), 0)}/{fmt(previous.get('pump_pct_reported'), 0)}"
            f" → {fmt(current.get('pump_pct_commanded'), 0)}/{fmt(current.get('pump_pct_reported'), 0)} %."
        )
    stop = first_pump_stop(samples)
    if stop is not None:
        sample = samples[stop]
        sentences.append(
            f"La pompe rapportée s’arrête à {number(sample.get('t_ms')) / 1000:.2f} s, "
            f"à {fmt(sample.get('weight_g'), 1)} g et {fmt(sample.get('volume_ml'), 1)} ml."
        )
    sentences.append(
        "Pendant la récupération post-arrêt, les variations de poids ne sont pas interprétées ; "
        f"le volume atteint {fmt(samples[-1].get('volume_ml'), 1)} ml."
    )
    return " ".join(sentences)


def metric_card(label: str, value: str, detail: str) -> str:
    return (f'<div class="metric"><span>{html.escape(label)}</span>'
            f'<strong>{html.escape(value)}</strong><small>{html.escape(detail)}</small></div>')


def validity_table(samples: list[dict[str, Any]], duration_s: float,
                   start_s: float = 0.0) -> str:
    scoped_samples = [sample for sample in samples
                      if start_s <= number(sample.get("t_ms")) / 1000 <= duration_s]
    interval_s = max(0.0, duration_s - start_s)
    rows = []
    for signal, (_bit, label) in VALIDITY_FLAGS.items():
        valid_count = sum(valid(sample, signal) for sample in scoped_samples)
        ranges = invalid_ranges(scoped_samples, signal, duration_s)
        invalid_s = sum(end_s - start_s for start_s, end_s in ranges)
        coverage = 100 * max(0.0, interval_s - invalid_s) / interval_s if interval_s > 0 else 0.0
        longest = max((end_s - start_s for start_s, end_s in ranges), default=0.0)
        rows.append(f"""
          <tr class="{'signal-warning' if ranges else ''}">
            <th scope="row">{html.escape(label)}</th>
            <td>{valid_count}/{len(scoped_samples)}</td><td>{coverage:.1f} %</td>
            <td>{len(ranges)}</td><td>{invalid_s:.2f} s</td><td>{longest:.2f} s</td>
          </tr>""")
    return "\n".join(rows)


def invalid_pressure_details(samples: list[dict[str, Any]], duration_s: float,
                             start_s: float = 0.0) -> str:
    scoped_samples = [sample for sample in samples
                      if number(sample.get("t_ms")) / 1000 >= start_s]
    ranges = invalid_ranges(scoped_samples, "pressure", duration_s)
    if not ranges:
        return "<p>Aucune plage de pression invalide.</p>"
    items = "".join(f"<li>{start_s:.2f}–{end_s:.2f} s ({end_s - start_s:.2f} s)</li>"
                    for start_s, end_s in ranges)
    return f"<details><summary>Plages de pression invalide ({len(ranges)})</summary><ul>{items}</ul></details>"


def interactive_plot(capture: dict[str, Any], weight_flow_window_s: float) -> str:
    samples: list[dict[str, Any]] = capture["samples"]
    duration_s = capture_duration_s(capture)
    times = [number(sample.get("t_ms")) / 1000 for sample in samples]
    cup_flows = cup_flow_series(samples, weight_flow_window_s)
    report_weights = report_weight_series(samples)
    temperatures = [
        number(sample.get("boiler_temperature_c", sample.get("temperature_c")))
        if valid(sample, "boiler") else float("nan")
        for sample in samples
    ]
    initial_temperature = next((value for value in temperatures if math.isfinite(value)), float("nan"))
    valid_temperature_points = [(index, value) for index, value in enumerate(temperatures) if math.isfinite(value)]
    min_temperature_index, min_temperature = min(valid_temperature_points, key=lambda item: item[1])
    max_temperature_drop = initial_temperature - min_temperature

    def series(field: str, predicate: Any = None) -> list[float | None]:
        result: list[float | None] = []
        for sample in samples:
            value = number(sample.get(field))
            result.append(value if math.isfinite(value) and (predicate is None or predicate(sample)) else None)
        return result

    def finite_series(values: list[float]) -> list[float | None]:
        return [value if math.isfinite(value) else None for value in values]

    pressure_bridge_x: list[float | None] = []
    pressure_bridge_y: list[float | None] = []
    index = 0
    while index < len(samples):
        if valid(samples[index], "pressure"):
            index += 1
            continue
        first_invalid = index
        while index < len(samples) and not valid(samples[index], "pressure"):
            index += 1
        if first_invalid > 0 and index < len(samples):
            before, after = first_invalid - 1, index
            pressure_bridge_x.extend([times[before], times[after], None])
            pressure_bridge_y.extend([
                number(samples[before].get("pressure_bar")),
                number(samples[after].get("pressure_bar")),
                None,
            ])

    traces = [
        {"name": "pression valide", "x": times, "y": series("pressure_bar", lambda s: valid(s, "pressure")),
         "type": "scatter", "mode": "lines", "line": {"color": "#d62728", "width": 2}, "yaxis": "y",
         "hovertemplate": "%{y:.2f} bar<extra></extra>", "legendrank": 1},
        {"name": "pompe demandée", "x": times, "y": series("pump_pct_commanded"), "type": "scatter",
         "mode": "lines", "line": {"color": "#1f77b4", "dash": "dash", "shape": "hv"}, "yaxis": "y2",
         "hovertemplate": "%{y:.0f} %<extra></extra>"},
        {"name": "pompe rapportée", "x": times, "y": series("pump_pct_reported"), "type": "scatter",
         "mode": "lines", "line": {"color": "#1f77b4", "shape": "hv"}, "yaxis": "y2",
         "hovertemplate": "%{y:.0f} %<extra></extra>"},
        {"name": f"débit tasse · balance ({weight_flow_window_s:g} s)", "x": times,
         "y": finite_series(cup_flows), "type": "scatter", "mode": "lines",
         "line": {"color": "#9467bd", "width": 2.5}, "yaxis": "y3",
         "hovertemplate": "%{y:.2f} g/s<extra></extra>"},
        {"name": "débitmètre", "x": times,
         "y": series("flow_ml_s", lambda s: valid(s, "flow")),
         "type": "scatter", "mode": "lines", "line": {"color": "#2ca02c", "dash": "dot", "width": 1.5},
         "opacity": 0.65, "yaxis": "y3", "hovertemplate": "%{y:.2f} ml/s<extra></extra>"},
        {"name": "température chaudière", "x": times, "y": finite_series(temperatures), "type": "scatter",
         "mode": "lines", "line": {"color": "#ff7f0e", "width": 2}, "yaxis": "y4",
         "hovertemplate": "%{y:.2f} °C<extra></extra>"},
        {"name": "chauffage demandé", "x": times, "y": series("heating_power_pct"), "type": "scatter",
         "mode": "lines", "line": {"color": "#d62728", "dash": "dash", "shape": "hv"}, "yaxis": "y5",
         "hovertemplate": "%{y:.0f} %<extra></extra>"},
        {"name": "baisse thermique maximale", "x": [times[min_temperature_index]], "y": [min_temperature],
         "customdata": [max_temperature_drop], "type": "scatter", "mode": "markers",
         "marker": {"color": "#d62728", "size": 10, "symbol": "diamond"}, "yaxis": "y4",
         "hovertemplate": "minimum %{y:.2f} °C<br>baisse %{customdata:.2f} °C<extra></extra>"},
        {"name": "poids", "x": times, "y": finite_series(report_weights),
         "type": "scatter",
         "mode": "lines", "line": {"color": "#9467bd", "width": 2.5}, "yaxis": "y7",
         "hovertemplate": "%{y:.1f} g<extra></extra>"},
        {"name": "volume depuis début", "x": times, "y": series("volume_ml"), "type": "scatter",
         "mode": "lines", "line": {"color": "#7f7f00"}, "yaxis": "y8",
         "hovertemplate": "%{y:.1f} ml<extra></extra>"},
        {"name": "pression indisponible · liaison", "x": pressure_bridge_x, "y": pressure_bridge_y,
         "type": "scatter", "mode": "lines", "line": {"color": "#d62728", "width": 1.5, "dash": "dash"},
         "yaxis": "y", "hoverinfo": "skip", "legendrank": 2},
    ]
    shapes = []
    annotations = []
    for start, _end, start_s, end_s, mode in phase_ranges(samples, duration_s):
        label, color = PHASE_STYLE.get(mode, (mode_label(mode), "#d9d9d9"))
        shapes.append({"type": "rect", "xref": "x", "yref": "paper", "x0": start_s, "x1": end_s,
                       "y0": 0, "y1": 1, "fillcolor": color, "opacity": 0.09, "line": {"width": 0},
                       "layer": "below"})
        if end_s - start_s >= 0.6:
            annotations.append({"xref": "x", "yref": "paper", "x": (start_s + end_s) / 2, "y": 1.015,
                                "text": label, "showarrow": False, "font": {"size": 11, "color": "#555"}})
    for index in transition_indices(samples):
        time_s = times[index]
        shapes.append({"type": "line", "xref": "x", "yref": "paper", "x0": time_s, "x1": time_s,
                       "y0": 0, "y1": 1, "line": {"color": "#667078", "width": 1, "dash": "dot"}})
    temperature_drop_shape_index = len(shapes) + 1
    shapes.extend([
        {"type": "line", "xref": "x", "yref": "y4", "x0": 0, "x1": duration_s,
         "y0": initial_temperature, "y1": initial_temperature,
         "line": {"color": "#777", "width": 1, "dash": "dash"}},
        {"type": "line", "xref": "x", "yref": "y4", "x0": times[min_temperature_index],
         "x1": times[min_temperature_index], "y0": min_temperature, "y1": initial_temperature,
         "line": {"color": "#d62728", "width": 2, "dash": "dot"}},
    ])
    annotations.append({"xref": "x", "yref": "y4", "x": duration_s, "y": initial_temperature,
                        "text": f"T₀ {initial_temperature:.2f} °C", "showarrow": False,
                        "xanchor": "right", "yanchor": "bottom", "font": {"size": 10, "color": "#666"}})
    panel_titles = [("Pression et pompe", 1.045), ("Débit tasse", 0.755), ("Température", 0.505),
                    ("Poids et volume", 0.235)]
    annotations.extend({"xref": "paper", "yref": "paper", "x": 0, "y": y, "text": f"<b>{title}</b>",
                        "showarrow": False, "xanchor": "left"} for title, y in panel_titles)
    layout = {
        "height": 920, "margin": {"l": 70, "r": 70, "t": 95, "b": 60},
        "paper_bgcolor": "#fff", "plot_bgcolor": "#fff", "hovermode": "x unified",
        "legend": {"orientation": "h", "y": 1.09, "x": 0}, "shapes": shapes, "annotations": annotations,
        "xaxis": {"title": "temps depuis le start (s)", "range": [0, duration_s], "showspikes": True,
                  "spikemode": "across", "spikesnap": "cursor", "gridcolor": "#e8ebed"},
        "yaxis": {"title": "bar", "domain": [0.78, 1], "gridcolor": "#e8ebed", "zeroline": False},
        "yaxis2": {"title": "pompe (%)", "overlaying": "y", "side": "right", "range": [0, 105]},
        "yaxis3": {"title": "g/s · ml/s", "domain": [0.53, 0.73], "gridcolor": "#e8ebed", "zeroline": False},
        "yaxis4": {"title": "°C", "domain": [0.28, 0.48], "gridcolor": "#e8ebed", "zeroline": False},
        "yaxis5": {"title": "chauffage (%)", "overlaying": "y4", "side": "right", "range": [0, 105]},
        "yaxis7": {"title": "poids (g)", "domain": [0, 0.23], "gridcolor": "#e8ebed", "zeroline": False},
        "yaxis8": {"title": "volume (ml)", "overlaying": "y7", "side": "right"},
    }
    payload = json.dumps({"traces": traces, "layout": layout,
                          "controls": {"temperatureTrace": 5, "dropTrace": 7,
                                       "dropShape": temperature_drop_shape_index,
                                       "dropTime": times[min_temperature_index]}}, ensure_ascii=False,
                         separators=(",", ":")).replace("</", "<\\/")
    return f"""<div class="plot-controls">
  <label for="ntc-delay"><strong>Retard NTC supposé</strong> <output id="ntc-delay-value">0.00 s</output></label>
  <input id="ntc-delay" type="range" min="-5" max="10" value="0" step="0.05">
  <small>Une valeur positive décale la température vers la gauche pour compenser une sonde supposée en retard. Le chauffage et les tables restent au temps brut.</small>
</div>
<div id="capture-plot" role="img" aria-label="Graphiques interactifs de la capture"></div>
<script>
  const plot = {payload};
  const plotDiv = document.getElementById("capture-plot");
  const delaySlider = document.getElementById("ntc-delay");
  const delayValue = document.getElementById("ntc-delay-value");
  const rawTemperatureTimes = [...plot.traces[plot.controls.temperatureTrace].x];
  Plotly.newPlot(plotDiv, plot.traces, plot.layout, {{responsive:true, displaylogo:false,
    modeBarButtonsToRemove:["lasso2d", "select2d"]}});
  delaySlider.addEventListener("input", () => {{
    const delay = Number(delaySlider.value);
    const correctedTimes = rawTemperatureTimes.map(time => time - delay);
    delayValue.value = `${{delay.toFixed(2)}} s`;
    Plotly.restyle(plotDiv, {{x:[correctedTimes, [plot.controls.dropTime - delay]]}},
                   [plot.controls.temperatureTrace, plot.controls.dropTrace]);
    const shapeUpdate = {{}};
    shapeUpdate[`shapes[${{plot.controls.dropShape}}].x0`] = plot.controls.dropTime - delay;
    shapeUpdate[`shapes[${{plot.controls.dropShape}}].x1`] = plot.controls.dropTime - delay;
    Plotly.relayout(plotDiv, shapeUpdate);
  }});
</script>"""


def render_report(capture: dict[str, Any], source: Path, weight_flow_window_s: float = 2.0) -> str:
    samples: list[dict[str, Any]] = capture["samples"]
    if not samples:
        raise RuntimeError("la capture ne contient aucun échantillon")
    duration_s = capture_duration_s(capture)
    taken_at = capture_datetime(capture, source)
    date_text = taken_at.strftime("%d.%m.%Y à %H:%M:%S %Z (UTC%z)") if taken_at else "date inconnue"
    stop_index = first_pump_stop(samples)
    stop_sample = samples[stop_index] if stop_index is not None else samples[-1]
    stop_s = number(stop_sample.get("t_ms")) / 1000
    hydraulic_start_index = next(
        (index for index, sample in enumerate(samples)
         if sample.get("mode") != "thermal_preheat"),
        0,
    )
    hydraulic_start_s = number(samples[hydraulic_start_index].get("t_ms")) / 1000
    hydraulic_duration_s = max(0.0, stop_s - hydraulic_start_s)
    origin = ORIGIN_LABELS.get(str(capture.get("origin", "")), str(capture.get("origin", "capture")))
    pressure_max = extrema(samples, "pressure_bar", signal="pressure")
    filling_flow_candidates = [
        (number(sample.get("flow_ml_s")), number(sample.get("t_ms")) / 1000)
        for sample in samples if sample.get("mode") == "filling" and valid(sample, "flow")
    ]
    filling_flow_candidates = [item for item in filling_flow_candidates if math.isfinite(item[0])]
    filling_flow_max = max(filling_flow_candidates, default=None, key=lambda item: item[0])
    elapsed_s = [number(sample.get("t_ms")) / 1000 for sample in samples]
    weights_g = [number(sample.get("weight_g")) for sample in samples]
    cup_flows = cup_flow_series(samples, weight_flow_window_s)
    cup_flow_candidates = [flow for time_s, flow in zip(elapsed_s, cup_flows)
                           if time_s <= stop_s and math.isfinite(flow)]
    cup_flow_max = max(cup_flow_candidates, default=float("nan"))
    stop_weight_g = number(stop_sample.get("weight_g"))
    cup_flow_average = ((stop_weight_g - weights_g[hydraulic_start_index]) / hydraulic_duration_s
                        if hydraulic_duration_s > 0 and math.isfinite(stop_weight_g)
                        and math.isfinite(weights_g[hydraulic_start_index])
                        else float("nan"))
    temp_values = [number(sample.get("boiler_temperature_c", sample.get("temperature_c")))
                   for sample in samples if valid(sample, "boiler")]
    temp_values = [value for value in temp_values if math.isfinite(value)]
    temp_drop = temp_values[0] - min(temp_values) if temp_values else float("nan")
    phase_names = [mode_label(item[4]) for item in phase_ranges(samples, duration_s)]
    hydraulic_samples = samples[hydraulic_start_index:]
    pressure_invalid_s = invalid_duration_s(hydraulic_samples, "pressure", stop_s)
    pressure_invalid_pct = (100 * pressure_invalid_s / hydraulic_duration_s
                            if hydraulic_duration_s > 0 else 0.0)
    active_phase_names = [name for name in phase_names if name != mode_label("cooldown")]
    cooldown_s = max(0.0, duration_s - stop_s)
    summary = (
        f"Cette {origin} parcourt {' → '.join(active_phase_names)} et s’arrête après {stop_s:.2f} s "
        f"({hydraulic_duration_s:.2f} s depuis le démarrage de la pompe). "
        f"Le poids est de {fmt(stop_weight_g, 1)} g au stop ; les variations pendant la récupération "
        f"ne sont pas utilisées. La pression est invalide pendant {pressure_invalid_s:.2f} s "
        f"({pressure_invalid_pct:.1f} % du temps d’infusion)."
    )
    hero_cards = [
        metric_card("Temps total", f"{stop_s:.2f} s",
                    f"dont {hydraulic_duration_s:.2f} s pompe démarrée"),
        metric_card("Poids au stop", f"{fmt(stop_weight_g, 1)} g", "cooldown exclu"),
        metric_card("Débit tasse", f"{fmt(cup_flow_average)} / {fmt(cup_flow_max)} g/s",
                    f"moyen / maximum avant stop, lissé sur {weight_flow_window_s:g} s"),
    ]
    diagnostic_cards = [
        metric_card("Pression maximale", f"{fmt(pressure_max[0])} bar" if pressure_max else "—",
                    f"valide, à t = {pressure_max[1]:.2f} s" if pressure_max else "mesure valide absente"),
        metric_card("Pression valide", f"{100 - pressure_invalid_pct:.1f} %",
                    f"{pressure_invalid_s:.2f} s invalides"),
        metric_card("Débit de remplissage", f"{fmt(filling_flow_max[0])} ml/s" if filling_flow_max else "—",
                    f"débitmètre, max à t = {filling_flow_max[1]:.2f} s" if filling_flow_max else "mesure absente"),
        metric_card("Baisse thermique max.", f"{fmt(temp_drop)} °C", "depuis la température initiale"),
    ]
    dropped = int(number(capture.get("dropped_samples"))) if math.isfinite(number(capture.get("dropped_samples"))) else 0
    quality = (f"{len(samples)} échantillons à une période annoncée de {capture.get('sample_period_ms', '—')} ms. "
               f"{dropped} échantillon{'s' if dropped != 1 else ''} perdu{'s' if dropped != 1 else ''}. "
               "Les pourcentages de validité ci-dessous portent sur start→stop et sont pondérés par le temps.")
    warning = ("<p class=\"warning\"><strong>Limite d’interprétation :</strong> la capture ne contient pas les seuils "
               "de configuration. Les mesures au changement de mode permettent d’identifier les causes plausibles "
               "(temps, pression ou poids), mais pas de prouver le critère déclencheur lorsqu’ils coïncident.</p>")
    escaped_source = html.escape(str(source))
    return f"""<!doctype html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Rapport CoffeeFlow · {html.escape(date_text)}</title>
  <script src="https://cdn.plot.ly/plotly-4.1.1.min.js" charset="utf-8"></script>
  <style>
    :root {{ color-scheme: light; --ink:#202124; --muted:#687076; --line:#dfe3e6; --paper:#fff; --wash:#f5f7f7; --accent:#7a4e2d; }}
    * {{ box-sizing:border-box; }}
    body {{ margin:0; background:#eef0ef; color:var(--ink); font:15px/1.5 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
    main {{ max-width:1500px; margin:0 auto; background:var(--paper); min-height:100vh; padding:44px clamp(20px,4vw,64px) 72px; }}
    h1 {{ margin:0 0 4px; font-size:clamp(28px,4vw,46px); letter-spacing:-.035em; }}
    h2 {{ margin:42px 0 12px; font-size:22px; }}
    p {{ max-width:1000px; }} .lede {{ font-size:18px; margin:20px 0 24px; }}
    .meta {{ color:var(--muted); display:flex; flex-wrap:wrap; gap:8px 24px; }}
    .metrics {{ display:grid; grid-template-columns:repeat(auto-fit,minmax(180px,1fr)); gap:12px; margin:24px 0; }}
    .hero-metrics {{ grid-template-columns:repeat(3,1fr); }}
    .hero-metrics .metric {{ padding:20px; border-color:#cbb8a9; background:#faf7f4; }}
    .hero-metrics .metric strong {{ font-size:clamp(24px,3vw,34px); }}
    .metric {{ background:var(--wash); border:1px solid var(--line); border-radius:10px; padding:14px 16px; }}
    .metric span,.metric small {{ display:block; color:var(--muted); }} .metric strong {{ display:block; font-size:22px; margin:3px 0; }}
    .plot {{ overflow-x:auto; border:1px solid var(--line); border-radius:12px; padding:10px; background:white; }}
    #capture-plot {{ width:100%; min-width:760px; }}
    .plot-controls {{ min-width:760px; display:grid; grid-template-columns:max-content minmax(260px,520px); gap:5px 18px; align-items:center; padding:10px 16px 0; }}
    .plot-controls output {{ display:inline-block; min-width:62px; margin-left:8px; color:var(--accent); font-variant-numeric:tabular-nums; }}
    .plot-controls input {{ width:100%; accent-color:var(--accent); }} .plot-controls small {{ grid-column:1 / -1; }}
    .table-wrap {{ overflow-x:auto; border:1px solid var(--line); border-radius:10px; }}
    table {{ width:100%; border-collapse:collapse; min-width:850px; }}
    th,td {{ padding:10px 12px; border-bottom:1px solid var(--line); text-align:left; vertical-align:top; white-space:nowrap; }}
    thead th {{ background:var(--wash); color:#4f575c; font-size:12px; letter-spacing:.04em; text-transform:uppercase; }}
    tbody tr:last-child th,tbody tr:last-child td {{ border-bottom:0; }} tbody th {{ font-weight:600; }}
    tr.transition {{ background:#fff9e8; }} tr.signal-warning {{ background:#fff0f0; }} tr.cooldown {{ color:var(--muted); background:#fafafa; }} .mode {{ padding:3px 7px; background:#eceff1; border-radius:5px; }}
    small {{ color:var(--muted); }} .warning {{ border-left:4px solid #d09b33; background:#fff9e8; padding:12px 16px; }}
    details {{ margin-top:14px; color:var(--muted); }} details ul {{ columns:3 180px; }} code {{ overflow-wrap:anywhere; }}
    @media (max-width:700px) {{ .hero-metrics {{ grid-template-columns:1fr; }} }}
    @media print {{ body {{ background:white; }} main {{ padding:0; }} .plot {{ break-inside:avoid; }} }}
  </style>
</head>
<body><main>
  <header>
    <h1>Rapport d’{html.escape(origin)} CoffeeFlow</h1>
    <div class="meta"><span>{html.escape(date_text)}</span><span>cycle {stop_s:.2f} s · hydraulique {hydraulic_duration_s:.2f} s</span><span>récupération technique {cooldown_s:.2f} s</span></div>
    <p class="lede">{html.escape(summary)}</p>
    <div class="metrics hero-metrics">{''.join(hero_cards)}</div>
    <div class="metrics">{''.join(diagnostic_cards)}</div>
  </header>

  <section aria-labelledby="graph-title">
    <h2 id="graph-title">Mesures</h2>
    <p><small>Survolez pour lire les valeurs, tirez horizontalement pour zoomer et double-cliquez pour réinitialiser. Les segments rouges pointillés relient deux mesures de pression valides séparées par une indisponibilité ; ils ne représentent pas des mesures intermédiaires. Les légendes sont cliquables.</small></p>
    <div class="plot">{interactive_plot(capture, weight_flow_window_s)}</div>
  </section>

  <section aria-labelledby="reading-title">
    <h2 id="reading-title">Lecture textuelle</h2>
    <p>{html.escape(textual_reading(samples, duration_s, weight_flow_window_s))}</p>
    {warning}
  </section>

  <section aria-labelledby="phases-title">
    <h2 id="phases-title">Chronologie des phases</h2>
    <p>Les phases jusqu’au stop constituent l’infusion. La ligne récupération, atténuée, correspond uniquement à la fenêtre technique post-arrêt.</p>
    <div class="table-wrap"><table>
      <thead><tr><th>Phase</th><th>Intervalle</th><th>Durée</th><th>Pression</th><th>Poids</th><th>Volume</th><th>Température</th></tr></thead>
      <tbody>{phase_table(samples, duration_s)}</tbody>
    </table></div>
  </section>

  <section aria-labelledby="points-title">
    <h2 id="points-title">Valeurs aux points clés</h2>
    <p>Pour chaque transition, les cellules donnent « dernière valeur avant → première valeur après ». Le temps est celui du premier échantillon portant le nouveau mode. Le repère à 1 s utilise l’échantillon temporellement le plus proche.</p>
    <div class="table-wrap"><table>
      <thead><tr><th>Événement</th><th>Temps</th><th>Mode</th><th>Pression</th><th>Débit tasse</th><th>Volume</th><th>Poids</th><th>Température</th><th>Pompe dem./rapp.</th><th>Chauffage</th></tr></thead>
      <tbody>{key_points_table(samples, weight_flow_window_s)}</tbody>
    </table></div>
  </section>

  <section aria-labelledby="quality-title">
    <h2 id="quality-title">Validité des signaux</h2>
    <p>{html.escape(quality)}</p>
    <div class="table-wrap"><table>
      <thead><tr><th>Signal</th><th>Échantillons valides</th><th>Temps valide</th><th>Interruptions</th><th>Temps invalide</th><th>Plus longue</th></tr></thead>
      <tbody>{validity_table(samples, stop_s, hydraulic_start_s)}</tbody>
    </table></div>
    {invalid_pressure_details(samples, stop_s, hydraulic_start_s)}
    <h2>Provenance</h2>
    <p><small>Source : <code>{escaped_source}</code> · schéma <code>{html.escape(str(capture.get('schema', 'inconnu')))}</code>. La capture brute dure {duration_s:.2f} s, dont {cooldown_s:.2f} s après le stop. Le débit tasse exclut la précharge thermique, le cooldown et tout poids négatif ; son calcul ne modifie pas le JSON brut. Sa courbe utilise une dérivée centrée sur {weight_flow_window_s:g} s et son maximum est recherché jusqu’au stop.</small></p>
  </section>
</main></body></html>
"""


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="fichier JSON produit par download_hf_capture.py")
    parser.add_argument("--output", type=Path, help="fichier HTML (défaut : même nom que la capture)")
    parser.add_argument("--weight-flow-window-s", type=float, default=2.0,
                        help="fenêtre centrée du débit balance, en secondes (défaut : 2)")
    args = parser.parse_args(argv)
    output = args.output or args.capture.with_suffix(".html")
    try:
        capture = validate_capture(json.loads(args.capture.read_text(encoding="utf-8")))
        report = render_report(capture, args.capture, args.weight_flow_window_s)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(report, encoding="utf-8")
    except (OSError, json.JSONDecodeError, RuntimeError, ValueError) as error:
        print(f"erreur: {error}", file=sys.stderr)
        return 1
    print(f"rapport écrit dans {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

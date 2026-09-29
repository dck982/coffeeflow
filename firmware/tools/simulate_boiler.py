#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy", "scipy"]
# ///
"""Rejoue des lois de chauffe sur un modèle thermique de la NTC chaudière.

Le modèle prédit la **NTC** (température utilisateur), pas l'eau au groupe.
Il superpose deux chemins linéaires :

* chauffe : énergie commandée (``heating_power_pct`` × 12 J/%·s), retard pur
  ``dh`` puis trois premiers ordres ``th`` ;
* eau admise : volume du débitmètre × 4,18 × 70,5 K (eau à 24,5 °C portée
  à 95 °C réels), retard pur ``dw`` puis trois premiers ordres ``tw``.

Chaque chemin porte une part locale ``ah``/``aw`` qui se mélange au reste de
la chaudière en ``tm`` s. ``C`` est la capacité thermique effective, ``kl``
les pertes. La pente mesurée sur les 5 premières secondes de chaque capture
décroît en 20 s : elle représente l'histoire antérieure à la capture.

Les scénarios gardent l'hydraulique de la capture (débit, durées de phase)
et remplacent la précharge et la commande pendant l'écoulement. La
récupération est simulée à 0 %, comme dans les captures de référence.

Exemples :
    uv run firmware/tools/simulate_boiler.py
    uv run firmware/tools/simulate_boiler.py --fit
    uv run firmware/tools/simulate_boiler.py --capture captures/260929-074002.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np

DT = 0.1
HOLD_PCT = 3.5
WATER_DELTA_K = 70.5
CAPTURES = Path(__file__).resolve().parents[2] / "captures"
FIT_BREWS = ("260928-130104.json", "260929-074002.json")
FIT_MONITOR = "monitor-heating-20260928-095631-275212.json"
NAMES = ("C", "dh", "th", "dw", "tw", "kl", "ah", "aw", "tm")
# Ajustement du 29 septembre sur FIT_BREWS + FIT_MONITOR (--fit pour refaire).
DEFAULT_PARAMS = np.array([1.471, 3.95, 4.560, 2.021, 1.466, 0.0012, 1.100, 0.798, 4.713])
FLOWING = ("filling", "preinfusion", "infusion")


def load_hf(path: Path) -> dict:
    samples = json.loads(path.read_text())["samples"]
    t = np.array([s["t_ms"] / 1000 for s in samples])
    grid = np.arange(0, t[-1], DT)
    idx = np.clip(np.searchsorted(t, grid, side="right") - 1, 0, None)
    return {
        "name": path.name,
        "T": np.interp(grid, t, [s["boiler_temperature_c"] for s in samples]),
        "cmd": np.array([samples[i]["heating_power_pct"] for i in idx]),
        "vol": np.interp(grid, t, [s["volume_ml"] for s in samples]),
        "flow": np.array([samples[i]["flow_ml_s"] for i in idx]),
        "mode": [samples[i]["mode"] for i in idx],
    }


def load_monitor(path: Path) -> dict:
    samples = json.loads(path.read_text())["samples"]
    t = np.array([s["elapsed_s"] for s in samples])
    grid = np.arange(0, t[-1], DT)
    idx = np.clip(np.searchsorted(t, grid, side="right") - 1, 0, None)
    power = [s["telemetry"]["heating"]["power_pct"] for s in samples]
    return {
        "name": path.name,
        "T": np.interp(grid, t, [s["telemetry"]["temperature"]["boiler"]["c"] for s in samples]),
        "cmd": np.array([power[i] for i in idx], dtype=float),
        "vol": np.zeros_like(grid),
        "flow": np.zeros_like(grid),
        "mode": ["idle"] * len(grid),
    }


def lag(x: np.ndarray, dead_s: float, tau_s: float, order: int) -> np.ndarray:
    k = int(round(dead_s / DT))
    y = np.concatenate([np.full(k, x[0]), x[: len(x) - k]]) if k > 0 else x.copy()
    a = DT / (tau_s + DT)
    for _ in range(order):
        z = np.empty_like(y)
        z[0] = y[0]
        for i in range(1, len(y)):
            z[i] = z[i - 1] + a * (y[i] - z[i - 1])
        y = z
    return y


def model(p: np.ndarray, cap: dict, cmd: np.ndarray, water_ml_s: np.ndarray) -> np.ndarray:
    c, dh, th, dw, tw, kl, ah, aw, tm = p
    heat = lag(np.cumsum(cmd * 12 * DT) / 1000, dh, th, 3)
    cold = lag(np.cumsum(water_ml_s * 4.18 * WATER_DELTA_K * DT) / 1000, dw, tw, 3)
    x = (heat - cold) / c
    x += (ah * (heat - lag(heat, 0, tm, 1)) - aw * (cold - lag(cold, 0, tm, 1))) / c
    t0 = cap["T"][0]
    t = np.arange(len(x)) * DT
    initial_slope = (cap["T"][50] - cap["T"][0]) / 5.0
    loss = -kl * (t0 - 24.5) / 1000 / c
    return t0 + x + loss * t + (initial_slope - loss) * 20 * (1 - np.exp(-t / 20))


def replay(p: np.ndarray, cap: dict) -> np.ndarray:
    return model(p, cap, cap["cmd"], np.gradient(cap["vol"], DT).clip(0))


def rms(p: np.ndarray, cap: dict) -> float:
    return float(np.sqrt(np.mean((replay(p, cap) - cap["T"]) ** 2)))


def fit(caps: list[dict], start: np.ndarray) -> np.ndarray:
    from scipy.optimize import minimize

    def cost(p: np.ndarray) -> float:
        if min(p) < 0 or p[0] < 0.3 or p[2] < 0.1 or p[4] < 0.1 or p[8] < 0.5:
            return 1e9
        return sum(rms(p, c) ** 2 for c in caps)

    return minimize(cost, start, method="Nelder-Mead",
                    options={"maxiter": 10000, "xatol": 1e-4, "fatol": 1e-7}).x


def need_pct(flow_ml_s: float, cap_pct: float) -> float:
    """Piste 1 : maintien + chaleur de l'eau admise, bornée."""
    return min(cap_pct, HOLD_PCT + flow_ml_s * 4.18 * WATER_DELTA_K / 12)


def scenario(p, cap, preheat_s, preheat_pct, cap_pct, cut_s=0.0, lead_s=None):
    """Précharge puis appoint proportionnel au débit mesuré.

    ``cut_s`` coupe la chauffe ``cut_s`` secondes avant l'arrêt de la pompe.
    ``lead_s`` (oracle) commande le besoin du débit ``lead_s`` s plus tard ;
    la précharge est alors ce besoin anticipé.
    """
    i0 = next(i for i, m in enumerate(cap["mode"]) if m != "thermal_preheat")
    water = np.gradient(cap["vol"], DT).clip(0)[i0:]
    flow = cap["flow"][i0:]
    modes = list(cap["mode"][i0:])
    flowing = [m in FLOWING for m in modes]
    stop = max(i for i, m in enumerate(modes) if m == "infusion")
    cmd = np.zeros(len(water))
    if lead_s is None:
        for j in range(len(water)):
            if flowing[j] and j <= stop - int(cut_s / DT):
                cmd[j] = need_pct(flow[j], cap_pct)
        n0 = int(round(preheat_s / DT))
        pre = np.full(n0, preheat_pct)
    else:
        n0 = int(lead_s / DT)
        ahead = lambda k: need_pct(flow[k], cap_pct) if k < len(flow) and flowing[k] else 0.0
        cmd = np.array([ahead(j + n0) for j in range(len(water))])
        pre = np.array([ahead(k) for k in range(n0)])
    cmd = np.r_[pre, cmd]
    water = np.r_[np.zeros(n0), water]
    modes = ["preheat"] * n0 + modes
    return summarize(model(p, cap, cmd, water), modes, cmd, n0)


def summarize(T, modes, cmd, n0):
    infusion = [i for i, m in enumerate(modes) if m == "infusion"]
    stop = infusion[-1]
    end = min(len(T) - 1, stop + int(30 / DT))
    return {
        "peak": float(T[: infusion[0]].max()),
        "min": float(T[:end].min()),
        "end": float(T[end]),
        "energy_kj": float(np.sum(cmd) * 12 * DT / 1000),
    }


def rebound_timing(p, cap):
    """Délai entre l'arrêt de la pompe et le maximum du rebond (commande réelle)."""
    ext = int(40 / DT)
    padded = dict(cap, T=np.r_[cap["T"], np.full(ext, cap["T"][-1])])
    T = model(p, padded, np.r_[cap["cmd"], np.zeros(ext)],
              np.r_[np.gradient(cap["vol"], DT).clip(0), np.zeros(ext)])
    stop = max(i for i, m in enumerate(cap["mode"]) if m == "infusion")
    seg = T[stop: stop + int(60 / DT)]
    k = int(np.argmax(seg))
    within = next(j for j in range(len(seg)) if seg[j] >= seg[k] - 0.1)
    return k * DT, within * DT


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--fit", action="store_true", help="réajuster le modèle")
    ap.add_argument("--capture", type=Path, action="append",
                    help="capture HF d'infusion à rejouer (défaut : captures d'ajustement)")
    args = ap.parse_args()

    brews = [load_hf(CAPTURES / n) for n in FIT_BREWS]
    monitor = load_monitor(CAPTURES / FIT_MONITOR)
    p = fit(brews + [monitor], DEFAULT_PARAMS) if args.fit else DEFAULT_PARAMS
    print("paramètres :", ", ".join(f"{n}={v:.3g}" for n, v in zip(NAMES, p)))
    for cap in brews + [monitor]:
        print(f"  écart RMS {cap['name']}: {rms(p, cap):.2f} °C")

    targets = [load_hf(c) for c in args.capture] if args.capture else brews
    for cap in targets:
        m = replay(p, cap)
        i_inf = cap["mode"].index("infusion")
        print(f"\n== {cap['name']} — départ {cap['T'][0]:.2f} °C")
        print(f"{'':48} {'pic':>6} {'min':>6} {'+30 s':>6} {'kJ':>5}")
        print(f"{'mesure':48} {cap['T'][:i_inf].max():6.2f} {cap['T'].min():6.2f}")
        print(f"{'modèle, commande réelle':48} {m[:i_inf].max():6.2f} {m.min():6.2f}")
        rows = [(f"précharge {s:g} s à 90 %, débit ≤ 90 %", dict(preheat_s=s, preheat_pct=90, cap_pct=90))
                for s in (0, 3, 6, 8, 10)]
        rows += [("précharge 3 s à 90 %, débit ≤ 100 %", dict(preheat_s=3, preheat_pct=90, cap_pct=100)),
                 ("précharge 3 s à 90 %, débit ≤ 70 %", dict(preheat_s=3, preheat_pct=90, cap_pct=70))]
        rows += [(f"précharge {s:g} s à 90 %, débit ≤ 90 %, coupe {c:g} s", dict(preheat_s=s, preheat_pct=90, cap_pct=90, cut_s=c))
                 for s, c in ((3, 8), (6, 8), (8, 8), (8, 11), (10, 11))]
        rows += [(f"oracle : besoin avancé de {l:g} s", dict(preheat_s=0, preheat_pct=0, cap_pct=90, lead_s=l))
                 for l in (8, 11)]
        for label, kw in rows:
            r = scenario(p, cap, **kw)
            print(f"{label:48} {r['peak']:6.2f} {r['min']:6.2f} {r['end']:6.2f} {r['energy_kj']:5.1f}")
        peak_s, within_s = rebound_timing(p, cap)
        print(f"rebond : maximum {peak_s:.1f} s après la pompe, à 0,1 °C près dès {within_s:.1f} s")


if __name__ == "__main__":
    main()

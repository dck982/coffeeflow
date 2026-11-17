"""Vérifie le montage cellule de charge -> M5Stack Unit Weight I2C -> AtomS3.

Lancer avec `mpremote run tests/test_scale_check.py`, la cellule à vide. Le script
enchaîne trois phases minutées et annonce chacune :

  1. repos   : la cellule ne doit pas être saturée, ni figée, ni bruyante
  2. appui   : appuyer franchement sur le bout chargé, le brut doit bouger
  3. relâche : lâcher, le brut doit revenir près du zéro de la phase 1

Le verdict se lit sur les comptes bruts de l'ADC, qui ne dépendent pas de
l'étalonnage (GAP) stocké dans l'unité.
"""

import struct
import time
from machine import Pin, SoftI2C

ADDR = 0x26
REG_RAW_ADC = 0x00
REG_WEIGHT = 0x10
REG_GAP = 0x40
REG_FILTER = 0x80
REG_VERSION = 0xFE

# Grove de l'AtomS3
i2c = SoftI2C(scl=Pin(1), sda=Pin(2), freq=100_000)

ADC_ZERO = 1 << 23
# HX711 : 24 bits signés ; au-delà, un fil de pont est coupé ou inversé
ADC_LIMIT = 8_000_000
# Mesuré : une masse de 100 g déplace le brut d'environ 4200 comptes, pour un bruit
# au repos d'environ 130 comptes crête-à-crête ; 2000 comptes laisse passer une
# masse de 100 g sans confondre le bruit avec une charge
PRESS_MIN = 2_000
# Le firmware moyenne déjà ; au repos l'écart crête-à-crête reste de quelques
# dizaines de comptes. Au-delà, le pont est mal câblé ou la cellule touche quelque chose
REST_P2P_MAX = 2_000
# Le zéro doit revenir à quelques fois le bruit près : sinon la cellule frotte
RETURN_MAX = 1_000

REST_S = 3
PRESS_S = 6
RELEASE_S = 3
PERIOD_MS = 50

failures = []


def check(ok, label):
    print("  [{}] {}".format("OK " if ok else "KO ", label))
    if not ok:
        failures.append(label)
    return ok


def raw_adc():
    # L'unité publie le HX711 décalé de 2^23 : entrée nulle = 8 388 608. On recentre
    # pour que zéro soit zéro et que la saturation soit à ±2^23
    return int.from_bytes(i2c.readfrom_mem(ADDR, REG_RAW_ADC, 4), "little") - ADC_ZERO


def weight_g():
    return struct.unpack("<f", i2c.readfrom_mem(ADDR, REG_WEIGHT, 4))[0]


def sample(seconds, show=False):
    vals = []
    end = time.ticks_add(time.ticks_ms(), seconds * 1000)
    last_show = 0
    while time.ticks_diff(end, time.ticks_ms()) > 0:
        v = raw_adc()
        vals.append(v)
        if show and time.ticks_diff(time.ticks_ms(), last_show) >= 500:
            last_show = time.ticks_ms()
            print("    brut={:>9}  poids={:>9.1f} g".format(v, weight_g()))
        time.sleep_ms(PERIOD_MS)
    return vals


def mean(vals):
    return sum(vals) / len(vals)


def countdown(msg, n=3):
    print(msg)
    for i in range(n, 0, -1):
        print("   ", i)
        time.sleep(1)


print("== Bus I2C")
found = i2c.scan()
print("  scan:", [hex(a) for a in found])
if not check(ADDR in found, "Unit Weight présente en 0x26"):
    raise SystemExit

version = i2c.readfrom_mem(ADDR, REG_VERSION, 1)[0]
filt = i2c.readfrom_mem(ADDR, REG_FILTER, 3)
gap = struct.unpack("<f", i2c.readfrom_mem(ADDR, REG_GAP, 4))[0]
print("  firmware v{}  filtres lp={} avg={} ema={}  GAP={:.3f}".format(
    version, filt[0], filt[1], filt[2], gap))

print()
print("== 1. Repos ({} s) — ne rien toucher".format(REST_S))
time.sleep_ms(500)
rest = sample(REST_S)
zero = mean(rest)
p2p = max(rest) - min(rest)
print("  moyenne={:.0f}  crête-à-crête={}  n={}".format(zero, p2p, len(rest)))
check(abs(zero) < ADC_LIMIT, "brut hors saturation (|{:.0f}| < {})".format(zero, ADC_LIMIT))
check(p2p > 0, "brut vivant (il varie d'une lecture à l'autre)")
check(p2p < REST_P2P_MAX, "bruit au repos < {} comptes".format(REST_P2P_MAX))

print()
countdown("== 2. Appui ({} s) — appuyer sur le bout chargé ou poser la masse dans…".format(PRESS_S))
press = sample(PRESS_S, show=True)
peak = max(press, key=lambda v: abs(v - zero))
delta = peak - zero
print("  écart max={:+.0f} comptes".format(delta))
if check(abs(delta) > PRESS_MIN, "la cellule répond à l'appui (> {} comptes)".format(PRESS_MIN)):
    check(abs(peak) < ADC_LIMIT, "pas de saturation sous l'appui")
    print("  sens: {}".format("positif" if delta > 0 else
                              "NÉGATIF — fils S+/S- ou E+/E- inversés, ou cellule retournée"))

print()
countdown("== 3. Relâche ({} s) — lâcher ou retirer la masse dans…".format(RELEASE_S), 2)
time.sleep_ms(500)
back = mean(sample(RELEASE_S))
drift = back - zero
print("  retour={:.0f}  écart au zéro={:+.0f}".format(back, drift))
check(abs(drift) < RETURN_MAX, "retour au zéro (< {} comptes)".format(RETURN_MAX))

print()
if failures:
    print("ÉCHEC — {} point(s) :".format(len(failures)))
    for f in failures:
        print("  -", f)
else:
    print("MONTAGE OK")

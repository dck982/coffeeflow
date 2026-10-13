# CoffeeFlow combined board

**Combined on October 13, 2026:** the former `accontrol` project now also
carries the sensors controller, so one board holds both the 230 V side
(valve and pump) and the low-voltage side (XIAO ESP32-S3, CAN, sensors,
boiler SSR output). The AC board alone had been shelved on October 11: it did
not fit the AC enclosure (`print/parts/boitier_ac.py`, 68 × 31.6 × 25.4 mm
inside), and a single cable to the sensors board did not justify it. Merging
both boards removes that cable altogether; the enclosure remains to be
decided (see Open points).

The schematic has three sheets. `combined.kicad_sch` is the AC side, as
below. `combined_controller.kicad_sch` (XIAO, 5 V input, status LED, pressure
ADC) and `combined_interfaces.kicad_sch` (CAN, I2C pull-ups and TMP102,
flowmeter, boiler SSR output) come from `../sensors/`, whose design notes in
`sensors_design.md` still apply to those circuits, with the references
renumbered as listed in "Low-voltage side from sensors". `combined.pdf` is a
generated review copy. `combined.kicad_pcb` has a first placement and a
partial routing of the valve channel, with clearance violations left open;
it does not have the sensors footprints yet (Update PCB from Schematic).
`combined_bom.csv` lists values, footprints and design notes.
D3, F1, J2–J4, Q2 and Q4 have their JLCPCB parts, as do all the sensors
parts; the other MPN and LCSC fields of the AC side are still to fill.

The board switches the two original 230 V actuators from 3.3 V logic: the
solenoid valve (OLAB coil, 15 VA, about 65 mA) and the vibration pump (OLAB
Silent Green 35 W, about 0.15 A RMS). It replaces the M5Stack Unit SSR on the
valve and the RBDimmer 4 A on the pump. Both channels reuse those modules'
circuits. The pump channel follows the RobotDyn dimmer. Unlike the RBDimmer in
DimmerLink mode, the pump channel has no microcontroller of its own: the
XIAO receives the zero-cross signal and times the pump firing itself.

## Connectors

| Connector | Type | Pin 1 | Pin 2 | Pin 3 |
|---|---|---|---|---|
| J1, 5 V power | JST XH 2 | GND | +5V | — |
| J2, mains in | HT396R 2 | L | N | — |
| J3, valve | HT396R 2 | L switched | N | — |
| J4, pump | HT396R 2 | L switched | N | — |
| J5, CAN | JST XH 2 | CAN_L | CAN_H | — |
| J6, flow | JST XH 3 | GND | +5V | FLOW_PULSE |
| J8, boiler SSR | JST XH 2 | GND | BOILER_5V | — |
| J10, pressure | JST XH 3 | +3V3 | GND | PRESSURE |

The former 5-pin control header (GND, 3V3, DIM, ZC, SSR) is gone: DIM, ZC
and SSR run as tracks to the XIAO sockets, and 3V3 comes from the XIAO.
The sensors board's two Grove I2C headers and its VALVE header are not
carried over either: the dimmer and the valve SSR are now on board.

J2, J3 and J4 are HT396R-3.96-2P right-angle headers (LCSC C577799, a JST VH
clone). Their footprint and 3D model come from LCSC, in `Combined_Local`;
the courtyard was enlarged to cover the pads and the body. The pins sit at
the back of the body, which extends 10 mm forward: place the connectors with
the pins inside the board and the mating face at the edge.

Only L is switched. N goes straight from J2 to J3 and J4 pin 2. On the board,
the zero-cross bridge is the only circuit connected to N.

F1 (2 A fast, 250 V) is in series with L right after J2 (net L_IN between
them) and protects the whole mains side. It is an AEM MF2410F2.000TM (LCSC
C83797), assembled by JLCPCB on the LCSC `F2410` footprint in
`Combined_Local`, whose courtyard was enlarged to cover the pads. The pads
leave 3 mm between L_IN and L.

- Fast-acting: it clears 20 A (10 In) in 1 to 10 ms. The valve and pump
  draw no inrush that would call for a time-lag fuse.
- Breaking capacity: 100 A at 250 VAC. A short after F1 on a household
  outlet can exceed that (230 V / 0.5 Ω loop ≈ 460 A); the circuit breaker
  then clears it (a 10 A type B breaker trips magnetically from 50 A).
- Its melting I²t (3 A²s) is about that of a BT136S, so F1 does not save a
  triac from a shorted load. It protects the board and the wiring.
- Soldered, not replaceable. If it blows, something else has already failed.

## Low-voltage side: SSR and DIM inputs

Both channels share the same input stage. The valve channel uses Q1, R1–R4, D1
and U1; the pump channel uses Q3, R8–R11, D2 and U2.

- R2 / R9 (4.7 kΩ) pull the input to GND, so the channel stays off while the
  XIAO is unplugged or its pin floats during boot. They replace the 10 kΩ
  VALVE pull-down of the sensors board, which is not carried over.
- R1 / R8 (1 kΩ) limit the base current of Q1 / Q3: about 2.5 mA from a
  3.3 V GPIO. The transistor turns on above about 0.85 V on the input.
- Q1 / Q3 (SS8050) are low-side switches. They sink the optotriac LED current
  and the indicator LED current. That current comes from the XIAO 3V3 output,
  not from the GPIO, which only supplies the base current. With 2.5 mA of base
  current for about 12 mA of collector current, the transistor saturates.
- R3 / R10 (180 Ω) set the optotriac LED current:
  (3.3 − 1.2 − 0.1) / 180 ≈ 11 mA typical, at least 8 mA worst case (LED
  forward voltage 1.5 V max). U1 and U2 are guaranteed to trigger at 5 mA;
  onsemi recommends any current between that and the 60 mA maximum.
- D1 / D2 (red) with R4 / R11 (1 kΩ) light at about 1.2 mA when the channel is
  commanded. They sit in parallel with the optotriac LED, so they show the
  command, not whether the load is actually conducting.

Load on the 3V3 rail: about 12 mA per active channel, plus 0.25 mA for the ZC
pull-up. That is 25 mA with both channels on. C1 (1 µF) decouples 3V3 for this side.

## Mains side: valve channel (zero-cross)

U1 (MOC3043M) is a zero-cross optotriac. It does not carry the valve current.
Its output triac only drives a pulse into the gate of Q2 (BT136S), and
Q2 connects L to L_VALVE.

- Gate path: L → R5 (360 Ω) → U1 pins 6–4 → R6 (330 Ω) → Q2 gate →
  L_VALVE. U1 only turns on while the voltage across it is below its inhibit
  voltage (20 V max). Once on, it stays on above that voltage as long as its
  current exceeds its holding current (400 µA typical). The gate current then
  grows with the line voltage until Q2 triggers. With the standard BT136S gate
  (35 mA max in the two quadrants used here), that happens below about 28 V,
  0.3 ms after the voltage zero. U1 then carries at most about 35 mA, well
  under its 100 mA repetitive peak rating. Once Q2 conducts, only about 1.5 V
  remains across it, and the U1 current drops to zero.
- R5 + R6 = 690 Ω limit the U1 current if it ever fires at a high
  instantaneous voltage: 325 V / 690 Ω ≈ 0.47 A, below its 1 A non-repetitive
  surge rating. The values come from the M5Stack design.
- While SSR is high, U1 fires Q2 again at the start of every half-cycle. When
  SSR goes low, Q2 turns off at the next current zero. Because the coil is
  inductive, the current zero comes after the voltage zero. Q2 actually drops
  out when the current falls below its holding current (15 mA max), about
  0.5 ms before the current zero at the valve's 92 mA peak.
- R7 (39 Ω) + C2 (10 nF, 1 kV) form a snubber across Q2. At turn-off on an
  inductive load, the voltage across the triac jumps quickly. The snubber
  limits that dV/dt so Q2 does not re-trigger on its own. At 50 Hz, C2 lets
  about 0.7 mA through the valve while it is off (230 V / 318 kΩ). A solenoid
  ignores this.

## Mains side: pump channel (phase control)

U2 (MOC3023M) has no zero-cross detection: it fires Q4 as soon as DIM goes
high. Q4 then conducts until the load current reaches zero at the end of the
half-cycle.

- Gate path: L → R12 (1 kΩ) → U2 pins 6–4 → Q4 gate directly, as in the
  RobotDyn design. Firing at the voltage peak is normal operation here:
  325 V / 1 kΩ ≈ 0.33 A for the few microseconds before Q4 turns on. U2 is
  rated for repeated 1 A pulses of 100 µs at 120 per second.
- Q4 only stays on once the load current exceeds its latching current: 20 mA
  max at 25 °C in the quadrants used, about 30 % more at 0 °C. The pump
  current rises slowly because the pump is inductive, so the gate must stay
  driven for a while after firing. See the firmware rules below.
- R13 (39 Ω) + C3 (10 nF, 1 kV) form the snubber across Q4, as on the valve
  channel. Q4 fires mid half-cycle, so C3 dumps its charge through R13 at
  each firing: up to 325 V / 39 Ω ≈ 8.3 A for about 0.4 µs (RC = 390 ns),
  and ½CV² ≈ 0.53 mJ, 100 times per second. With the recharge, R13
  dissipates about 0.1 W at worst. R13 is therefore a 1210 (Yageo
  RC1210JR-0739RL, LCSC C728404, 0.5 W), like the RobotDyn snubber
  resistor. Its datasheet gives no pulse rating; anti-surge 1210 parts
  (ROHM ESR25, Panasonic ERJ-P14) were out of stock at JLCPCB. R7 stays a
  1206: Q2 fires at the zero crossing, when C2 is nearly empty.

## Triac dissipation

The BT136S on-state model is 1.27 V + 0.091 Ω, and the junction-to-ambient
resistance is 75 K/W on the datasheet's FR4 footprint. The pump costs Q4
about 0.17 W, a 13 K rise; the valve costs Q2 under 0.1 W. Neither needs a
heat sink beyond the DPAK tab copper.

## Zero-cross detection

R15 and R14 (2 × 120 kΩ) in series feed the AC input of the D3 bridge (ABS6).
D3 feeds the U3 LED (EL817C). The U3 phototransistor pulls ZC to GND, and R16
(10 kΩ) pulls ZC up to 3V3.

- The bridge rectifies both half-waves, so the LED conducts on each one and
  never sees reverse voltage. Everlight does not specify the EL817 for reverse
  operation.
- The LED current is |V| / 240 kΩ, at most 325 / 240k ≈ 1.35 mA. The two
  resistors split the voltage and power: 160 V peak and 0.11 W each, which a
  single resistor could not take (0.22 W, 325 V). They dissipate whenever
  the machine is on, so they are 1210 parts (UNI-ROYAL 1210W2F1203T5E, LCSC
  C422421: 0.5 W, 200 V working), at 22 % of their rating, as on the RobotDyn
  dimmer. D3 is rated 600 V
  repetitive reverse voltage and carries only these microamps to milliamps.
- **ZC is low most of the time and goes high in a pulse centred on each zero
  crossing, at 100 Hz.** Near zero, the line voltage is too low for the LED to
  hold the phototransistor on against R16.
- Threshold: to pull ZC below the ESP32-S3 input low level (0.25 × 3.3 V ≈
  0.8 V), the phototransistor must sink about 0.25 mA. The EL817C CTR is 200
  to 400 % at 5 mA, but falls to about a third of that at 0.5 mA (datasheet
  Fig. 2) and lower still below, where the curve stops. The LED therefore
  needs about 0.25 to 0.4 mA, which means |V| ≈ 60 to 95 V once the bridge and
  LED drops (about 2 V) are added.
- Expected pulse width: **about 1.2 to 1.9 ms at 25 °C**, wider when hot,
  since the CTR falls by about 20 % at 60 °C. At the 325 V peak, the LED
  current still gives several times the needed collector current, so ZC is a
  clean low between pulses.
- The phototransistor turns off more slowly than it turns on (around 150 µs
  with 10 kΩ, datasheet Fig. 6), so the ZC rising edge lags. Firmware takes the
  zero crossing as the midpoint between the rising and falling edges of the
  pulse; the turn-off lag makes it slightly late. Measure the width and offset
  with a scope on the first assembled board and record them here.

## Pump firing in firmware

The XIAO replaces the DimmerLink timing:

1. Timestamp both edges of each ZC pulse. The zero crossing is their midpoint.
2. Raise DIM after the firing delay for the requested power: short delay =
   high power. The earliest firing point is the ZC falling edge, about 0.6 to
   0.9 ms after the zero crossing. Skipping that start of the half-cycle
   costs about 0.2 % of the power, because the voltage is still low there.
3. Hold DIM high until the next ZC rising edge, then lower it. This keeps the
   gate driven while the inductive pump current builds up past the latching
   current of Q4. Lowering DIM before the zero crossing prevents U2 from
   firing again at the start of the next half-cycle.
4. Full power: hold DIM high continuously; U2 then fires at the start of each
   half-cycle. Off: hold DIM low.
5. If ZC pulses stop (mains missing), hold DIM low.

SSR needs no timing: U1 does its own zero-cross synchronisation.

## Low-voltage side from sensors

Everything on the sensors board except its Grove I2C headers (J4, J5) and its
VALVE header (J7, with its R17 pull-down) is copied with unchanged values and
connections. The 4.7 kΩ SDA/SCL pull-ups stay: the TMP102 and the ADS1115
are on the bus. References that clashed with the AC side were renumbered:

| sensors | combined | Part |
|---|---|---|
| U1 | U7 | XIAO ESP32-S3 (schematic only, plugs into J11/J12) |
| D2 | D4 | Green status LED |
| J3 | J5 | CAN connector |
| Q2 | Q5 | AO3401A, boiler output |
| Q3 | Q6 | 2N7002, boiler output |
| R11 | R17 | TJA1051 TXD pull-up |
| R12 | R18 | CAN termination, 120 Ω 1206 |
| R13, R14 | R20, R23 | SDA, SCL pull-ups |
| R15 | R24 | FLOW pull-up |

The other references (C8–C15, H1, J1, J6, J8, J10–J12, R19, R21, R22, R25,
R26, U4–U6) keep their sensors numbers. The XIAO footprint and its socket
footprint are copied into `Combined_Local`; `sym-lib-table` points to the
local Seeed XIAO symbol library, as in sensors.

XIAO pins used by the AC side, following the plan already noted in
`sensors_design.md`:

| Signal | GPIO | Seeed name | U7 / J11–J12 pin |
|---|---:|---|---|
| DIM | 1 | D0 | U7 / 1, J11 / 1 |
| ZC | 3 | D2 | U7 / 3, J11 / 3 |
| SSR | 9 | D10 | U7 / 11, J12 / 4 (former VALVE_CMD) |

The other pins keep their sensors assignments; only D6/GPIO43 stays free.
ZC is hard-wired to the U3 phototransistor and R16, so it needs no internal
pull-down. GPIO3 is a JTAG strapping pin, read only when the
`STRAP_JTAG_SEL` eFuse is burnt, which it is not by default; U3 may then
drive it at boot. SSR stays on GPIO9, active HIGH, as VALVE_CMD was. DIM and
ZC replace the I2C link to the RobotDyn dimmer: the firmware must time the
pump itself (see "Pump firing in firmware") before the board is used.

The XIAO 3V3 output now also supplies the AC-side logic: up to 25 mA with
both channels on, on top of the sensors load.

## Overvoltage protection decision

The board has **no varistor**. RV1 (430 V MOV across Q2, copied from the
M5Stack Unit SSR) was removed on October 10, 2026. The optotriacs stay at
400 V (MOC3043M, MOC3023M).

- Margin: U1 and U2 are rated 400 V. The mains peak is 325 V nominal and
  358 V at +10 %, the grid's sustained limit, which leaves 42 V. Q2 and Q4 are
  rated 600 V, so the optotriacs are the weak point. The BT136S datasheet adds
  that up to 800 V does not damage the triac but may switch it on.
- Sources inside the machine are negligible. The boiler SSR (Keysolu KS53
  D-24Z20N-LQ) switches at zero cross, has a built-in RC snubber, and drives a
  resistive load. Its own datasheet rates it at 600 V transient with no
  varistor, and lists coffee machines among its uses. Our triacs turn the
  valve and pump off at current zero, when the coils store almost no energy.
- What is left is fast, low-energy bursts from appliances on the same
  household circuit, which the snubbers attenuate, and lightning, which
  would take the rest of the machine with it anyway.
- If a spike exceeds 400 V, the most likely result is that the optotriac
  breaks over and runs the load for one half-cycle (10 ms). The current stays
  limited by R5 + R6 or R12. Fast edges below 400 V are not a particular
  weakness: both optotriacs withstand at least 1000 V/µs.
- A varistor across a triac usually fails short after repeated surges, which
  would leave the load permanently on. F1 would not blow, because the load
  limits the current. The current setup already runs the pump on the
  RBDimmer, which has no varistor.
- Fallback if spurious firing shows up, all 600 V and 1000 V/µs, same
  footprint and pinout:
  - U1: MOC3163M (5 mA, inhibit voltage 15 V max). R3 stays at 180 Ω.
  - U2: MOC3053M (6 mA). Its datasheet asks for an LED current of at least
    150 % of the maximum trigger current, 9 mA, so R10 drops to 150 Ω: about
    14 mA typical, at least 9.6 mA worst case. The MOC3052M (10 mA, so 15 mA
    of LED current) would need about 100 Ω.

The snubber capacitors (1 kV X7R MLCC) can also fail short, leaving 39 Ω
across the triac and the load on. A ceramic capacitor does not wear with each
surge the way a varistor does, which is why it is accepted here.

## Isolation and layout

U1, U2 and U3 are the only parts that cross the barrier between 3V3 and mains.
Their datasheets guarantee at least 7 mm of external creepage and clearance;
on the SMD footprints, about 7.5 mm separates the low-voltage pads from the
mains pads.

The board sits in a closed box at the bottom of the machine, in the cold zone
away from steam: pollution degree 2. At 250 V RMS on FR4, IEC 60664-1 then
asks for 2.5 mm of creepage between mains nets and 5 mm (reinforced) between
mains and 3V3. In a steamy spot (pollution degree 3), these would rise to
4 mm and 8 mm.

`combined.kicad_dru` enforces this in the DRC: 1.5 mm minimum for the AC net
class (L_IN, L, N, L_VALVE, L_PUMP, single 1 oz layer for the 2 A
of F1), 2.5 mm between mains nets, and 7 mm between mains and 3V3. Pads of
one part keep their footprint spacing, and a mains track may come to 1.25 mm
of a neighbouring mains pad inside a courtyard, as it enters its own pad.

U1, U2 and U3 use `Combined_Local` copies of the SMDIP footprints with a
1 mm routed slot between the pad rows, extending 1 mm past each end of the
body, as on the RobotDyn dimmer. It cuts the surface creepage path under the
package against dust or residue.

When laying out the PCB:

- Keep at least 7 mm between any 3V3/GND copper and any mains copper, on both
  layers.
- Run N on the bottom layer and L on the top. They may cross through the
  board: L to N is functional insulation, and the FR4 is far beyond 325 V.
  Along the connectors, N runs under their bodies, between the pins and the
  edge, at least 4.45 mm from the pin axis, and reaches each N pin from that
  side; the L traces leave the pins towards the inside of the board.
- The output pin of each triac is trapped between its gate trace and the L
  feed of its optotriac (R5, R12): L_VALVE and L_PUMP leave pin 1 through
  two vias (0.8/0.4 mm, about 1.5 to 2 A each) and reach J3 and J4 on the
  bottom layer. No rotation avoids it, since rotating a part keeps the order
  of its pads.
- D3.1 (+) reaches U3.1 under the D3 body, between its pad columns, which
  avoids crossing D3.2 to U3.2 without a via.
  Keep L_IN, between J2.1 and F1, short; F1 does not protect it.
- Do not pour GND under the mains side.
- Keep the L, L_VALVE, L_PUMP and N traces short and wide enough for F1's 2 A
  rating.
- Give the Q2 and Q4 DPAK tabs (MT2 = L) some copper. This copper is at mains
  potential.

## Open points

- Enclosure and board outline: the AC side alone did not fit the AC box.
  The combined board needs its outline, and the place where it sits in the
  machine, decided before the layout. Its low-voltage side must keep 7 mm
  from every mains net, including the new sensors nets.

- The gate, latching and holding values above come from the Philips BT136S
  datasheet (standard -600 gate), the worst case for gate current. The chosen
  JCST BT136S (LCSC C23427, the part in the M5Stack Unit SSR) has a more
  sensitive gate (10 mA max) and a 20 mA max holding current, but specifies
  neither latching current nor dV/dt.
- D3 is the Hongjiacheng ABS6 (LCSC C48642435) on
  `Diode_SMD:Diode_Bridge_Diotec_ABS` (4 mm pitch, + on pad 1, + and − on the
  same side). Check the bridge polarity in JLCPCB's assembly preview.

## Assembly and maintenance

Update the PCB from the schematic (replaces the 5-pin J1 with the 5 V
header, adds the sensors footprints),
fill the remaining BOM MPN and LCSC columns, confirm JLCPCB stock for the
optotriacs and the EL817C.
The manufacturing export (`uv run ../generate_manufacturing.py .`) requires
the LCSC column and a placed, DRC-clean PCB.

References, in `../../docs/datasheets/`:
- `m5stack-unit-ssr.md` (valve channel origin), `KS53_EN.pdf` (boiler SSR).
- `MOC3043M-D.PDF`, `MOC3023M-D.PDF`, `MOC3163M-D.PDF`, `MOC3052M-D.PDF`
  (onsemi optotriacs; the last covers the MOC3053M).
- `bt136.pdf` (JCST BT136S, LCSC C23427, the M5Stack Unit SSR part),
  `el817.pdf` (Everlight), `abs6.pdf` (Hongjiacheng ABS6, LCSC C48642435),
  `mf2410.pdf` (AEM MF2410 fuses, F1).

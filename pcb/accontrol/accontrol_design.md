# CoffeeFlow AC control board

The connected schematic is `accontrol.kicad_sch`; `accontrol.pdf` is a
generated review copy. `accontrol.kicad_pcb` has the footprints imported from
the schematic, not yet placed or routed. `accontrol_bom.csv` lists values,
footprints and design notes. D3, Q2 and Q4 have their JLCPCB parts; the other
MPN and LCSC fields are still to fill.

The board switches the two original 230 V actuators from 3.3 V logic: the
solenoid valve (OLAB coil, 15 VA, about 65 mA) and the vibration pump (OLAB
Silent Green 35 W, about 0.15 A RMS). It replaces the M5Stack Unit SSR on the
valve and the RBDimmer 4 A on the pump. Both channels reuse those modules'
circuits. The pump channel follows the RobotDyn dimmer. Unlike the RBDimmer in
DimmerLink mode, this board has no microcontroller: the controller receives
the zero-cross signal and times the pump firing itself.

## Connectors

| Connector | Type | Pin 1 | Pin 2 | Pin 3 | Pin 4 | Pin 5 |
|---|---|---|---|---|---|---|
| J1, control | JST XH 5 | GND | 3V3 | ZC | DIM | SSR |
| J2, mains in | JST VH 2 | L | N | — | — | — |
| J3, valve | JST VH 2 | L switched | N | — | — | — |
| J4, pump | JST VH 2 | L switched | N | — | — | — |

Only L is switched. N goes straight from J2 to J3 and J4 pin 2. On the board,
the zero-cross bridge is the only circuit connected to N.

F1 (2 A fast, 250 V) is in series with L right after J2 and protects the whole
mains side. The BOM footprint is 2512, while the planned fuse is a 2410 part.
Check the footprint against the chosen fuse before layout.

## Low-voltage side: SSR and DIM inputs

Both channels share the same input stage. The valve channel uses Q1, R1–R4, D1
and U1; the pump channel uses Q3, R8–R11, D2 and U2.

- R2 / R9 (4.7 kΩ) pull the input to GND, so the channel stays off when J1 is
  unplugged or the controller pin floats during boot.
- R1 / R8 (1 kΩ) limit the base current of Q1 / Q3: about 2.5 mA from a
  3.3 V GPIO. The transistor turns on above about 0.85 V on the input.
- Q1 / Q3 (SS8050) are low-side switches. They sink the optotriac LED current
  and the indicator LED current. That current comes from the 3V3 pin of J1,
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
pull-up. That is 25 mA with both channels on. C1 (1 µF) decouples 3V3 at J1.

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
  channel.

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
  single 1206 resistor could not take (0.22 W, 325 V). D3 is rated 600 V
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

The controller replaces the DimmerLink timing:

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
5. If ZC pulses stop (mains missing or J1 unplugged), hold DIM low.

SSR needs no timing: U1 does its own zero-cross synchronisation.

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
mains pads. When laying out the PCB:

- Keep at least 7 mm between any 3V3/GND copper and any mains copper, on both
  layers. Add a routed slot under the optocouplers if the layout brings the
  two sides closer elsewhere.
- Do not pour GND under the mains side.
- Keep the L, L_VALVE, L_PUMP and N traces short and wide enough for F1's 2 A
  rating.
- Give the Q2 and Q4 DPAK tabs (MT2 = L) some copper. This copper is at mains
  potential.

## Open points

- The gate, latching and holding values above come from the Philips BT136S
  datasheet (standard -600 gate), the worst case for gate current. The chosen
  JCST BT136S (LCSC C23427, the part in the M5Stack Unit SSR) has a more
  sensitive gate (10 mA max) and a 20 mA max holding current, but specifies
  neither latching current nor dV/dt.
- D3 is the Hongjiacheng ABS6 (LCSC C48642435) on
  `Diode_SMD:Diode_Bridge_Diotec_ABS` (4 mm pitch, + on pad 1, + and − on the
  same side). Check the bridge polarity in JLCPCB's assembly preview.
- F1 footprint, see Connectors.

## Assembly and maintenance

Fill the remaining BOM MPN and LCSC columns, confirm JLCPCB stock for the
optotriacs and the EL817C, and settle the F1 footprint before layout.
The manufacturing export (`uv run ../generate_manufacturing.py .`) requires
the LCSC column and a placed, DRC-clean PCB.

References, in `../../docs/datasheets/`:
- `m5stack-unit-ssr.md` (valve channel origin), `KS53_EN.pdf` (boiler SSR).
- `MOC3043M-D.PDF`, `MOC3023M-D.PDF`, `MOC3163M-D.PDF`, `MOC3052M-D.PDF`
  (onsemi optotriacs; the last covers the MOC3053M).
- `bt136.pdf` (JCST BT136S, LCSC C23427, the M5Stack Unit SSR part),
  `el817.pdf` (Everlight), `abs6.pdf` (Hongjiacheng ABS6, LCSC C48642435).

# CoffeeFlow combined board

**Combined on October 13, 2026:** the former `accontrol` project now also
carries the sensors controller, so one board holds both the 230 V side
(valve and pump) and the low-voltage side (XIAO ESP32-S3, CAN, sensors,
boiler SSR output). The AC board alone had been shelved on October 11: it did
not fit the AC enclosure (`print/parts/boitier_ac.py`, 68 × 31.6 × 25.4 mm
inside), and a single cable to the sensors board did not justify it. Merging
both boards removes that cable altogether; the enclosure remains to be
decided (see Open points).

**Version 2, October 26, 2026:** the pump becomes the MPB003 gear pump, a
24 V BLDC pump with its own electronics (`../../docs/pompe-engrenage.md`,
"Interface électrique"). The board keeps the 230 V valve channel and drops
the pump dimmer and the zero-cross detector. It takes 24 V from the pump
supply, makes its own 5 V with a buck, sends 5 V to the screen, and drives
the pump through two optocouplers. The vibration pump setup stays on the
sensors board with the RBDimmer and the M5Stack Unit SSR.

**Version 3, November 15, 2026:** two changes for the next board. A
CA-IS3721CLS digital isolator (U8) replaces the two EL817C: the setpoint
becomes duty × yellow voltage, linear, and the tachometer edges become clean
(see "Gear pump interface"). J13 extends the I2C bus, through two BSS138
level shifters (Q7, Q8), to a NAU7802 on the scale board, `../scale/` (see
"Scale bus"). The
board in service is still v2; "Board in service (v2)" keeps what the
firmware needs for it.

The schematic has three sheets. `combined.kicad_sch` holds the valve
channel and the pump interface, as below. `combined_controller.kicad_sch`
(24 V input and 5 V buck, XIAO, status LED, pressure ADC) and
`combined_interfaces.kicad_sch` (CAN, I2C pull-ups and TMP102, flowmeter,
boiler SSR output) come from `../sensors/`, except the 24 V input and the
buck. The design notes in `sensors_design.md` still apply to those circuits,
with the references renumbered as listed in "Low-voltage side from
sensors". `combined.pdf` is a
generated review copy. `combined.kicad_pcb` is the v3 layout.
`Combined_Local.kicad_sym` holds the U8 symbol: KiCad's ISO7721 symbol does
not fit, its channels run the other way.
`combined_bom.csv` lists values, footprints and design notes.
Every fitted part has its JLCPCB part.

The board switches the 230 V solenoid valve (OLAB coil, 15 VA, about 65 mA)
from 3.3 V logic, with the circuit of the M5Stack Unit SSR it replaces.

## Connectors

| Connector | Type | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|---|
| J1, screen | JST XH 4 | GND | +5V | CAN_L | CAN_H |
| J2, mains in | HT396R 2 | L | N | — | — |
| J3, valve | HT396R 2 | N | L switched | — | — |
| J6, flow | HT396V 3 | GND | +5V | FLOW_PULSE | — |
| J7, 24 V in | HT396V 2 | +24V | GND | — | — |
| J8, boiler SSR | HT396V 2 | GND | BOILER_5V | — | — |
| J9, pump | JST XH 4 | GND_P (green) | SPEED_P (brown) | TACHO_P (blue) | +5V_P (yellow) |
| J10, pressure | JST XH 3 | +3V3 | GND | PRESSURE | — |
| J13, scale | JST XH 4 | SCALE_SDA | SCALE_SCL | +5V | GND |

J1 was the 5 V input in v1; it now carries the screen cable, 5 V and CAN
in one XH 4 housing, the same part as J9. J7 takes two 0.75 mm² wires from the 24 V
supply. In J7's plug, each one shares a double 2 × 0.75 mm² ferrule with the
pump's red or black wire: the pump is fed from J7, and its current passes
from wire to wire in the ferrule without crossing the board's tracks. J9 carries only the pump's isolated signal wires;
the white wire (rotation) stays unconnected in the cable.

The former 5-pin control header (GND, 3V3, DIM, ZC, SSR) is gone: SSR,
PUMP_SPEED and PUMP_TACHO run as tracks to the XIAO sockets, and 3V3 comes
from the XIAO. The sensors board's two Grove I2C headers and its VALVE
header are not carried over either: the valve SSR is on board.

J7 (24 V in) and J8 (boiler SSR) are Kefa HT396V-3.96-2P vertical headers
(LCSC C577788). Both mate with the HT396K-3.96-2P plug (LCSC C577777, screw
terminals, 0.2 to 1.5 mm², AWG 14 to 26). J6 (flow) is the 3-pin version,
HT396V-3.96-3P (LCSC C577789), with the HT396K-3.96-3P plug (LCSC C577778):
the Digmesa cable ends in a VH3.96 housing at the sensor and needs 0.5 mm²
wires, which go straight into the screw plug, without the JST-SM to XH
adapter. J7's plug takes the supply and pump wires
in two double ferrules. J8's takes the two 0.75 mm² wires to the boiler SSR
directly; the SSR end uses red 4.8 mm crimp lugs, which need that section
and do not fit a JST XH crimp. On sensors, two Wago changed the section.
The plugs are not assembled: order them with the board. The LCSC footprint
matches the Kefa drawing (`../../docs/datasheets/C577788.pdf`, 2 to 24
poles): body 7.92 × 7.40 mm centred on the pins (11.88 × 7.40 mm in 3
poles), 10.1 mm high; the courtyard only gained
a 0.25 mm margin. Leave room beside each header for the plug's wire entry
and for a screwdriver above.

J2 and J3 are Maixu MXHT396R-3.96-02P-OE01-Cu-A right-angle headers (LCSC
C48688287, orange; C48688320 is the green one), a JST VH clone. They replace
the Kefa HT396R-3.96-2P (C577799), out of stock at JLCPCB: same pitch, pins
and IEC ratings. Their footprint and 3D model come from LCSC for the Kefa
part, in `Combined_Local`; the 3D model is still the Kefa one. The outline
follows the Maixu drawing (`../../docs/datasheets/C48688287.pdf`): the mating
face is 9.7 mm in front of the pin axis (9.6 mm on the Kefa) and the body
0.5 mm behind it. The silkscreen stops 0.4 mm short of the mating face; the
courtyard covers the pads, the body and the slot, which reaches 2.4 mm behind
the pin axis. Place the connectors with the pins inside the board and the
mating face at the edge: pin axis 9.7 mm from the edge. The drawing asks for
1.7 mm holes; the footprint keeps 1.6 mm, which still takes the 1.41 mm
diagonal of the 1 mm square pin and keeps 0.2 mm of copper around the hole.
The plug stays the Kefa HT396K-3.96-2P; that it latches on the Maixu header
is to be checked with the parts in hand. The pads, holes
and the slot between the pins are set for isolation; see "Isolation and
layout".

Only L is switched. N goes straight from J2 pin 2 to J3 pin 1; nothing else
on the board connects to N. J3 is reversed against J2 so that, side by side,
the two N pins face each other. The valve cable follows: N on pin 1.

F1 (0.5 A fast, 250 V) is in series with L right after J2 (net L_IN between
them) and protects the whole mains side, which is now the valve alone. It is
an AEM MF2410F0.500TM (LCSC C91409), the smallest rating of the series,
assembled by JLCPCB on the LCSC `F2410` footprint in
`Combined_Local`, whose courtyard was enlarged to cover the pads. The pads
leave 3 mm between L_IN and L.

- Load: the valve holds at 15 VA, 65 mA, 13 % of the rating. Olab gives no
  inrush for the coil; small AC valves draw 2 to 3 times the holding current
  while the plunger is open. At 4 times, 0.26 A for up to 3 cycles, the
  inrush is about 0.004 A²s, 28 times under the fuse's 0.11 A²s melting
  I²t. Q2 fires at the zero crossing and adds no peak.
- Fast-acting: it clears 5 A (10 In) in 1 to 10 ms, and a coil with shorted
  turns drawing 1 A (200 %) in under 120 s.
- Breaking capacity: 100 A at 250 VAC. A short after F1 on a household
  outlet can exceed that (230 V / 0.5 Ω loop ≈ 460 A); the circuit breaker
  then clears it (a 10 A type B breaker trips magnetically from 50 A).
- Its melting I²t (0.11 A²s) is 28 times under the BT136S surge rating
  (25 A for 20 ms, about 3.1 A²s): F1 saves Q2 from a shorted valve or
  wiring, which the 2 A fuse of v1 (3 A²s) did not.
- Soldered, not replaceable. If it blows, something else has already failed.

## Low-voltage side: SSR input

The valve channel uses Q1, R1–R4, D1 and U1.

- R2 (4.7 kΩ) pulls the input to GND, so the channel stays off while the
  XIAO is unplugged or its pin floats during boot. It replaces the 10 kΩ
  VALVE pull-down of the sensors board, which is not carried over.
- R1 (1 kΩ) limits the base current of Q1: about 2.5 mA from a 3.3 V GPIO.
  The transistor turns on above about 0.85 V on the input.
- Q1 (SS8050) is a low-side switch. It sinks the optotriac LED current and
  the indicator LED current. That current comes from the XIAO 3V3 output,
  not from the GPIO, which only supplies the base current. With 2.5 mA of
  base current for about 15 mA of collector current, the transistor
  saturates.
- R3 (150 Ω) sets the optotriac LED current: (3.3 − 1.2 − 0.1) / 150 ≈ 13 mA
  typical, at least 9.6 mA worst case (LED forward voltage 1.5 V max). U1 is
  guaranteed to trigger at 5 mA; onsemi recommends any current between that
  and the 60 mA maximum.
- D1 (red) with R4 (1 kΩ) lights at about 1.2 mA when the channel is
  commanded. It sits in parallel with the optotriac LED, so it shows the
  command, not whether the valve is actually conducting.

Load on the 3V3 rail: about 12 mA with the valve on, plus U8's side A
(1.8 to 3.1 mA typical, 4.9 mA max), drawn from the XIAO 3V3.

## Mains side: valve channel (zero-cross)

U1 (MOC3043M) is a zero-cross optotriac. It does not carry the valve current.
Its output triac only drives a pulse into the gate of Q2 (BT136S), and
Q2 connects L to L_VALVE.

- Gate path: L → R5 (300 Ω) → U1 pins 6–4 → R6 (300 Ω) → Q2 gate →
  L_VALVE. U1 only turns on while the voltage across it is below its inhibit
  voltage (20 V max). Once on, it stays on above that voltage as long as its
  current exceeds its holding current (400 µA typical). The gate current then
  grows with the line voltage until Q2 triggers. With the standard BT136S gate
  (35 mA max in the two quadrants used here), that happens below about 25 V,
  0.25 ms after the voltage zero. U1 then carries at most about 35 mA, well
  under its 100 mA repetitive peak rating. Once Q2 conducts, only about 1.5 V
  remains across it, and the U1 current drops to zero.
- R5 + R6 = 600 Ω limit the U1 current if it ever fires at a high
  instantaneous voltage: 325 V / 600 Ω ≈ 0.54 A, below its 1 A non-repetitive
  surge rating. The M5Stack design uses 360 Ω + 330 Ω; two 300 Ω 1206 are
  JLCPCB basic parts.
- While SSR is high, U1 fires Q2 again at the start of every half-cycle. When
  SSR goes low, Q2 turns off at the next current zero. Because the coil is
  inductive, the current zero comes after the voltage zero. Q2 actually drops
  out when the current falls below its holding current (15 mA max), about
  0.5 ms before the current zero at the valve's 92 mA peak.
- R7 (100 Ω) + C2 (10 nF, 1 kV X7R 1812, PSA FV43X103K102EFG) form a
  snubber across Q2. At turn-off on an inductive load, the voltage across the
  triac jumps quickly. The snubber limits that dV/dt so Q2 does not
  re-trigger on its own. At 50 Hz, C2 lets about 0.7 mA through the valve
  while it is off (230 V / 318 kΩ). A solenoid ignores this.
- R7 limits the discharge of C2 into Q2 when it fires: about 0.25 A at the
  usual 25 V, 3.3 A if Q2 ever fires at the 325 V peak. It hardly changes
  the turn-off dV/dt: the voltage across Q2 steps by 15 mA × 100 Ω = 1.5 V,
  then rises at 15 mA / 10 nF ≈ 1.5 V/µs whatever R7 is. The onsemi
  application value is 39 Ω; 100 Ω is a JLCPCB basic part in the same 1206.

## Triac dissipation

The BT136S on-state model is 1.27 V + 0.091 Ω, and the junction-to-ambient
resistance is 75 K/W on the datasheet's FR4 footprint. The valve costs Q2
under 0.1 W. It needs no heat sink beyond the DPAK tab copper.

## 24 V input and 5 V buck

J7 brings 24 V from the pump supply. The buck is not isolated: board GND is
the supply's negative, the pump's black wire.

U10 is a TI LMR33630ADDAR (LCSC C841384): synchronous buck, 3.8 to 36 V in,
3 A, 400 kHz, HSOP-8 with a thermal pad (pin 9, GND).

- Output: 1.0 V reference (0.985 to 1.015 V) and R28 / R29 (120 kΩ /
  30 kΩ): 1.0 × (1 + 120 / 30) = 5.00 V, 4.93 to 5.08 V from the
  reference tolerance.
- Enable as undervoltage lockout: R30 / R31 (330 kΩ / 24 kΩ) from VIN.
  EN starts the switching at 1.231 V (1.2 to 1.26 V) and stops it 100 mV
  lower: start at 18.2 V (17.7 to 18.6 V), stop at 16.7 V. The board waits
  for the supply to be close to 24 V, and stops before the pump's own
  undervoltage trip at 15.5 V. When the XIAO goes down, R34 holds
  PUMP_SPEED low and the pump setpoint drops to 0 (see "Gear pump
  interface").
- L1: 10 µH, FXL0630-100-M (LCSC C167223), 4.5 A saturation, 68 mΩ. Ripple
  current (24 − 5) × 5 / (24 × 10 µH × 400 kHz) ≈ 1.0 A peak to peak, a
  third of the 3 A rating, as TI recommends.
- Output: C17–C19, 3 × 22 µF 1206. The plan asked for 16 V parts; the JLCPCB
  basic part is 25 V X5R (Samsung CL31A226KAHNNNE, LCSC C12891), which keeps
  more of its capacitance at 5 V. With about 40 µF left at 5 V, the ripple
  is 1.0 / (8 × 400 kHz × 40 µF) ≈ 8 mV.
- Input: C1, 10 µF 50 V X5R 1206 (Samsung CL31A106KBHNNNE, LCSC C13585),
  and C5, 220 nF 50 V X7R 0805 (Samsung CL21B224KBFNNNE, LCSC C5378) next
  to the VIN and PGND pins. TI's reference input for the HSOP package at
  400 kHz is the same values in X7R 1210 and 0603, sized for 3 A out. Those
  are JLCPCB extended parts (the basic 220 nF 0603 is 25 V); the X5R 1206
  keeps less of its capacitance at 24 V, which the board accepts because it
  draws about 1 A at most.
- No electrolytic on the input. TI adds one to damp the ringing of long
  supply leads with the ceramics, which matters when J7 is plugged in live:
  the overshoot could reach about 48 V, above U10's 36 V. J7 is never
  plugged with the 24 V on; the board is only powered from the mains side,
  where the supply ramps up slowly.
- C7 (100 nF) from BOOT to SW, C16 (1 µF) on VCC, the internal 5 V LDO.
- D5 (red 0603) with R32 (1 kΩ) shows +5V: about 3 mA.
- PG is not used.

Load: the XIAO, CAN, flow sensor, boiler SSR output and the screen. 5 W
at 5 V is 0.23 A from 24 V at 90 % efficiency; the supply budget counts
0.25 A for the buck. With 2 A at 5 V for margin, U10 runs well inside its
3 A.

Layout:

- C5, then C1, right at VIN and PGND: the input loop carries the switched
  current and must stay small.
- Keep the SW node, C7 and L1 compact; nothing sensitive under L1.
- Thermal pad soldered to a GND area with the footprint's vias to a bottom
  GND plane.
- R28 / R29 next to FB, away from SW; feedback taken at the output
  capacitors.

The SW node is an 8.3 mm, 0.8 mm wide track from U10 pin 8 to L1 pin 1,
longer than in TI's figure 10-2: C7 (BOOT) and C16 (VCC) sit between U10
and L1 and push L1 away. TI's layout guidelines (section 10.1) rank CIN,
then CVCC, CBOOT and the FB divider at their pins before the SW length
(point 9). L1 stays where it is (decided on November 15, 2026). A shorter SW
node would not change regulation or efficiency (about 10 nH in series with
10 µH), only radiation and ringing on the switching edges, over the bottom
GND plane. Nothing sensitive is within 3 mm of it: the CAN transceiver is
about 22 mm away and the ADS1115 about 45 mm. The input loop (C5 at 2.3 mm
from VIN and GND) and the thermal vias follow TI.

## XIAO 5 V pin and USB

On the XIAO ESP32-S3, the 5V pin is the USB-C VBUS, with no diode between
them (Seeed schematic v1.2, `../../docs/datasheets/xiao_esp32s3_sch.pdf`).
The board feeds that pin straight from +5V.

**Never plug USB into the XIAO while it sits on this board.** Flash it off
the board, or through coffeetool once mounted.

- USB without 24 V: the USB 5 V drives +5V, then through L1 and the body
  diode of U10's high-side MOSFET the 24 V rail. It charges the supply's
  2 × 470 µF and the pump electronics from the PC port, whose
  overcurrent protection is the only limit.
- USB with 24 V: two 5 V sources in parallel. Whichever is higher feeds the
  other; the PC may end up supplying the screen and the whole board.

A Schottky (B5819W) from +5V to the XIAO 5V pin would remove both cases for
0.3 V of drop. It was left out on October 26, 2026: USB is never used with
the XIAO on the board.

## Gear pump interface

U8, a Chipanalog CA-IS3721CLS (LCSC C41784135,
`../../docs/datasheets/CA-IS3721CLS.pdf`, version 1.03), keeps the pump's signal wires (green
GND_P, yellow +5V_P, brown SPEED_P, blue TACHO_P) on its side B: they never
touch a ground or a supply of the board. The pump datasheet does not say
whether green is tied to black inside; on the received pump, the meter finds
no continuity between them, which does not rule out a path through the
electronics. The SOVOFLO family forbids tying the setpoint reference to the
power ground. The circuit works either way.

The CA-IS3721C is a two-channel capacitive isolator with one channel each
way, 3.0 to 5.5 V on each side, Schmitt-trigger inputs and push-pull
outputs. Each channel sends a carrier across the barrier for one input state
and nothing for the other (on-off keying); the receiver rebuilds the level.
The "L" parts output LOW when their input is open or the input side is
unpowered (`tDO` 15 ns max). The output is undefined while its own side is
below its 2.7 V undervoltage threshold. It needs 25 to 37 µs to start.

**Pinout, not the ISO7721's.** Side A: 1 VDDA, 2 VI1, 3 VO2, 4 GNDA. Side B:
5 GNDB, 6 VI2, 7 VO1, 8 VDDB. Channel 1 runs from pin 2 to pin 7, channel 2
from pin 6 to pin 3. TI's ISO7721 has the same supply pins but its channels
run the other way (input on pin 7, output on pin 2): the two are not
interchangeable, and KiCad's `Isolator:ISO7721D` symbol does not fit. The
CA-IS3722C has the ISO7721 order. The default-high CA-IS3721CHS (C41409432)
fits the same pads but would start the pump without a XIAO: never substitute
it.

Side A is on the XIAO 3V3 with C22 (100 nF) and C23 (1 µF), as the
datasheet asks; side B runs from the yellow wire with C24 (100 nF) and C25
(1 µF).

Setpoint (channel 1):

- PUMP_SPEED (GPIO1) drives VI1 directly. R34 (100 kΩ) holds GPIO1 low while
  the XIAO is absent or booting; VI1 left open gives LOW as well.
  ESP32-S3 high output ≥ 2.64 V against VIH = 0.7 × 3.3 = 2.31 V.
- VO1 swings from GND_P to +5V_P through about 50 Ω (± 40 %). Filter: R36 /
  C20 and R37 / C21, 2 × 2.2 kΩ / 2.2 µF (33 Hz each, nearer 40 Hz with the
  X5R's loss at 5 V), between VO1 and the brown wire, capacitors to green.
  C20 and C21 are 50 V X5R 0805 (Samsung CL21A225KBQNNNE, LCSC C377773),
  kept from v2. Charge and discharge both go through R36, so
  **SPEED_P = duty × V(yellow) × 176 / (176 + 4.4) ≈ 0.976 × duty ×
  V(yellow)**: 176 kΩ is the pump input, 4.4 kΩ the filter. The 2.4 %
  is proportional; full scale is about 4.86 V with the yellow wire at
  4.98 V.
- What is left of non-linearity is the difference between the high and low
  output resistances, 40 Ω at worst against 2.2 kΩ: at most 0.45 % of
  V(yellow) at 50 % duty, about 23 mV, much less with matched outputs. Pulse
  width distortion (10 ns max) is 10 ppm of the 1 ms period. With the LEDC at
  1 kHz and 14 bits, one step is 61 ns, three times the 20 ns minimum pulse,
  and 0.3 mV of setpoint. The ripple stays under 10 mV, about 10 rpm, and
  the response takes about 15 ms.
- 2.2 kΩ is a compromise. 1.5 kΩ (v2) would draw up to 3.3 mA peaks from
  VO1, close to its 4 mA rating, and double the non-linearity; 4.7 kΩ would
  lose 5 % of the full scale, 4.73 V, too close to the 4.6 V of the purge.
- 3V3 absent, XIAO absent or booting, firmware stopped: VO1 is LOW and the
  pump stays at 0.

Why it matters: a MPB015 (0.36 ml/turn, `docs/pompe-engrenage.md`,
"Alternative : MPB015") would run between about 0.6 and 2 V, where 0.1 V is
about 0.6 ml/s. On v2 that range is 0.5 to 5 % duty, pulses of 5 to 50 µs
that the EL817C stretches by 24 to 31 µs, and the setpoint depends mostly on
the optocoupler's turn-off time, which varies from part to part and with
temperature. With U8 it depends on the duty and the yellow voltage only.

Tachometer (channel 2):

- Blue drives VI2; R38 (10 kΩ) pulls it up to the yellow wire, so an
  open-collector output would work too (0.5 mA while blue is low). Measured
  on the received pump: active 0–5 V output, 2 pulses per turn, 167 Hz at
  5 000 rpm.
- VI2 thresholds from the yellow 4.98 V: high above 3.49 V, low below
  1.49 V, with hysteresis.
- VO2 drives PUMP_TACHO (GPIO3) push-pull at 3.3 V, rise and fall under
  5 ns: **PUMP_TACHO follows blue, it is no longer inverted.** No pull-up.
  Pump unpowered or J9 unplugged: PUMP_TACHO stays LOW.
- GPIO3 is a JTAG strapping pin, read only when the `STRAP_JTAG_SEL` eFuse
  is burnt, which it is not by default.

Yellow wire load: U8 side B draws 1.9 mA typical with both inputs low,
3.4 mA with both high, 5.0 mA at most; add up to 0.5 mA for R38 and about
0.6 mA for the filter at 50 % duty. That is 2.5 to 4.3 mA typical and about
6 mA at worst. Measured on the received pump: 4.98 V open, unchanged at
2.3 mA and 5 mA. The pump pulls the brown wire to green through 176 kΩ:
left open, the setpoint reads 7 mV and the pump stays stopped, so a loose
J9 cannot start it.

A strongly driven input can feed an unpowered side through its protection
diodes and make the outputs undefined. Blue and yellow come from the same
pump electronics, so this cannot happen in service; do not plug J9 with the
pump powered, its contacts do not mate in order.

Firmware for v3:

- Setpoint: duty = V / (0.976 × V(yellow)), no table. Measure one point at
  the bench to fix the 0.976.
- Tachometer: the edge to count is the same as on v2. On v2, PUMP_TACHO
  falls when blue falls (U9 conducts); on v3 it falls with blue too. Only
  the idle level without the pump changes, LOW instead of HIGH. The software
  filter (35 % of a turn) can stay, it should no longer reject anything.
- The firmware needs to know which board it runs on (see Open points).

On the board, U8 straddles the boundary: pins 1–4 toward the XIAO, pins 5–8
toward J9. Group J9, R36–R38, C20, C21, C24 and C25 on the pump side, and
keep the board GND pour away from GND_P and +5V_P so that they are not tied
by accident. The isolation is functional only and the default rules apply.
U8 uses KiCad's `SOIC-8_3.9x4.9mm_P1.27mm`, as U4. Chipanalog's land
pattern (version 1.03) has 0.60 × 1.40 mm pads at 5.50 mm; KiCad's are
0.6 × 1.95 mm at 4.95 mm, with the same outer edge. Between the pad rows
that leaves 3.0 mm instead of 4.1 mm, enough for functional isolation.

### Board in service (v2)

Two EL817C on `Package_DIP:SMDIP-4_W9.53mm`. U8: R33 (330 Ω) from GPIO1 to
its LED, about 6 mA; collector on yellow, emitter pulled to green by R35
(10 kΩ), then 2 × 1.5 kΩ / 2.2 µF. U9: LED from yellow to blue through R38
(1.8 kΩ, 2 mA); transistor pulls PUMP_TACHO to GND against R39 (4.7 kΩ to
3V3): the signal is inverted.

**The v2 setpoint is not linear in the duty cycle.** Measured on the board
on November 11, 2026, J9 yellow on the board +5V (5.16 V), SPEED_P with no
pump, PWM at 1 kHz:

| Duty | 0 % | 0.1 % | 0.3 % | 0.5 % | 1 % | 2 % | 3.09 % | 7.79 % | 15.77 % | 32.37 % | 50 % | 87.87 % | 100 % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| SPEED_P (V) | 0 | 0.071 | 0.318 | 0.576 | 1.045 | 1.481 | 1.727 | 2.455 | 3.240 | 4.09 | 4.55 | 5.02 | 5.09 |

C20 charges through R36 alone (1.5 kΩ) while U8 conducts, but discharges
through R36 + R35 (11.5 kΩ) while it is off: for a duty D, V = Vmax × k·D /
(1 + (k − 1)·D) with k ≈ 7.7. U8 also stays on after its LED goes off: about
1 µs for a 1 µs pulse, 11 µs at 5 µs, then 24 to 31 µs from 20 µs on. The
curve is monotonic, and 0 % gives exactly 0 V. The firmware maps a setpoint
to a duty by linear interpolation in this table, as a fraction of Vmax, and
reads the speed back on the tachometer. The fall time is about 25 ms.

On v2, U9's transistor turns off slowly when blue rises, and R39 pulls GPIO3
up over tens of µs. In the first purge in the machine (2026-11-13, pump at
4 605 rpm), noise during that ramp gave about 4.6 extra falling edges per
second, 0 to 60 µs after the rising edge, about 3 % of the count. The
firmware rejects them since 0.5.1: an edge under 35 % of a turn after the
previous one is dropped.

## Scale bus

The scale board (`../scale/`, a NAU7802 at the load cell under the drip
tray) sits on this board's I2C bus. The only way through is a 4 mm hole from
the inside of the machine to the purge area of the drip tray, so the cable
carries four wires: GND, +5V, SDA and SCL, 20 to 25 cm, not twisted. The
scale board has no microcontroller and no firmware; the XIAO here reads the
weight. Datasheet: `../../docs/datasheets/nau7802.pdf` (Nuvoton, rev 2.6).

Why a NAU7802 on I2C. The first design put an ADS1232 and a XIAO on the
scale board and a second TJA1051 here, with the CAN logic lines on the
cable. The NAU7802 drops the third firmware and the module behind the drip
tray, and needs no GPIO here (only GPIO43 is free). It is noisier than the
ADS1232, 50 nV rms at 10 SPS and 150 nV at 80 SPS (gain 128) against 17 and
44 nV, which stays far below 0.1 g: with a 1 mV/V cell excited at 4.2 V, a
1 kg cell gives 4.2 mV full scale, so 0.012 g rms at 10 SPS and 0.036 g at
80 SPS. The cost is an I2C bus that leaves the board.

Why the scale side runs at 5 V:

- The NAU7802's I2C thresholds follow DVDD (VIH = 0.7 × DVDD), and AVDD
  cannot exceed DVDD; its LDO needs DVDD at least 0.3 V above its output.
- At gain 64 and 128, the PGA inputs must stay between AVSS + 1.5 V and
  AVDD − 1.5 V, a figure given only at 5 V. The bridge midpoint is half the
  excitation: at 3.0 V (DVDD on 3.3 V), the window shrinks to nothing.
- So DVDD is on +5V and the LDO gives 4.2 V to AVDD, REFP and the bridge:
  midpoint 2.1 V in a 1.5 to 2.7 V window, and 0.8 V of headroom for the
  LDO. That makes VIH 3.5 V, out of reach of the 3.3 V bus: hence Q7 and
  Q8.

Level shifters:

- Q7 (SDA) and Q8 (SCL) are BSS138-7-F (LCSC C40912), gate on +3V3, source
  on I2C_SDA / I2C_SCL with R20 / R23 (4.7 kΩ), drain on SCALE_SDA /
  SCALE_SCL with R40 / R41 (2.2 kΩ) to +5V. A side pulled low pulls the
  other low, first through the body diode, then through the channel once
  Vgs rises; released, each side rises on its own pull-ups. The 2N7002DW
  dual was left out: its threshold reaches 2.5 V against 3.3 V of gate.
- The NAU7802 asks rise and fall times of 300 ns at most in every mode, and
  400 pF at most on the bus. With about 80 pF of cable and pins, 2.2 kΩ
  gives about 150 ns; the 4.7 kΩ of the 3.3 V side alone would give about
  320 ns. The bus stays at 100 kHz.
- The 5 V side gives 1.5 V of noise margin (VIH 3.5 V of 5 V), against
  0.8 V for an ESP32-S3 input on the 3.3 V bus.
- With the scale unplugged, R40 / R41 hold SCALE_SDA / SCALE_SCL high and
  the bus on this board works as before. A fault on the cable that holds a
  line low blocks the whole bus, ADS1115 and TMP102 included: the firmware
  recovers it (nine SCL pulses, then a STOP) and reports the scale as
  missing.

J13 (JST XH 4, B4B-XH-A, as J1): 1 SCALE_SDA, 2 SCALE_SCL, 3 +5V, 4 GND,
pin for pin with J1 of the scale board, a screw terminal block: straight
cable. The XH end
is crimped on this side first; the wires then go through the hole and into
the terminal block. The scale board draws about 7.7 mA from +5V (NAU7802
2.1 mA, the 1 kΩ cell (1 000 Ω measured between red and black, and between green and white) 4.2 mA at 4.2 V, its LED 1.4 mA), plus R40 / R41 while
a line is low (2.3 mA each). The NAU7802's LDO gives 10 mA at most: at
4.2 V the cell must be above 420 Ω (a 350 Ω cell would draw 12 mA).

Firmware, from the NAU7802 datasheet:

- Address 0x2A, fixed; no conflict with the TMP102 (0x48) or the ADS1115
  (0x49).
- Nothing runs until configured: RR = 1 then 0 with PUD = 1, wait for PUR
  (about 200 µs), LDO at 4.2 V (VLDO = 001) with AVDDS = 1, PUA = 1, gain
  128, rate, REG0x15 = 0x30, PGA_CAP_EN for the Cfilter on VIN2, then an
  internal offset calibration (CALS, check CAL_ERR), and drop the first six
  conversions.
- Read the result by polling CR (REG0x00 bit 5), then a 3-byte burst read
  from 0x12. Never enable CRSD (REG0x11 bit 7): the chip would then hold SDA
  low to signal a conversion, and block the shared bus. DRDY is not wired.
- Disable the internal 50 kΩ pull-ups (WPD = 1); the 2.2 kΩ here are enough.

On the board, Q7, Q8, R40 and R41 sit next to J13, on the edge facing the
drip tray hole. I2C_SDA and I2C_SCL already run to the TMP102 and the
ADS1115; SCALE_SDA and SCALE_SCL stay short, over the GND plane.

## Low-voltage side from sensors

Everything on the sensors board except its Grove I2C headers (J4, J5) and its
VALVE header (sensors J7, with its R17 pull-down) is copied with unchanged values and
connections. The 4.7 kΩ SDA/SCL pull-ups stay: the TMP102 and the ADS1115
are on the bus. References that clashed with the AC side were renumbered:

| sensors | combined | Part |
|---|---|---|
| U1 | U7 | XIAO ESP32-S3 (schematic only, plugs into J11/J12) |
| D2 | D4 | Status LED, red 0603 here (green 0805 on sensors) |
| J1, J3 | J1 | 5 V and CAN connectors, merged into the screen connector |
| Q2 | Q5 | AO3401A, boiler output |
| Q3 | Q6 | 2N7002, boiler output |
| R11 | R17 | TJA1051 TXD pull-up |
| R12 | R18 | CAN termination, 120 Ω 1206 |
| R13, R14 | R20, R23 | SDA, SCL pull-ups |
| R15 | R24 | FLOW pull-up |

The other references (C8–C15, H1, J6 and J8 (now HT396V), J10–J12,
R19, R21, R22, R25, R26, U4–U6) keep their sensors numbers. The XIAO
footprint and its socket footprint are copied into `Combined_Local`;
`sym-lib-table` points to the local Seeed XIAO symbol library, as in sensors.

XIAO pins used by the valve and the pump:

| Signal | GPIO | Seeed name | U7 / J11–J12 pin | Firmware |
|---|---:|---|---|---|
| PUMP_SPEED | 1 | D0 | U7 / 1, J11 / 1 | LEDC PWM, 1 kHz, 14 bits; 0 at boot |
| PUMP_TACHO | 3 | D2 | U7 / 3, J11 / 3 | GPIO interrupt on falling edges (blue falling, on v2 and v3), software filter: 3 ms, and 35 % of a turn |
| SSR | 9 | D10 | U7 / 11, J12 / 4 (former VALVE_CMD) | active HIGH |

The other pins keep their sensors assignments; only D6/GPIO43 stays free.
PUMP_SPEED and PUMP_TACHO take the former DIM and ZC pins. SSR stays on
GPIO9, active HIGH, as VALVE_CMD was.

The firmware recognises a pump fault (undervoltage, overvoltage,
temperature, locked rotor) when the tachometer stays silent while the
setpoint is above 0.3 V. It then sets the setpoint to 0 for at least 10 ms,
which re-arms the pump, logs the fault and does not restart on its own during
a shot. The pump ignores a setpoint sent within 65 ms of a cold start.

The XIAO 3V3 output also supplies the valve logic (12 mA with the valve on),
U8's side A (4.9 mA at most), on top of the
sensors load.

## Overvoltage protection decision

The board has **no varistor**. RV1 (430 V MOV across Q2, copied from the
M5Stack Unit SSR) was removed on October 10, 2026. The optotriac stays at
400 V (MOC3043M).

- Margin: U1 is rated 400 V. The mains peak is 325 V nominal and 358 V at
  +10 %, the grid's sustained limit, which leaves 42 V. Q2 is rated 600 V, so
  the optotriac is the weak point. The BT136S datasheet adds
  that up to 800 V does not damage the triac but may switch it on.
- Sources inside the machine are negligible. The boiler SSR (Keysolu KS53
  D-24Z20N-LQ) switches at zero cross, has a built-in RC snubber, and drives a
  resistive load. Its own datasheet rates it at 600 V transient with no
  varistor, and lists coffee machines among its uses. Q2 turns the valve
  off at current zero, when the coil stores almost no energy.
- What is left is fast, low-energy bursts from appliances on the same
  household circuit, which the snubber attenuates, and lightning, which
  would take the rest of the machine with it anyway.
- If a spike exceeds 400 V, the most likely result is that the optotriac
  breaks over and opens the valve for one half-cycle (10 ms). The current
  stays limited by R5 + R6. Fast edges below 400 V are not a particular
  weakness: the optotriac withstands at least 1000 V/µs.
- A varistor across a triac usually fails short after repeated surges, which
  would leave the valve permanently open. F1 would not blow, because the
  load limits the current.
- Fallback if spurious firing shows up: MOC3163M (600 V, 1000 V/µs, 5 mA,
  inhibit voltage 15 V max), same footprint and pinout. R3 stays at 150 Ω.

The snubber capacitor (1 kV X7R MLCC) can also fail short, leaving 100 Ω
across the triac and the load on. A ceramic capacitor does not wear with each
surge the way a varistor does, which is why it is accepted here.

## Isolation and layout

U1 is the only part that crosses the barrier between 3V3 and mains. Its
datasheet guarantees at least 7 mm of external creepage and clearance;
on the SMD footprints, about 7.5 mm separates the low-voltage pads from the
mains pads.

The board sits in a closed box at the bottom of the machine, in the cold zone
away from steam: pollution degree 2, overvoltage category II (2.5 kV
impulse), FR4 in material group IIIa. IEC 60335-1 then asks:

- Between mains nets (functional insulation), 2.0 mm of creepage at
  125–250 V (Table 18), 1.1 mm up to 50 V, and 1.5 mm of clearance
  (Table 16). Creepage follows the long-term RMS voltage across it;
  transients are neglected (IEC 60664-1, 5.2.2.2).
- Between mains and 3V3 (reinforced insulation), twice the 2.5 mm of basic
  insulation (Table 17): 5 mm.

In a steamy spot (pollution degree 3), these would rise to 3.2 mm between
mains nets and 8 mm to 3V3.

`combined.kicad_dru` enforces this in the DRC: 1 mm minimum width for the
AC net class (L_IN, L, N, L_VALVE, single 1 oz layer for the 0.5 A of F1), 2.2 mm between mains nets, and 7 mm between mains and 3V3. The
0.2 mm over Table 18 covers etching tolerance. Some mains nets stay within a
few volts of each other and may come closer:

- L side: L, L_IN, and the far ends of R5 and R7 (the gate resistor and the
  snubber resistor). R5 drops at most 11 V for 0.25 ms per half-cycle. 1.5 mm,
  the clearance.
- Q2's gate and R6 with L_VALVE. 1.5 mm.

Pads of one part keep their footprint spacing. A mains track may come to
1.25 mm of a mains pad of the part it enters, inside that part's courtyard,
and two mains tracks ending on the pads of R5 or R6 may come to 1.8 mm, the
pad gap of a 1206; these resistors see little voltage. C2 has the full mains
across it when Q2 is off, so it is an 1812: 2.7 mm between pads.

H2 and H6, M2 mounting holes, sit on the mains side. The board rests on a
printed PETG cylinder holding a brass insert, and a nylon screw clamps it:
nothing conductive passes through the board, and the holes are unplated.
The insert, under the board, touches no copper and floats. The hardware
therefore cannot bridge the mains to the low-voltage side, whatever its
distance to the copper. For reference, from a 2.3 mm radius around the hole,
H2 is 10.0 mm from the nearest mains copper (L_VALVE on the bottom layer);
H6 is 7.2 mm from it (R6 pad 2, Q2 gate) and 2.0 mm from the GND plane.
Metal screws or standoffs would make these gaps the insulation: H6 would
then keep only 0.2 mm over the 7 mm reinforced distance. H1, H3, H4 and H5 are on
the low-voltage side. Under the hardware of H1, H4 and H5 there is only GND
plane, so the plane runs up to the hole clearance, without a keepout.

J2 and J3 (Maixu MXHT396R-3.96) are rated 320 V at
overvoltage category II and pollution degree 2 by their IEC data, with 1 mm
pins in 1.6 mm holes (`../../docs/datasheets/C48688287.pdf`). They have L and N
on adjacent pins. Their 2.0 × 2.8 mm oval pads leave 1.96 mm of air, above
the 1.5 mm clearance, and 0.2 mm of copper around the hole; a 1 mm
milled slot between the pins lengthens the creepage path around it; a groove
counts from 1 mm wide at pollution degree 2 (IEC 60664-1, 6.2). The pads
come 0.48 mm from their slot, which the DRC allows for J2 and J3 only.

U1 uses a `Combined_Local` copy of the SMDIP footprint with a 1 mm routed
slot between the pad rows, extending 1 mm past each end of the
body, as on the RobotDyn dimmer. It cuts the surface creepage path under the
package against dust or residue.

When laying out the PCB:

- Keep at least 7 mm between any 3V3/GND copper and any mains copper, on both
  layers.
- Run N on the bottom layer and L on the top. They may cross through the
  board: L to N is functional insulation, and the FR4 is far beyond 325 V.
  Along the connectors, N runs under their bodies, between the pins and the
  edge, at least 4.45 mm from the pin axis, and reaches each N pin from that
  side, or from the side away from the slot; the L traces leave the pins
  towards the inside of the board.
- Place J2 and J3 side by side with their N pins adjacent (J2 pin 2, J3
  pin 1): N joins them with one short track, and L_IN and L_VALVE stay on
  the outer pins.
- The output pin of Q2 is trapped between its gate trace and the L feed of
  U1 (R5): L_VALVE leaves pin 1 through one via (0.6/0.3 mm, about 1 A,
  for 65 mA) and reaches J3 on the bottom layer. No rotation avoids it, since
  rotating a part keeps the order of its pads.
- Keep L_IN, between J2.1 and F1, short; F1 does not protect it.
- Do not pour GND under the mains side.
- Keep the L, L_VALVE and N traces short and at least 1 mm wide.
- Give the Q2 DPAK tab (MT2 = L) some copper. This copper is at mains
  potential.

## Open points

- Enclosure (`boitier_dc`), outline and mounting of the v2 board, to be
  decided with its layout. The outline already has two 6.6 × 3.9 mm notches
  for the M2 lid corbels of `boitier_ac` / `boitier_dc`: on the west edge of
  the 24 V wing (X 55.9–59.8, Y 112.5–119.1) and on the south edge of the
  mains band east of H6 (X 123.9–130.5, Y 78.1–82.0). Its low-voltage side, including the 24 V input
  and the pump side, must keep 7 mm from every mains net.
- **Before ordering v3: load the yellow wire with 6 mA** (820 Ω from yellow
  to green, pump powered, motor stopped) and check that it stays near
  4.98 V. U8, R38 and the filter draw 2.5 to 4.3 mA typical and about 6 mA
  at worst; the wire was only checked up to 5 mA.
- **First power-up of v3, before the pump turns**, pump on J9, XIAO
  firmware with PUMP_SPEED at 0:
  - SPEED_P at 0 V with the XIAO removed, and with the XIAO in place but
    held in reset (RESET button held): the default-low output and R34.
  - One point, 50 % duty with the pump powered: SPEED_P against
    0.976 × duty × V(yellow), within 1 %; this fixes the firmware's factor.
  - Blue at the scope against VI2's thresholds: low under 1.49 V with
    R38's 0.5 mA, high above 3.49 V.
  - PUMP_TACHO: one clean falling edge per blue falling edge, no extra
    edge after the rising one (count over a purge against the software
    filter's rejects, which should stay at 0).
- **Firmware: tell v2 from v3.** v2 needs the duty table and sees PUMP_TACHO
  HIGH without the pump; v3 maps the duty linearly and sees it LOW. A
  build or configuration setting is enough: the board cannot report its
  version. To decide in the firmware.
- Pump interface value to check: the start threshold and the low end of
  the setpoint-to-speed curve, below 1.2 V (measured: about 1 000 rpm per
  volt from 1.2 to 5 V).

- The gate, latching and holding values above come from the Philips BT136S
  datasheet (standard -600 gate), the worst case for gate current. The chosen
  JCST BT136S (LCSC C23427, the part in the M5Stack Unit SSR) has a more
  sensitive gate (10 mA max) and a 20 mA max holding current, but specifies
  neither latching current nor dV/dt.

## Assembly and maintenance

U1 is the surface-mount lead-bend option (MOC3043SR2M); the plain part
number is a through-hole DIP and does not fit the SMDIP footprint. U8 must
be the CA-IS3721CLS: the CHS variant defaults high, and the ISO7721 and
CA-IS3722C have the channels the other way. The manufacturing export
(`uv run ../generate_manufacturing.py .`) requires the LCSC column, matching
fields in the schematic, and a DRC-clean PCB.

References, in `../../docs/datasheets/`:
- `m5stack-unit-ssr.md` (valve channel origin), `KS53_EN.pdf` (boiler SSR).
- `MOC3043M-D.PDF`, `MOC3163M-D.PDF` (onsemi optotriacs).
- `bt136.pdf` (JCST BT136S, LCSC C23427, the M5Stack Unit SSR part),
  `el817.pdf` (Everlight, v2 U8 and U9), `mf2410.pdf` (AEM MF2410 fuses, F1).
- `CA-IS3721CLS.pdf` (Chipanalog CA-IS372xC, version 1.03, U8),
  `TJA1051.pdf` (NXP, U4), `nau7802.pdf` (Nuvoton, on the scale board).
- `lmr33630.pdf` (TI, U10), `xiao_esp32s3_sch.pdf` (Seeed XIAO ESP32-S3
  schematic v1.2), `gearpump-mpb003.pdf` (Chenhui MPB003 pump); pump notes
  in `../../docs/pompe-engrenage.md`.

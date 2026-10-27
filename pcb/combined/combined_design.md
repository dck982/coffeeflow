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

The schematic has three sheets. `combined.kicad_sch` holds the valve
channel and the pump interface, as below. `combined_controller.kicad_sch`
(24 V input and 5 V buck, XIAO, status LED, pressure ADC) and
`combined_interfaces.kicad_sch` (CAN, I2C pull-ups and TMP102, flowmeter,
boiler SSR output) come from `../sensors/`, except the 24 V input and the
buck. The design notes in `sensors_design.md` still apply to those circuits,
with the references renumbered as listed in "Low-voltage side from
sensors". `combined.pdf` is a
generated review copy. `combined.kicad_pcb` is still the v1 layout: it must
be updated from the schematic, placed and routed again.
`combined_bom.csv` lists values, footprints and design notes.
Every fitted part has its JLCPCB part.

The board switches the 230 V solenoid valve (OLAB coil, 15 VA, about 65 mA)
from 3.3 V logic, with the circuit of the M5Stack Unit SSR it replaces.

## Connectors

| Connector | Type | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|---|
| J1, 5 V to the screen | JST XH 2 | GND | +5V | — | — |
| J2, mains in | HT396R 2 | L | N | — | — |
| J3, valve | HT396R 2 | N | L switched | — | — |
| J5, CAN | JST XH 2 | CAN_L | CAN_H | — | — |
| J6, flow | JST XH 3 | GND | +5V | FLOW_PULSE | — |
| J7, 24 V in | HT396V 2 | GND | +24V | — | — |
| J8, boiler SSR | HT396V 2 | GND | BOILER_5V | — | — |
| J9, pump | JST XH 4 | GND_P (green) | SPEED_P (brown) | TACHO_P (blue) | +5V_P (yellow) |
| J10, pressure | JST XH 3 | +3V3 | GND | PRESSURE | — |

J1 was the 5 V input in v1; it is now the 5 V output to the screen, whose
two wires run along the CAN cable. J7 comes from the Wago split of the 24 V
supply, which also feeds the pump's red and black wires: the pump current
does not cross the board. J9 carries only the pump's isolated signal wires;
the white wire (rotation) stays unconnected in the cable.

The former 5-pin control header (GND, 3V3, DIM, ZC, SSR) is gone: SSR,
PUMP_SPEED and PUMP_TACHO run as tracks to the XIAO sockets, and 3V3 comes
from the XIAO. The sensors board's two Grove I2C headers and its VALVE
header are not carried over either: the valve SSR is on board.

J7 (24 V in) and J8 (boiler SSR) are Kefa HT396V-3.96-2P vertical headers
(LCSC C577788). Both mate with the HT396K-3.96-2P plug (LCSC C577777, screw
terminals, 0.2 to 1.5 mm², AWG 14 to 26). J7's plug takes the two wires
from the Wago split. J8's takes the two 0.75 mm² wires to the boiler SSR
directly; the SSR end uses red 4.8 mm crimp lugs, which need that section
and do not fit a JST XH crimp. On sensors, two Wago changed the section.
The plugs are not assembled: order them with the board. The LCSC footprint
matches the Kefa drawing (`../../docs/datasheets/C577788.pdf`): body
7.92 × 7.40 mm centred on the pins, 10.1 mm high; the courtyard only gained
a 0.25 mm margin. Leave room beside each header for the plug's wire entry
and for a screwdriver above.

J2 and J3 are HT396R-3.96-2P right-angle headers (LCSC C577799, a JST VH
clone). Their footprint and 3D model come from LCSC, in `Combined_Local`.
The outline was redrawn from the Kefa drawing
(`../../docs/datasheets/C577799.pdf`), as on ps_upstream: the mating face is
9.6 mm in front of the pin axis and the body 0.5 mm behind it. The silkscreen
stops 0.4 mm short of the mating face; the courtyard covers the pads, the
body and the slot, which reaches 2.4 mm behind the pin axis. Place the
connectors with the pins inside the board and the mating face at the edge:
pin axis 9.6 mm from the edge. The pads, holes
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
  base current for about 12 mA of collector current, the transistor
  saturates.
- R3 (180 Ω) sets the optotriac LED current: (3.3 − 1.2 − 0.1) / 180 ≈ 11 mA
  typical, at least 8 mA worst case (LED forward voltage 1.5 V max). U1 is
  guaranteed to trigger at 5 mA; onsemi recommends any current between that
  and the 60 mA maximum.
- D1 (red) with R4 (1 kΩ) lights at about 1.2 mA when the channel is
  commanded. It sits in parallel with the optotriac LED, so it shows the
  command, not whether the valve is actually conducting.

Load on the 3V3 rail: about 12 mA with the valve on, plus 0.7 mA for the
PUMP_TACHO pull-up, drawn from the XIAO 3V3 without a local bulk capacitor.

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
- R7 (39 Ω) + C2 (10 nF, 1 kV X7R 1812, PSA FV43X103K102EFG) form a
  snubber across Q2. At turn-off on an inductive load, the voltage across the
  triac jumps quickly. The snubber limits that dV/dt so Q2 does not
  re-trigger on its own. At 50 Hz, C2 lets about 0.7 mA through the valve
  while it is off (230 V / 318 kΩ). A solenoid ignores this.

## Triac dissipation

The BT136S on-state model is 1.27 V + 0.091 Ω, and the junction-to-ambient
resistance is 75 K/W on the datasheet's FR4 footprint. The valve costs Q2
under 0.1 W. It needs no heat sink beyond the DPAK tab copper.

## 24 V input and 5 V buck

J7 brings 24 V from the pump supply. The buck is not isolated: board GND is
the supply's negative, the pump's black wire.

U10 is a TI LMR33630ADDAR (LCSC C841384): synchronous buck, 3.8 to 36 V in,
3 A, 400 kHz, HSOP-8 with a thermal pad (pin 9, GND).

- Output: 1.0 V reference (0.985 to 1.015 V) and R28 / R29 (100 kΩ /
  24.9 kΩ): 1.0 × (1 + 100 / 24.9) = 5.02 V, 4.94 to 5.09 V from the
  reference tolerance.
- Enable as undervoltage lockout: R30 / R31 (100 kΩ / 7.15 kΩ) from VIN.
  EN starts the switching at 1.231 V (1.2 to 1.26 V) and stops it 100 mV
  lower: start at 18.4 V (18.0 to 18.9 V), stop at 16.9 V. The board waits
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
- Input: C1, 10 µF 50 V X7R 1210 (Samsung CL32B106KBJNNNE, LCSC C138687),
  and C5, 220 nF 50 V X7R 0603 (Yageo CC0603KRX7R9BB224, LCSC C107083,
  extended part: the basic 220 nF is 25 V) next to the VIN and PGND pins.
  This is TI's reference input for the HSOP package at 400 kHz, sized for
  3 A out; the board draws about 1 A at most.
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

Layout, for the board update:

- C5, then C1, right at VIN and PGND: the input loop carries the switched
  current and must stay small.
- Keep the SW node, C7 and L1 compact; nothing sensitive under L1.
- Thermal pad soldered to a GND area with the footprint's vias to a bottom
  GND plane.
- R28 / R29 next to FB, away from SW; feedback taken at the output
  capacitors.

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

Two EL817C (U8, U9) keep the pump's signal wires (green GND_P, yellow
+5V_P, brown SPEED_P, blue TACHO_P) on their isolated side: they never touch
a ground or a supply of the board. The pump datasheet does not say whether
green is tied to black inside; the SOVOFLO family forbids tying the setpoint
reference to the power ground. The circuit works either way.

Setpoint (U8):

- PUMP_SPEED (GPIO1) lights the U8 LED through R33 (330 Ω):
  (3.3 − 1.2) / 330 ≈ 6 mA. R34 (100 kΩ) holds GPIO1 low.
- U8's collector is on the yellow 5 V, its emitter pulled to green by R35
  (10 kΩ). LED on, the emitter rises to about 5 V; 0.5 mA is enough at a CTR
  of 200 to 400 %.
- Filter: R36 / C20 and R37 / C21, 2 × 2.2 kΩ / 1.5 µF (48 Hz each), between
  the emitter and the brown wire, capacitors to green. C20 and C21 are
  25 V X7R 0805 (0805B155K250CT, LCSC C384767). With the LEDC at 1 kHz and
  16 bits, the ripple is about 5 V × (48 / 1000)² ≈ 12 mV, about 12 rpm, and
  the response time about 10 ms. The source impedance, about 4.4 kΩ,
  matches a 10 kΩ potentiometer at mid travel.
- U8 off, R35 brings the setpoint to green: 0 at power-up, during a reset,
  with the XIAO absent or the firmware stopped.
- The PWM bends the setpoint near the ends, monotonically: the speed is read
  back on the tachometer.

Tachometer (U9):

- U9's LED runs from yellow to blue through R38 (1.8 kΩ): about 2 mA while
  blue is low. This works with a push-pull or an open-collector output.
- U9's transistor pulls PUMP_TACHO (GPIO3) to GND against R39 (4.7 kΩ to
  3V3, 0.7 mA). The signal is inverted; the PCNT counts its edges.
- The EL817C follows up to about 2.7 kHz, 5 000 rpm at 32 pulses per turn.
  A faster opto (6N137) would replace it if the pump gives more pulses.
- GPIO3 is a JTAG strapping pin, read only when the `STRAP_JTAG_SEL` eFuse
  is burnt, which it is not by default.

The pump's yellow wire supplies about 2.5 mA (0.5 mA for the setpoint, 2 mA
for the tachometer). If it cannot, R38 goes up.

U8 and U9 are on `Package_DIP:SMDIP-4_W9.53mm`, the pads of the v1 U3
without the slot: the pump side is low voltage. Between the pump side and
board GND, the isolation is functional only and the default rules apply. On
the board, group U8, U9, R35–R38, C20, C21 and J9, and keep the board GND
pour away from GND_P and +5V_P so that they are not tied by accident.

## Low-voltage side from sensors

Everything on the sensors board except its Grove I2C headers (J4, J5) and its
VALVE header (sensors J7, with its R17 pull-down) is copied with unchanged values and
connections. The 4.7 kΩ SDA/SCL pull-ups stay: the TMP102 and the ADS1115
are on the bus. References that clashed with the AC side were renumbered:

| sensors | combined | Part |
|---|---|---|
| U1 | U7 | XIAO ESP32-S3 (schematic only, plugs into J11/J12) |
| D2 | D4 | Status LED, red 0603 here (green 0805 on sensors) |
| J3 | J5 | CAN connector |
| Q2 | Q5 | AO3401A, boiler output |
| Q3 | Q6 | 2N7002, boiler output |
| R11 | R17 | TJA1051 TXD pull-up |
| R12 | R18 | CAN termination, 120 Ω 1206 |
| R13, R14 | R20, R23 | SDA, SCL pull-ups |
| R15 | R24 | FLOW pull-up |

The other references (C8–C15, H1, J1, J6, J8 (now an HT396V), J10–J12, R19, R21, R22, R25,
R26, U4–U6) keep their sensors numbers. The XIAO footprint and its socket
footprint are copied into `Combined_Local`; `sym-lib-table` points to the
local Seeed XIAO symbol library, as in sensors.

XIAO pins used by the valve and the pump:

| Signal | GPIO | Seeed name | U7 / J11–J12 pin | Firmware |
|---|---:|---|---|---|
| PUMP_SPEED | 1 | D0 | U7 / 1, J11 / 1 | LEDC PWM, 1 kHz, 16 bits; 0 at boot |
| PUMP_TACHO | 3 | D2 | U7 / 3, J11 / 3 | PCNT on the inverted signal, glitch filter |
| SSR | 9 | D10 | U7 / 11, J12 / 4 (former VALVE_CMD) | active HIGH |

The other pins keep their sensors assignments; only D6/GPIO43 stays free.
PUMP_SPEED and PUMP_TACHO take the former DIM and ZC pins. SSR stays on
GPIO9, active HIGH, as VALVE_CMD was.

The firmware recognises a pump fault (undervoltage, overvoltage,
temperature, locked rotor) when the tachometer stays silent while the
setpoint is above 0.3 V. It then sets the setpoint to 0 for at least 10 ms,
which re-arms the pump, logs the fault and does not restart on its own during
a shot. The pump ignores a setpoint sent within 65 ms of a cold start.

The XIAO 3V3 output also supplies the valve logic (12 mA with the valve on)
and the PUMP_TACHO pull-up, on top of the sensors load.

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
  inhibit voltage 15 V max), same footprint and pinout. R3 stays at 180 Ω.

The snubber capacitor (1 kV X7R MLCC) can also fail short, leaving 39 Ω
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
  snubber resistor). R5 drops at most 13 V for 0.3 ms per half-cycle. 1.5 mm,
  the clearance.
- Q2's gate and R6 with L_VALVE. 1.5 mm.

Pads of one part keep their footprint spacing. A mains track may come to
1.25 mm of a mains pad of the part it enters, inside that part's courtyard,
and two mains tracks ending on the pads of R5 or R6 may come to 1.8 mm, the
pad gap of a 1206; these resistors see little voltage. C2 has the full mains
across it when Q2 is off, so it is an 1812: 2.7 mm between pads.

The v1 netclass patterns and rules for the pump channel and the zero-cross
detector are removed. Until the board is updated from the schematic, its
old pump and zero-cross copper has no class left and trips the 7 mm rule.

H2, the second M2.5 mounting hole, sits on the mains side, left of the
notch, 6.3 mm from the nearest mains copper. It takes a nylon screw and a
nylon standoff of at least 5 mm, screwed into a brass insert of the
enclosure: the insert stays at least 5 mm from the board, the reinforced
distance, so it cannot bridge the mains to H1 and the 3V3 side. H3, in the
top-left corner of the low-voltage side, has no such constraint.

J2 and J3 (Kefa HT396R-3.96) are rated 320 V at
overvoltage category II and pollution degree 2 by their IEC data, with 1 mm
pins in 1.6 mm holes (`../../docs/datasheets/C577799.pdf`). They have L and N
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
  U1 (R5): L_VALVE leaves pin 1 through two vias (0.8/0.4 mm, about 1.5 to
  2 A each) and reaches J3 on the bottom layer. No rotation avoids it, since
  rotating a part keeps the order of its pads.
- Keep L_IN, between J2.1 and F1, short; F1 does not protect it.
- Do not pour GND under the mains side.
- Keep the L, L_VALVE and N traces short and at least 1 mm wide.
- Give the Q2 DPAK tab (MT2 = L) some copper. This copper is at mains
  potential.

## Open points

- Enclosure (`boitier_dc`), outline and mounting of the v2 board, to be
  decided with its layout. Its low-voltage side, including the 24 V input
  and the pump side, must keep 7 mm from every mains net.
- Pump interface values to check on the first board: the current the
  yellow wire supplies, the tachometer output type and its pulses per
  turn, and the setpoint-to-speed curve.

- The gate, latching and holding values above come from the Philips BT136S
  datasheet (standard -600 gate), the worst case for gate current. The chosen
  JCST BT136S (LCSC C23427, the part in the M5Stack Unit SSR) has a more
  sensitive gate (10 mA max) and a 20 mA max holding current, but specifies
  neither latching current nor dV/dt.

## Assembly and maintenance

U1, U8 and U9 are the surface-mount lead-bend options (MOC3043SR2M,
EL817S1(C)(TU)-F); the plain part numbers are through-hole DIPs and do not
fit the SMDIP footprints. The manufacturing export
(`uv run ../generate_manufacturing.py .`) requires the LCSC column, matching
fields in the schematic, and a DRC-clean PCB.

References, in `../../docs/datasheets/`:
- `m5stack-unit-ssr.md` (valve channel origin), `KS53_EN.pdf` (boiler SSR).
- `MOC3043M-D.PDF`, `MOC3163M-D.PDF` (onsemi optotriacs).
- `bt136.pdf` (JCST BT136S, LCSC C23427, the M5Stack Unit SSR part),
  `el817.pdf` (Everlight), `mf2410.pdf` (AEM MF2410 fuses, F1).
- `lmr33630.pdf` (TI, U10), `xiao_esp32s3_sch.pdf` (Seeed XIAO ESP32-S3
  schematic v1.2), `gearpump.pdf`; pump notes in `../../docs/pompe-engrenage.md`.

# CoffeeFlow scale board

**Version 2, November 15, 2026:** the board becomes a NAU7802 (U1), a 24-bit
bridge ADC with its own LDO, read over I2C by the XIAO of the combined board.
It has no microcontroller and no firmware. The ADS1232, the XIAO ESP32-S3 and
its two sockets, the +5VA filter (10 Ω and 10 µF), the PDWN and CAN_TX
resistors and the GPIO status LED are gone. Why the NAU7802, why 5 V and the
cable are covered in `../combined/combined_design.md`, "Scale bus", which is
authoritative for the interface.

The board sits under the drip tray, next to the load cell. A 4-wire cable of
20 to 25 cm (GND, +5V, SDA, SCL) comes through a 4 mm hole from the combined
board's J13. Datasheet: `../../docs/datasheets/nau7802.pdf` (Nuvoton,
rev 2.6). Page numbers below refer to it.

`scale.kicad_sch` is the schematic, `scale.pdf` a generated review copy,
`scale_bom.csv` the part list with JLCPCB numbers. `Scale_Local.kicad_sym`
holds the U1 symbol (KiCad has none). `scale.kicad_pcb` is still the v1
layout: it must be updated from the schematic, placed and routed again.

## Connectors

Both are Dorabo DB125-2.54-4P-GN-S screw terminal blocks (LCSC C918122,
2.54 mm, 26 to 18 AWG, M1.6 screw, 0.1 N·m at most). The wires pass through
the hole bare, then get their ferrules and go into the block.

| Connector | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|
| J1, bus (to combined J13) | SDA | SCL | +5V | GND |
| J2, load cell | A+ (green) | A− (white) | E− (black) | E+ (red) |

Straight cable: combined J13 has the same order (1 SCALE_SDA,
2 SCALE_SCL, 3 +5V, 4 GND).
SDA and SCL are 5 V I2C. The pull-ups, 2.2 kΩ to +5V, are on the combined
board; there are none here.

## Circuit

- DVDD (pin 15) takes +5V from J1. The internal LDO drives AVDD (pin 16) at
  4.2 V; AVDD feeds REFP (pin 1) and the bridge (E+), so the reading is
  ratiometric. REFN, AVSS, DVSS and E− go to GND.
- Headroom: the LDO needs DVDD at least 0.3 V above its output, and AVDD
  must not exceed DVDD (p. 4, p. 7): 0.8 V here. At gain 64 and 128 the
  inputs must stay between AVSS + 1.5 V and AVDD − 1.5 V (p. 7): the bridge
  midpoint, 2.1 V, sits in a 1.5 to 2.7 V window.
- The LDO load must stay under 10 mA (p. 4): the cell must be above about
  420 Ω at 4.2 V. The cell in use measures 1 000 Ω (red to black, and green to white) and draws 4.2 mA; a 350 Ω cell would draw 12 mA,
  too much.
- XIN, XOUT and DRDY are not connected: internal RC oscillator, which is
  the power-on default (p. 21), and the firmware polls CR instead of DRDY
  (p. 13).

## Values

| Ref | Value | Role | Source |
|---|---|---|---|
| C1, C2 | 10 µF, 100 nF | +5V bulk and bypass at J1 | kept from v1 |
| C3 | 1 µF X5R | DVDD bypass, at pin 15 | application circuit, p. 24 |
| C4 | 1 µF X5R | AVDD/LDO output, at pin 16 | p. 24; pin description p. 6 asks low ESR, 1 Ω or less; LDOMODE = 0 (default) needs ESR < 1 Ω (p. 33); a ceramic has a few mΩ |
| C5 | 100 nF X7R | REFP to GND, at pin 1 | p. 24 |
| C6 | 100 nF X7R | VBG bypass, at pin 6 | "high quality X7R small value 0.1 µF", §1.13, p. 19 |
| R1, R2 | 47 Ω | series A+ to VIN1P, A− to VIN1N | p. 24 |
| C7 | 100 nF X7R | across VIN1P and VIN1N | p. 24; with R1 and R2, 17 kHz differential corner before the cell's own resistance |
| C8 | 680 pF X7R (C1630, preferred extended) | Cfilter across VIN2P and VIN2N, PGA output filter | p. 24: 330 pF at 3.3 V, 680 pF at 4.5 V; 4.2 V is not listed, 680 pF is the nearest row |
| R3, D1 | 2.2 kΩ, red | power LED from +5V, about 1.4 mA | as on combined |

C8 only acts with PGA_CAP_EN set (REG0x1C bit 7, p. 34), and channel 2 can
then no longer serve as an input: the firmware stays on channel 1.

The board draws about 7.7 mA from +5V: 2.1 mA for the NAU7802 (p. 7),
4.2 mA for a 1 kΩ cell, 1.4 mA for the LED.

## Firmware, from the datasheet

- Address 0x2A, fixed (p. 14).
- Nothing runs until configured (p. 22). Sequence:
  1. REG0x00: RR = 1, then RR = 0 with PUD = 1; wait for PUR (bit 3), about
     200 µs.
  2. REG0x01: VLDO = 001 (4.2 V) and the gain, 0x0F for gain 128 (p. 27).
  3. REG0x00: AVDDS = 1 (internal LDO), PUA = 1, PUD = 1, that is 0x86.
  4. REG0x02: rate in CRS (10, 20, 40, 80 or 320 SPS), CHS = 0 (p. 28).
  5. REG0x15 = 0x30 (REG_CHPS = 11, recommended, p. 32).
  6. REG0x1C: PGA_CAP_EN = 1 for C8.
  7. REG0x11: WPD = 1 to disable the internal 50 kΩ pull-ups (p. 30);
     the 2.2 kΩ on combined are enough.
  8. Internal offset calibration: CALMOD = 00, CALS = 1, wait for CALS to
     return to 0, then check CAL_ERR (REG0x02 bit 3, p. 18). Calibrate
     again after a change of gain, rate, channel or supply.
  9. Drop the first six conversions (p. 20).
- Read by polling CR (REG0x00 bit 5), then a 3-byte burst read from 0x12
  (p. 31).
- Never enable CRSD (REG0x11 bit 7): the chip would then hold SDA low to
  signal each conversion (p. 17) and block the shared bus.

## Terminal block footprint

Drawing: `../../docs/datasheets/db125-2.54.pdf`, the LCSC datasheet for
C918122 (Dorabo DB125-2.54-XXP-C-S, rev T0-1).
Pitch 2.54 mm, PCB hole 1.20 mm, pin 0.50 × 0.85 mm, body
N × 2.54 + 0.48 = 10.64 mm long (1.51 mm past the end pins), 6.30 mm deep,
8.40 mm high; the pin row sits 3.10 mm from the wire-entry face (the low
face, under the slope, in the side view) and 3.20 mm from the back face.

KiCad's closest stock footprint, `TerminalBlock_Xinya_XY308-2.54-4P`, has the
same pitch and a 1.2 mm drill with 2.0 mm pads, but a 10.66 × 6.50 mm body
whose entry face lies 3.40 mm from the pins: 0.3 mm too deep. The board uses
`Scale_Local:TerminalBlock_Dorabo_DB125-2.54-4P_1x04_P2.54mm_Horizontal`:
the Xinya pads and drill, the DB125 outline, the wire-entry side on +Y
marked by a second silkscreen line, the pin-1 triangle inside that entry
strip (nothing past the body, so the block can sit at the board edge), and
a courtyard 0.5 mm around the body. Keep the entry face at the board edge,
or leave room for the wires and the screwdriver in front of it. The 3D model is EasyEDA's (LCSC C918122, `easyeda2kicad`), in
`Scale_Local.3dshapes/DB125-2.54-4P-GN.step`. Its pins run from 0 toward −X
and the wire entry faces −Y, which is the footprint's +Y once KiCad flips Y:
the only correction is an X offset of 7.62 mm. The body spans 10.64 ×
6.27 mm, 3.13–3.14 mm each side of the pin row; the clamp plates show
0.35 mm out of the entry face.

## First power-up

1. Board on the cable, cell connected: +5V at C1, and after the firmware has
   run its init, 4.2 V at C4 (before, AVDD floats: the LDO starts off).
2. The firmware finds 0x2A, calibrates without CAL_ERR, and a known weight
   on the tray moves the reading.

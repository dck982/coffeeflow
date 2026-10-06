# CoffeeFlow screen sensor board

The connected schematic is `screen_sensors.kicad_sch`; the manually
reworked layout is `screen_sensors.kicad_pcb`. The schematic PDF and
netlist are generated review copies. `screen_sensors_bom.csv` lists
component sourcing information; assembly exports still need final preparation.

## Connectors and supply

All connectors are through-hole, vertical/top-entry JST XH, for JLCPCB assembly.
Genuine XH pitch is 2.50 mm, often advertised as 2.54 mm by generic sellers.
Follow numbered pads rather than assuming left-to-right cable orientation.

| Connector | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|
| J1, Waveshare I2C | 3V3 | GND | SCL | SDA |
| J2, boiler NTC/PT1000 | 3V3 | boiler divider midpoint | — | — |
| J3, analog XDB401 | 3V3 | GND | pressure output | — |

J1 intentionally swaps SDA/SCL compared with the Waveshare connector order
(VCC, GND, SDA, SCL), to simplify PCB routing. Cross those two wires when
building the custom cable. The selected headers are B4B-XH-A, B2B-XH-A-R,
and B3B-XH-A for J1, J2 and J3 respectively; exact MPNs are in the BOM.

Waveshare H8 must select 3V3. There are no additional SDA/SCL pull-ups on this
board because Waveshare H7 supplies them. Check the total sensor current
against the Waveshare supply capacity once the pressure sensor arrives.

## Circuit and component references

- U1 ADS1115IDGSR: address 0x48, ADDR grounded, ALERT/RDY unused. A0 reads
  the local 3V3 that excites the boiler probe, A1 reads the boiler divider,
  A2 reads pressure, and A3 is unused with a schematic no-connect.
- R1 is 4.7 kΩ, 0.1%, from the boiler divider midpoint to GND. The NTC or
  PT1000 probe connects between 3V3 and that midpoint. The same fixed resistor
  supports either probe; use the corresponding firmware conversion/calibration.
- R3 is 1 kΩ, 1%, in series between the pressure sensor output and A2.
  With C3 = 10 µF it forms a low-pass filter with nominal cutoff about 16 Hz
  and time constant 10 ms. R3 is UNI-ROYAL 0603WAF1001T5E, JLCPCB C21190
  (basic). It provides modest current limiting, not complete input protection.
  The ADS1115 input (about 6 MΩ) loads R3 by about 0.02 %, absorbed by the
  pressure calibration.

  C3 was 100 nF (cutoff 1.59 kHz) until October 6, 2026. That only removes
  fast edges: 50 Hz and 100 Hz mains pickup passes, and the ADS1115 at 128 SPS
  averages over 7.8 ms, less than one mains period, so the pickup aliases into
  sample-to-sample noise. On the current wiring, the mounted sensor's A2 noise
  rose from 1.5 codes standard deviation in open air to 7.9 codes (0.006 bar)
  once its cable ran near the 230 V power switch wiring. Combined attenuation,
  RC filter times ADS1115 averaging:

  | R3 / C3 | Cutoff | Time constant | 50 Hz | 100 Hz |
  |---|---|---|---|---|
  | 1 kΩ / 100 nF (previous) | 1.59 kHz | 0.1 ms | 0.77 | 0.26 |
  | 1 kΩ / 10 µF (current) | 16 Hz | 10 ms | 0.23 | 0.04 |
  | 10 kΩ / 4.7 µF (rejected) | 3.4 Hz | 47 ms | 0.05 | 0.01 |

  The 47 ms option would add half a pressure-control period (10 Hz) of lag.
  X5R DC bias at the 0.4–2.4 V output lowers the effective capacitance a little,
  raising the cutoff slightly; this does not change the choice. The
  attribution of the noise to mains pickup is not yet confirmed: an RC test on
  the current wiring is planned.
- U2 SHT40-AD1B-R3: address 0x44, measures enclosure humidity and temperature.
  Leave its heater off during normal sampling. Keep the sensing opening free
  of flux, washing, coating and adhesives; specify no board wash for assembly.
  It uses the same top-side SMT reflow pass as U1.

| Capacitor | Value | Function |
|---|---|---|
| C1 | 100 nF | ADS1115 supply decoupling, close to VDD with a short ground return |
| C2 | 4.7 µF | Board bulk supply capacitance, near incoming power at J1 |
| C3 | 10 µF | Pressure ADC input to GND, paired with R3 (16 Hz low-pass) |
| C4 | 100 nF | Supply bypass at pressure connector J3 |
| C5 | 100 nF | SHT40 supply decoupling |

Capacitors were renumbered after layout: old C4 → C3, old C5 → C4, and
old C6 → C5. C1 and C2 retained their references. The original boiler filter
R2 and old C3 were removed at the user's request; the present C3 belongs to
pressure measurement. The boiler input has no added series resistor or filter
capacitor. The local NTC connector and its associated passives were removed
because U2 provides enclosure temperature.

## Power indicator

POWER is a red 0603 LED, Hubei KENTO KT-0603R (JLCPCB C2286, basic part).
R4 is 2.2 kΩ, 1%, UNI-ROYAL 0603WAF2201T5E (C4190, basic part).
The branch is `3V3 → R4 → POWER anode (pin 2) → cathode (pin 1) → GND`.
Nominal current is approximately 0.6 mA for a 2 V forward voltage. Brightness
and actual current depend on the LED forward voltage at this low current.
The LED indicates incoming power only, not working I2C communication.
Place the indicator near J1 and away from U2 when updating the PCB.
The LED and R4 are placed and routed on the PCB. The LED reference is
POWER in the PCB, schematic and source BOM.

Sources: [POWER](https://jlcpcb.com/partdetail/KT-0603R/C2286),
[R4](https://jlcpcb.com/partdetail/0603WAF2201T5E/C4190).

## PCB layout

The reworked board is 30 × 30 mm, with four 3.2 mm M3 mounting
holes. Components are on the top side; the underside is free of components.
J1 is at the north edge, J2 west, J3 south, and U2 near the east edge.

The bottom layer contains the GND plane and a few short signal/supply crossings.
A top-side GND pour uses the same outline and sensor-area exclusion as the
bottom plane. Both pours are connected; five redundant ground vias were removed,
leaving three ground vias (two near U1 and one for the SHT40 ground return),
plus six signal/supply vias. Each filled layer has one connected copper region.
Keep ordinary component ground returns short, using nearby vias where useful.
Keep vias outside solder pads to avoid requiring filled/capped via processing.
The SHT40 area is excluded from the ground pour to reduce board heat conduction;
its narrow ground connection terminates at a via outside that exclusion.
A via is electrically permitted for U2; avoiding a direct connection to a large
plane near the sensor is a thermal-design choice. Provide access to enclosure
air and keep the sensor away from display heat.

## Firmware and calibration

Read A0 and A1 with the same PGA, initially ±4.096 V:
`Rprobe = Rfixed * (A0_raw / A1_raw - 1)`.
Calibrate the assembled board. The bench value 4676 Ω inferred on September 29
belongs to the old measurement setup and must not automatically be reused.
Select the NTC curve or IEC 60751 Callendar–Van Dusen conversion for PT1000 as
appropriate. Detect open/short/out-of-range boiler readings and inhibit heating
appropriately. Do not interpret the unused A3 reading as a measurement.

The purchased XDB401 is invoiced as 3.3 V supply, 0.4–2.4 V output, 0–12 bar;
its body is engraved "SUP: 5V". Nominal conversion is
`pressure_bar = 6 * (Vout - 0.4)`. No voltage divider or amplifier is included.

Measured on the received sensor (October 6, 2026), on the current wiring
through the Waveshare-side ADS1115:

- The output is not ratiometric: 397.7 mV in open air with either a 5 V or a
  3.3 V supply. Use the absolute A2 voltage; A0 does not enter the pressure
  conversion.
- Open-air zero: 3167.3 codes (395.9 mV), 1.5 codes standard deviation. The
  sensor reads gauge pressure.
- Loading the 3V3 rail: none visible on A0.

Pending: whether to supply the sensor from 3.3 V or 5 V. At 3.3 V, the open
point is output headroom near full scale (about 2.04 V at the 9.8 bar OPV
plateau); it is checked by the pressure comparison against the previous I2C
sensor. This board only provides 3V3 on J3 pin 1, so a 5 V supply would need a
5 V source on J3. With the ADS1115 at 3.3 V, A2 must stay below about 3.6 V; if
a 5 V-supplied sensor failed high, R3 would limit the input current to about
1.4 mA.

## Assembly and maintenance

The BOM prefers basic parts where possible, with R1 retained at 0.1% precision; R3 and R4 use basic 1% parts.
Confirm current JLCPCB stock, exact MPN/footprint correspondence and through-hole
assembly availability before ordering. Final assembly BOM, placement file and
manufacturing outputs must be generated from the reviewed design.

The KiCad schematic and manually reworked PCB are the design sources of truth.
The initial Python generation/routing helpers and intermediate review backup
were removed after the manual rework. Matching schematic labels are electrically
connected.

KiCad may create `.history/` for automatic local snapshots; it is excluded from
Git. These snapshots can be managed through KiCad's File > Local History menu.

References:
- `../../docs/ntc_ads1115_calibration.md`, `../../docs/cablage.md`.
- [ADS1115 datasheet](https://www.ti.com/lit/ds/symlink/ads1115.pdf).
- [SHT4x datasheet](https://sensirion.com/resource/datasheet/sht4x).

## Manufacturing exports

Run `uv run ../generate_manufacturing.py .` (or `python3 ../generate_manufacturing.py .`)
from this directory after saving changes. Python requires no extra dependencies;
KiCad 10's `kicad-cli` must be installed. The script discovers the macOS app or
the executable on PATH; `--kicad-cli` overrides its location. Project-specific
connector centres and rotation corrections
are defined in `customize-manufacturing.py`, loaded automatically by the shared
exporter in `pcb/`.

Generated files are in ignored `manufacturing/`: Gerbers, separate plated and
non-plated drill files, JLCPCB BOM and placement CSVs, DRC report, source hashes,
fabrication-only ZIP and a combined `screen_sensors_jlcpcb.zip`. The source BOM
is `screen_sensors_bom.csv`; its values, footprints and LCSC numbers must
match the saved PCB. Physical DRC findings and schematic parity
differences stop the export.

Connector placement uses the transformed midpoint of each JST XH pin row,
so moving or rotating a connector does not require updating hardcoded positions.
U1 has a +270° JLCPCB model rotation correction. Review connector alignment and
chip/LED polarity in JLCPCB's assembly preview after each new upload.

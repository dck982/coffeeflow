# CoffeeFlow screen sensor board

Connected electrical draft: `screen_sensors_draft.kicad_sch`. Open this file in
KiCad rather than the original `ads1115.kicad_sch`, which was left untouched.
The PDF is a review copy; the CSV is a preliminary component list, not a
finished JLCPCB assembly BOM. No PCB layout has been created.

KiCad 10.0.6 has no schematic-editing IPC API. The generator writes native
KiCad symbols, instances, wires and labels using the installed libraries.
Matching labels connect the circuit even where no long wire is drawn.

## Connectors

The Waveshare cable is custom wired, so use these assignments at the board.
Follow the numbered pads, not an assumed left-to-right orientation.

All four connector footprints are through-hole, vertical/top-entry JST XH:
the mating cable exits perpendicular to the PCB, not sideways. Genuine XH
pitch is 2.50mm (often advertised as 2.54mm by generic sellers). Use the JST
2.50mm specification for these footprints. J1 is B4B-XH-A, J2/J4 are
B2B-XH-A, and J3 is B3B-XH-A. J4 remains optional/DNP.

| Connector | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|
| J1, Waveshare I2C | GND | 3V3 | SDA | SCL |
| J2, boiler NTC/PT1000 | 3V3 | divider midpoint | — | — |
| J3, analog XDB401 | 3V3 | GND | pressure output | — |
| J4, optional local NTC | 3V3 | divider midpoint | — | — |

Waveshare H8 must be set to 3V3. In particular, the SHT40 is not a 5V device.
No extra SDA/SCL pull-ups: Waveshare H7 already supplies them.

## Circuit

- U1 ADS1115IDGSR: address 0x48, ADDR grounded, ALERT/RDY unused.
  A0 measures the same local 3V3 that excites the probe dividers.
  A1 reads the boiler, A2 the pressure, A3 the optional local probe.
- R1 is a single 4.7k, 0.1% resistor from boiler midpoint to GND, suitable
  for both the current NTC and the PT1000. The probe is between 3V3 and midpoint.
  R2 (100 ohms) and C3 (100nF) isolate/filter the ADC input.
- U2 SHT40-AD1B: address 0x44, 100nF C6 close to its VDD pin.
  Place near a board edge with access to enclosure air, away from display heat.
  Follow Sensirion handling instructions; keep its sensing opening free of
  flux, washing, coating and adhesives. Leave its heater off for normal sampling.
- C1 is 100nF at ADS1115 VDD, C2 is 4.7uF board bulk capacitance.
- J3 has C5 (100nF) supply decoupling. R3 (1k) and C4 (100nF) form a
  provisional input filter, nominal corner about 1.59kHz for low sensor
  source impedance. It is an RF/input filter, not a substitute for slower
  digital pressure filtering. No voltage divider or amplifier is required
  for the specified 0.4–2.4V output.
- J4/R4/R5/C7 are DNP (do not populate) by default because SHT40 already
  measures local temperature. They reserve the option of a remote 10k NTC.
  The 10k fixed resistor is provisional pending the probe curve and expected
  temperature. Do not interpret floating A3 as a valid measurement if omitted.

Input series resistors offer modest current limiting; this draft does not
include dedicated connector ESD clamps or protection against external power
voltages. Decide those requirements against cable routing before layout.

## Firmware and calibration

Read A0 and A1 with the same PGA, initially +/-4.096V:
`Rprobe = Rfixed * (A0_raw / A1_raw - 1)`.
The extra series resistor and ADC input loading are part of the assembled
measurement circuit; calibrate the completed board.

The bench value 4676 ohms inferred on September 29 belongs to the old
resistor/measurement setup, not automatically to this board. Use the correct
new-board value. Choose the NTC curve or IEC 60751 Callendar–Van Dusen according
to the attached probe. Revisit the existing NTC temperature offset for PT1000.
Detect open/short/out-of-range boiler readings and inhibit heating appropriately.

XDB401: user specifies 3.3V supply, 0.4–2.4V output, 0–12bar range.
Nominal conversion is `pressure_bar = 6 * (Vout - 0.4)`.
Linearity with pressure does not establish ratiometry with supply. Supply
scaling is unconfirmed; use A2 as an absolute voltage initially. Check zero
pressure, a known pressure and output versus supply when the sensor arrives.
Also confirm wire colors, supply current and permissible output loading.
Budget total sensor current against the Waveshare H7 supply before fabrication.

## Assembly status and checks

The original BOM MPN/LCSC numbers are preserved as leads. Verify supplier
listings, actual connector variants, resistor tolerance/TCR, capacitance under
DC bias, stock and JLCPCB through-hole assembly service before ordering.
New items without LCSC numbers still need sourcing; the SHT40 selection is
specifically the 0x44 AD1B variant. All assigned KiCad footprints exist, but
supplier-to-footprint dimensional checks remain part of layout preparation.

KiCad CLI successfully loaded the schematic, exported its PDF and netlist,
and reported zero ERC errors and warnings. The netlist was checked for power,
I2C pin mapping, ADC channels, boiler divider and pressure filter connections.
ERC does not validate sensor behavior, component sourcing or the PCB layout.

References:
- Project: `../../docs/ntc_ads1115_calibration.md`, `../../docs/cablage.md`.
- [KiCad IPC scope](https://dev-docs.kicad.org/en/apis-and-binding/ipc-api/for-addon-developers/).
- [ADS1115 datasheet](https://www.ti.com/lit/ds/symlink/ads1115.pdf).
- [SHT4x datasheet](https://sensirion.com/resource/datasheet/sht4x).

Regenerate with `python3 build_sensor_schematic.py` only if you intend to
replace the draft: regeneration overwrites it, including any manual edits.

# CoffeeFlow screen sensor board

The connected schematic is `screen_sensors_draft.kicad_sch`; the manually
reworked layout is `screen_sensors_draft.kicad_pcb`. The schematic PDF and
netlist are generated review copies. `screen_sensors_draft_bom.csv` lists
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
- R3 is 1 kΩ, 0.1%, in series between the pressure sensor output and A2.
  Together with C3 it forms a provisional input low-pass filter, with nominal
  cutoff about 1.59 kHz and time constant 100 µs for low sensor source impedance.
  It provides modest current limiting, not complete input protection. Confirm
  sensor output loading and assembled measurement accuracy when testing.
- U2 SHT40-AD1B-R3: address 0x44, measures enclosure humidity and temperature.
  Leave its heater off during normal sampling. Keep the sensing opening free
  of flux, washing, coating and adhesives; specify no board wash for assembly.
  It uses the same top-side SMT reflow pass as U1.

| Capacitor | Value | Function |
|---|---|---|
| C1 | 100 nF | ADS1115 supply decoupling, close to VDD with a short ground return |
| C2 | 4.7 µF | Board bulk supply capacitance, near incoming power at J1 |
| C3 | 100 nF | Pressure ADC input to GND, paired with R3 |
| C4 | 100 nF | Supply bypass at pressure connector J3 |
| C5 | 100 nF | SHT40 supply decoupling |

Capacitors were renumbered after layout: old C4 → C3, old C5 → C4, and
old C6 → C5. C1 and C2 retained their references. The original boiler filter
R2 and old C3 were removed at the user's request; the present C3 belongs to
pressure measurement. The boiler input has no added series resistor or filter
capacitor. The local NTC connector and its associated passives were removed
because U2 provides enclosure temperature.

## PCB layout

The reworked board is 30 × 30 mm, with four 3.2 mm M3 mounting
holes. Components are on the top side; the underside is free of components.
J1 is at the north edge, J2 west, J3 south, and U2 near the east edge.

The bottom layer contains the GND plane and a few short signal/supply crossings.
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

The purchased XDB401 is specified as 3.3 V supply, 0.4–2.4 V output, 0–12 bar.
Nominal conversion is `pressure_bar = 6 * (Vout - 0.4)`.
Linearity with pressure does not establish ratiometry with supply: use absolute
A2 voltage initially. Verify zero pressure, a known pressure, wire assignments,
supply current, output impedance, startup, and output versus supply when the
sensor arrives. No voltage divider or amplifier is included.

## Assembly and maintenance

The BOM prefers basic parts where possible, with 0.1% resistors taking priority.
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

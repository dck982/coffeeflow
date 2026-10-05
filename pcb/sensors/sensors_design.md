# CoffeeFlow sensors controller

`sensors.kicad_sch` is the root schematic; `sensors_interfaces.kicad_sch` is its
second sheet. Open the existing `sensors.kicad_pro`. `sensors.pdf` and
`sensors.net.xml` are exported review copies. The existing `sensors.kicad_pcb`
is still an empty board: this change creates the schematic, not a PCB layout.

## Controller and power

U1 is **ESP32-S3-MINI-1U-N8**, 8 MB flash, no PSRAM. The external-antenna
variant measures **15.4 × 15.4 mm**, whereas MINI-1 with PCB antenna measures
15.4 × 20.5 mm. No external antenna will be connected. Firmware must leave
Wi-Fi/BLE and their transmitters disabled; communication is exclusively CAN.
KiCad's `RF_Module:ESP32-S2-MINI-1U` footprint is the standard footprint assigned
to its ESP32-S3-MINI-1U symbol; the module's 65 pads use the same land pattern.
Connect all GND pads, including the underside pads, during layout.

J1 supplies external regulated **5 V** from the RECOM supply in production.
During initial flashing, disconnect J1 and power the board from USB-C J2.
Both inputs join the same +5 V rail. U2 is AMS1117-3.3 in SOT-223.
C1/C2 bypass its input (4.7 µF / 100 nF); C3 is **22 µF solid tantalum, 10 V**, following the supplied AMS datasheet's
stability recommendation. Do not substitute an arbitrary ceramic for C3.
No external minimum-load resistor is required for the fixed AMS1117-3.3.
The local datasheet (`../../docs/datasheets/ams1117.pdf`, page 3) specifies
load regulation from 0 to 800 mA for this version; the 10 mA minimum-load
requirement applies to the adjustable AMS1117. R1 is therefore omitted.
C4/C5 provide 10 µF / 100 nF local module decoupling.

Provide adequate copper area connected to the regulator's output/tab. Its
loss is approximately `(5 - 3.3) × I_3V3`: 0.17 W at 100 mA, 0.85 W at 500 mA.
The quoted regulator current rating is not a guaranteed enclosed-board thermal
capacity. CAN-only operation reduces the expected load, but layout and enclosure
thermal validation remain necessary. Reserve at least 500 mA supply capability
for the ESP32 design; budget external 3.3 V peripherals and both 5 V SSR loads
against the shared RECOM supply. During flashing, budget the board and any
attached peripherals against the USB host supply. Mount TMP102 away from the
LDO and heated copper.

## USB flashing

J2 is GCT USB4105-GF-A, a 16-contact USB 2.0 Type-C receptacle. Both D+ contacts
are connected together, as are both D− contacts. CC1 and CC2 each have their
own **5.1 kΩ pull-down** to GND (R4/R5), identifying the board as a USB
peripheral. No USB Power Delivery controller is needed for the intended 5 V
flashing supply. The ESP32's data pull-up is internal; no external D+ pull-up
is fitted.

**USB VBUS connects directly to +5 V**, feeding the existing AMS1117 regulator
and the board's 5 V circuits. USB-C and J1 are alternative power inputs:
**never connect USB-C and external XH power simultaneously.** There is no
power OR-ing, source selection, or reverse-current blocking between them.
The disconnected J1 pin 2 will carry USB VBUS during flashing.

The data pair connects directly through R6/R7, 22 Ω, to GPIO20 D+ / GPIO19 D−.
D1 is a ground-referenced TPD2E2U06 USB ESD device. U3, Q1, R8/R9/R10, and C7
have been removed. C1 is reduced to 4.7 µF to reduce USB plug-in inrush;
C3 remains 22 µF solid tantalum on the regulated 3.3 V rail.

Route the data pair at 90 Ω differential impedance, with short paired traces,
D1 close to J2 and R6/R7 close to U1. The connector shield joins board GND.
This is a USB **device**, not a host port.

To enter ROM flashing mode:

1. Disconnect the external 5 V power cable from J1.
2. Connect J2 to the Mac with a USB-C data cable; USB supplies board power.
3. Hold **BOOT**, tap **RESET**, then release BOOT and flash the firmware.
4. Disconnect USB-C before reconnecting J1 for production.

R3 pulls GPIO0 high for normal boot. R2/C6 form the EN startup delay
(10 kΩ / 1 µF). R23/R24 pull GPIO45/46 low for 3.3 V flash and
download-compatible straps. GPIO3 is unused. No UART service header is
fitted: native USB plus BOOT/RESET covers flashing and recovery. GPIO43/44
are unconnected.

## GPIO allocation and firmware migration

| Function | GPIO | Module pad | Previous sensors firmware |
|---|---:|---:|---|
| Digmesa pulse | 41 | 37 | GPIO44; move to GPIO41 |
| I2C SDA | 5 | 9 | GPIO5, unchanged |
| I2C SCL | 6 | 10 | GPIO6, unchanged |
| CAN TX | 11 | 15 | GPIO7; move to GPIO11 |
| CAN RX | 10 | 14 | GPIO8; move to GPIO10 |
| M5Stack valve SSR | 36 | 32 | GPIO9; move to GPIO36; HIGH = on |
| Boiler SSR driver | 12 | 16 | GPIO3 active-low; move to GPIO12 **active-high** |
| Green status LED D2 | 38 | 34 | New output; HIGH = on |
| USB D− / D+ | 19 / 20 | 23 / 24 | Native USB Serial/JTAG |
| UART TX / RX | 43 / 44 | 39 / 40 | Unconnected; no service header |

CAN signals are assigned to simplify routing to U4: pad 15 (GPIO11) drives
TXD and pad 14 (GPIO10) receives RXD. The ESP32-S3 GPIO matrix supports
this assignment; U4 TXD/RXD and R11 remain on their existing CAN nets.

HEATER_CMD uses pad 16 (GPIO12), FLOW_PULSE uses pad 37 (GPIO41),
and VALVE_CMD uses pad 32 (GPIO36) to simplify routing.
GPIO36 is available as a general-purpose output on the MINI-1U-N8
(8 MB Quad SPI flash, no PSRAM); its alternate SPI functions are unused.
GPIO41 is also the external JTAG MTDI input: this avoids using the MTDO
output pin but still overlaps external pad JTAG. Configure it as a GPIO input
for FLOW_PULSE and do not use external pad JTAG on this pin. Native USB
Serial/JTAG remains on GPIO19/20. Neither new assignment is a boot strap.

D2 is a green 0603 status LED, driven by **GPIO38 (module pad 34)** through
R25 = **1 kΩ**: GPIO38 → R25 → D2 anode (pin 2), cathode (pin 1) → GND.
HIGH turns it on; configure GPIO38 as a GPIO output in firmware. With a
green LED forward voltage of approximately 2.0–2.2 V, nominal current is
approximately 1.1–1.3 mA at 3.3 V; the exact LED part remains to be sourced.
The local Espressif `esp32-s3-mini-1_mini-1u_datasheet_en.pdf` v1.7 lists
GPIO38 / FSPIWP / SUBSPIWP for pad 34 (Table 3-1, p. 11). The alternate SPI
functions are multiplexed peripheral functions, not a reserved internal
flash connection. The MINI-1U-N8 uses in-package 8 MB Quad SPI flash and
has no PSRAM (Table 1-1). GPIO38 is neither a boot strap (GPIO0/3/45/46,
§4), a default UART pin (UART0 TX/RX on GPIO43/44), nor an external JTAG
pin (GPIO39–42). It is available for this LED provided firmware does not
route another peripheral to it. The schematic and review exports include
D2/R25; their PCB placement and routing are to be updated in the PCB editor.

Before using this PCB, change `kGpioCanTx` to `GPIO_NUM_11`, `kGpioCanRx`
to `GPIO_NUM_10`, `kGpioFlow` to `GPIO_NUM_41`, `kGpioHeater` to
`GPIO_NUM_12`, `kGpioValve` to `GPIO_NUM_36`, and `kHeaterActiveLevel` to **1**. Change the heater's configured
internal pull-up to disabled/pull-down as appropriate, and preload LOW before
setting the pin to output. Keep GPIO12 and GPIO36 LOW at startup. Existing firmware for
the XIAO/HW-399 cannot operate this boiler output correctly without this migration.
Firmware files have not been changed by this schematic task.

Add a TMP102 driver at address **0x48** if enclosure temperature telemetry is
wanted. The dimmer uses 0x50 in the current firmware, so this temperature
address is distinct. The new sensor measures local board/
enclosure temperature, not boiler temperature. ALERT is unused and marked NC.

## Cable connections

XH cables are home-built; use **GND, supply, signal** for the three-pin actuator/
flow connections. XH pitch is 2.50 mm. Follow numbered pads and the final PCB
silkscreen rather than assuming a cable view or pin position.

| Connector | Type | Pin 1 | Pin 2 | Pin 3 | Pin 4 |
|---|---|---|---|---|---|
| J1 external power | XH 2 | GND | 5 V in | — | — |
| J3 CAN | XH 2 | CAN_L | CAN_H | — | — |
| J4 dimmer | Grove HY2.0 4 | SCL | SDA | 3V3 | GND |
| J6 Digmesa | XH 3 | GND | 5 V | SIGNAL | — |
| J7 M5Stack valve SSR | XH 3 | GND | 5 V | SIGNAL (3.3 V) | — |
| J8 boiler SSR | XH 2 | GND / SSR − | switched 5 V / SSR + | — | — |

**Grove opening facing up: left-to-right GND, 3V3, SDA, SCL**, as requested.
Numbered pads therefore appear as 4, 3, 2, 1 in that view. This preserves the
existing black/red/white/yellow cables. The selected footprint is
`Connector:NS-Tech_Grove_1x04_P2mm_Vertical`, not a JST PH substitute.
Validate the physical connector's orientation against its keyed opening when
placing it on the board.

This board is intended for the configuration with an **analog XDB401 pressure
sensor connected to the screen module**. It has no XDB401 connector; pressure
measurement is handled by the screen module.

J3 intentionally omits GND. The common power GND conductor must run alongside
the twisted CAN_H/CAN_L pair; both nodes share the same supply.

## Interface circuits

U4 is **TJA1051T/3**, not the non-/3 variant: pin 3 gets 5 V, pin 5 VIO gets
3.3 V. C8/C9 decouple both supplies. Pin 8 S/SLNT is grounded for normal mode;
R11 pulls TXD high during ESP32 reset to keep the bus recessive.
R12, 120 Ω / 0.25 W, is permanently connected across CAN_H/CAN_L.
This board is a bus-end node, with a total of two terminations along the bus.

J4 and TMP102 share the 3.3 V I2C bus. R13/R14 are ordinary **4.7 kΩ, 1%**
pull-ups, always fitted. Each draws about 0.7 mA when its line is low at
3.3 V; any pull-ups in the attached dimmer act in parallel. Start with the
current 100 kHz bus and verify signal timing with the
actual cables and modules. C10 (100 nF) decouples the onboard TMP102; remote I2C modules
provide their own local supply decoupling. C11 is removed.
No 5 V I2C supply or level translation is included.

Verified **7-bit** addresses:

| Device | Address | Evidence |
|---|---|---|
| DimmerLink dimmer | 0x50 | Current firmware `kDimmerAddr`; `docs/dimmerlink-i2c.md`; vendor `tmp/DimmerLink/04_I2C_COMMUNICATION.md` |
| TMP102 | 0x48 | Supplied `tmp102.pdf`, Table 6-4: ADD0/A0 grounded gives binary 1001000; U5 pin 4 is connected to GND in the exported netlist |

These addresses do not conflict. Keep DimmerLink at its current configured
address: it supports changing the address through register 0x30, which the
current firmware intentionally does not write.

Digmesa is supplied by **5 V** at J6 pin 2. Its NPN open-collector signal at
pin 3 joins GPIO41, **R15 = 1 kΩ to 3.3 V**, and **C12 = 10 nF to GND**.
This reproduces `docs/cablage.md`, with a nominal rising-edge RC time constant
of 10 µs; no pull-up to 5 V is present. Disable the GPIO internal pull-up.
Supply-decoupling rule: fit local bypass capacitors at the supply pins of
chips located on this PCB. Do not add supply bypass capacitors solely for
connectors feeding remote chips; the remote board is responsible for
decoupling at its own chips. Accordingly, C13 is removed from J6/J7.
C11 is also removed from the remote I2C connector supply.
Sensor and cable impedance affect the falling edge; verify pulse capture
on the assembled wiring.

J7 supplies the M5Stack valve SSR at 5 V and connects its signal directly to
GPIO36. R16 has been removed. R17 = 10 kΩ remains from SIGNAL to GND to
define the board output as LOW during reset, including when a replacement
SSR lacks an internal pull-down.
The supplied Unit SSR schematic shows an internal 1 kΩ series resistor (R6)
and 4.7 kΩ input pull-down (R7), so the board pull-down is redundant; together
they give about 3.20 kΩ and draw about 1.03 mA at 3.3 V, in addition to the
module transistor base current. The signal is 3.3 V and HIGH means on.

## Boiler SSR driver and off state

The supplied SSR needs a powered 5 V input. Its new cable is **SSR − to J8
pin 1, SSR + to J8 pin 2**. This replaces the old HW-399 sinking arrangement;
J8 is a switched positive output, not the old OUT4 connection.

The current path is **5 V → Q2 P-channel MOSFET → SSR + → SSR − → GND**.
Q2 is AO3401A, source at 5 V, drain at J8 output. Q3 sinks only Q2's gate
current, not the SSR load current. A design load of up to **100 mA** is readily
within the MOSFET capability with its gate pulled near GND; this is the
intended connector load budget, not a built-in current limit. The supplied
`KS53_EN.pdf` specifies a maximum input current of **25 mA** for the family,
leaving ample margin, plus the local 5 mA output pull-down load. Confirm its actual input current and activation voltage
on the bench before connecting the heater's AC load.

GPIO12 turns on Q3, a 2N7002, whose drain connects directly to Q2's gate.
This pulls the P-channel MOSFET gate toward GND, switching the 5 V output on.
R21 supplies approximately 0.5 mA while Q3 is on. GPIO12 drives only the
MOSFET gate; there is no optocoupler LED current. U6 (TLP281), its LED resistor
R18, and series gate resistor R20 have been removed to reduce component and
assembly cost. Existing references are retained for the remaining components.

R19 pulls Q3's gate low, R21 pulls Q2's gate up to its source, and R22 pulls
the output to ground. With a floating GPIO or absent 3.3 V rail, the commanded
state is **off**, with nominal output 0 V. R22 is 1 kΩ, drawing 5 mA / 25 mW
when on; any semiconductor leakage can produce a small residual off voltage.
Verify startup, reset, flashing, 3.3 V power loss, and loaded output switching
on the assembled board. Hardware does not shut down a GPIO deliberately held
high by running firmware; retain firmware timeouts and the existing machine's
thermal protection.

The driver shares production supply GND with the ESP32. This board does not
provide galvanic isolation between logic and the boiler command output; the
SSR provides its own separation from the AC load. There are no mains nets
on this PCB. Removing the optocoupler does not remove an isolation barrier
that existed in the previous shared-GND circuit.

## BOM, libraries, and checks

`sensors_bom.csv` is a preliminary 46-component BOM with assigned footprints.
J1 and J3 now use vertical JST B2B-XH-A headers, with footprint
`Connector_JST:JST_XH_B2B-XH-A_1x02_P2.50mm_Vertical` in both saved schematics
and the PCB, and the matching vertical 3D model on the PCB. The cable pinouts
above are unchanged. Allow clearance above the board for the mating housings
and cable exit during layout and enclosure design.
The preliminary BOM still lists the former right-angle S2B-XH-A(LF)(SN)
footprint, MPN, and LCSC/JLCPCB code C157931 for J1/J3; those entries must be
updated before ordering. A supplier code for the vertical headers has not
been established. Other LCSC fields are blank: supplier availability and exact
sourcing have not been established. It is not an order-ready JLCPCB BOM. Generic resistor/
capacitor entries specify required value, package, and tolerance/type rather
than pretending to select a stocked MPN. For C3 use solid tantalum, not a generic
ceramic. R12 uses 1206 for its 0.25 W rating.

All symbols and footprints use standard KiCad 10 libraries, except the
project-local J2 variant described below. The unused
TLP281 custom symbol/footprint and local library tables have been removed.

The saved schematics are editable in KiCad. `generate_schematic.py` is the
reproducible starting-point generator; it overwrites both schematic sheets,
and the preliminary BOM, so do not run it after manual edits
without reconciling those edits. It uses standard-library Python only and can
be run with `uv run generate_schematic.py`.
Its J1/J3 overrides still select the former horizontal connectors and C157931;
reconcile them with the saved vertical selection before regenerating. The
exported PDF and netlist also need regeneration to reflect the footprint change.

Export/check commands, from `pcb/`:

```sh
/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli sch erc \
  --severity-all --exit-code-violations -o sensors/sensors_erc.rpt sensors/sensors.kicad_sch
/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli sch export pdf \
  -o sensors/sensors.pdf sensors/sensors.kicad_sch
/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli sch export netlist \
  --format kicadxml -o sensors/sensors.net.xml sensors/sensors.kicad_sch
```

ERC is run without adding exclusions or suppressing violations. The project's
standard ignored checks are recorded in the ERC report. Netlist validation
checks the GPIO/connector mapping, both supply rails, USB VBUS connection to the common 5 V input rail,
flowmeter filter, CAN supplies/mode, temperature address, driver topology,
and symbol-to-footprint pad correspondence. The two-page PDF is rendered and
visually checked. These checks establish schematic connectivity; they do not
replace PCB DRC, thermal validation, signal testing, or loaded SSR testing.

## References

- Local wiring: `../../docs/cablage.md`; current GPIOs: `../../firmware/sensors/main/main.cpp`.
- [ESP32-S3-MINI-1 / MINI-1U datasheet](https://www.espressif.com/sites/default/files/documentation/esp32-s3-mini-1_mini-1u_datasheet_en.pdf), also supplied in `../../docs/datasheets/`.
- [Espressif hardware design guidelines](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32s3/schematic-checklist.html).
- Supplied `../../docs/datasheets/TJA1051.pdf`, `ams1117.pdf`, `tmp102.pdf`, and `KS53_EN.pdf`.
- [AO3401A datasheet](https://www.aosmd.com/sites/default/files/res/datasheets/AO3401A.pdf).
- [Grove pin conventions](https://wiki.seeedstudio.com/Grove_System/).

## USB-C and mechanical DRC rules

J2 uses the project-local `Sensors_Local` USB4105 footprint variant. Its
A1/A12/B1/B12 ground pads retain the stock 0.6 x 1.15 mm size and centres,
but their corner-radius ratio is increased from 0.25 to 0.35 (0.15 to
0.21 mm radius). This increases the nominal copper-to-locating-hole gap
from 0.1944 mm to 0.2083 mm; locating holes and other pads are unchanged.
The standard geometry was checked against the
[GCT USB4105 drawing](https://gct.co/files/drawings/usb4105.pdf).

`sensors.kicad_dru` applies a 0.20 mm hole-clearance minimum only between
J2's locating holes and these four ground pads. The board-wide 0.25 mm
minimum remains in effect elsewhere. JLCPCB's
[published capabilities](https://jlcpcb.com/capabilities/Capabilities)
list a 0.20 mm NPTH-to-copper minimum. The rounded-pad variant is a local
design adjustment, not a footprint published by GCT.

H1-H4 identify the four M2 mounting holes. They are marked PCB-only and
excluded from the BOM and position files; their reference labels are hidden.
No routing, component positions, hole sizes or schematic nets were changed
for these DRC fixes.

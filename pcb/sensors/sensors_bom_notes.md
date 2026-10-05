# Sensors JLCPCB BOM

Checked against the saved PCB and both schematic sheets on **2026-10-05**.

**45 fitted components: 29 Basic, 16 Extended.** The upload BOM contains all 45 components in 26 part groups. Basic/Extended classifications and stock were read directly from JLCPCB catalog data; the dated snapshot is in `sensors_sourcing.json`. Stock and classifications can change.

- `sensors_bom.csv`: full per-component source BOM, with PCB and schematic references, exact MPN, LCSC code, class, assembly method and notes.
- `sensors_jlcpcb_bom.csv`: grouped four-column JLCPCB upload BOM using the actual PCB references.
- `sensors_manual_bom.csv`: header-only; no parts are currently selected for manual assembly.
- `sensors_bom_validation.json`: component counts, reference mapping and source hashes.

Regenerate from the saved designs with `uv run sensors/generate_bom.py` from `pcb/`, or `uv run generate_bom.py` from `sensors/`. This reads the saved designs, validates component identity/footprints/DNP and writes only BOM artifacts. Edit `sensors_sourcing.json` when changing part selections.

## Part selections

| PCB references | Part | JLCPCB | Class | Assembly | Catalog stock* |
|---|---|---|---|---|---:|
| 5V, BOILER, CAN | B2B-XH-A(LF)(SN) | [C158012](https://jlcpcb.com/partdetail/x/C158012) | Extended | THT | 299,933 |
| BOOT, RESET | TS-1088-AR02016 | [C720477](https://jlcpcb.com/partdetail/x/C720477) | Basic | SMT | 777,290 |
| C1 | CL21B475KOFNNNE | [C107365](https://jlcpcb.com/partdetail/x/C107365) | Extended | SMT | 46,921 |
| C2, C5, C8, C9, C10 | CC0603KRX7R9BB104 | [C14663](https://jlcpcb.com/partdetail/x/C14663) | Basic | SMT | 57,209,141 |
| C3 | TAJB226K010RNJ | [C7198](https://jlcpcb.com/partdetail/x/C7198) | Extended | SMT | 39,323 |
| C4 | CL21B106KOQNNNE | [C95841](https://jlcpcb.com/partdetail/x/C95841) | Extended | SMT | 394,770 |
| C6 | CL10B105KA8NNNC | [C29936](https://jlcpcb.com/partdetail/x/C29936) | Extended | SMT | 268,058 |
| C12 | 0603B103K500NT | [C57112](https://jlcpcb.com/partdetail/x/C57112) | Basic | SMT | 6,657,115 |
| D1 | TPD2E2U06DCKR | [C1855726](https://jlcpcb.com/partdetail/x/C1855726) | Extended | SMT | 11 |
| FLOW, VALVE | B3B-XH-A(LF)(SN) | [C144394](https://jlcpcb.com/partdetail/x/C144394) | Extended | THT | 174,813 |
| I2C | HY-4A | [C722737](https://jlcpcb.com/partdetail/x/C722737) | Extended | THT | 1,106 |
| J2 | USB4105-GF-A-060 | [C3025063](https://jlcpcb.com/partdetail/x/C3025063) | Extended | SMT | 1,287 |
| Q2 | AO3401A | [C15127](https://jlcpcb.com/partdetail/x/C15127) | Basic | SMT | 815,848 |
| Q3 | 2N7002 | [C8545](https://jlcpcb.com/partdetail/x/C8545) | Basic | SMT | 1,601,156 |
| R2, R3, R11, R17, R21, R23, R24 | 0603WAF1002T5E | [C25804](https://jlcpcb.com/partdetail/x/C25804) | Basic | SMT | 29,606,163 |
| R4, R5 | 0603WAF5101T5E | [C23186](https://jlcpcb.com/partdetail/x/C23186) | Basic | SMT | 25,206,931 |
| R6, R7 | 0603WAF220JT5E | [C23345](https://jlcpcb.com/partdetail/x/C23345) | Basic | SMT | 8,966,004 |
| R12 | 1206W4F1200T5E | [C17909](https://jlcpcb.com/partdetail/x/C17909) | Basic | SMT | 529,895 |
| R13, R14 | 0603WAF4701T5E | [C23162](https://jlcpcb.com/partdetail/x/C23162) | Basic | SMT | 22,251,089 |
| R15, R22, R25 | 0603WAF1001T5E | [C21190](https://jlcpcb.com/partdetail/x/C21190) | Basic | SMT | 21,294,251 |
| R19 | 0603WAF1003T5E | [C25803](https://jlcpcb.com/partdetail/x/C25803) | Basic | SMT | 22,034,608 |
| D2 | KT-0603YG | [C2289](https://jlcpcb.com/partdetail/x/C2289) | Extended | SMT | 29,232 |
| U1 | ESP32-S3-MINI-1U-N8 | [C2980299](https://jlcpcb.com/partdetail/x/C2980299) | Extended | SMT | 1,402 |
| U2 | AMS1117-3.3 | [C6186](https://jlcpcb.com/partdetail/x/C6186) | Basic | SMT | 1,056,675 |
| U4 | TJA1051T/3/1J | [C38695](https://jlcpcb.com/partdetail/x/C38695) | Extended | SMT | 287,313 |
| U5 | TMP102AIDRLR | [C99269](https://jlcpcb.com/partdetail/x/C99269) | Extended | SMT | 41,187 |

*Catalog `overseasStockCount`, not a reservation or a guarantee of quantity available to your order. D1 had only 11 units and `canPresaleNumber=0`; resolve its availability before submitting an assembly order. Other assigned parts had much larger stock.

## Decisions and remaining assembly details

**Keep X7R by default.** C1/C4/C6 retain the dielectric specified in the original BOM. There are footprint-compatible Basic X5R alternatives below, but they reduce the dielectric temperature rating from 125°C to 85°C. They are documented options, not selected substitutions. All MLCCs lose capacitance under DC bias; their voltage rating alone does not guarantee effective capacitance.

| Reference | Selected X7R (Extended) | Optional X5R (Basic) |
|---|---|---|
| C1 | C107365, CL21B475KOFNNNE, 4.7µF 16V 0805 | [C1779](https://jlcpcb.com/partdetail/x/C1779), CL21A475KAQNNNE, 4.7µF 25V 0805 |
| C4 | C95841, CL21B106KOQNNNE, 10µF 16V 0805 | [C15850](https://jlcpcb.com/partdetail/x/C15850), CL21A106KAYNNNE, 10µF 25V 0805 |
| C6 | C29936, CL10B105KA8NNNC, 1µF 25V 0603 | [C15849](https://jlcpcb.com/partdetail/x/C15849), CL10A105KB8NNNC, 1µF 50V 0603 |

**C3 stays solid tantalum.** TAJB226K010RNJ, 22µF/10V, case B 3528-21, fits the Kemet-B land pattern. The local AMS1117 datasheet, page 4, recommends 22µF solid tantalum for stability. Pin 1 is positive; do not replace this part with an MLCC or polymer capacitor solely to reduce loading fees.

**BOOT/RESET use the current footprint.** C720477 TS-1088-AR02016 is Basic in the current JLCPCB catalog, and is the exact part named by KiCad’s `SW_SPST_TS-1088-xR020` footprint. The old TL3342F160QG entry is a different, larger switch.

**STATUS uses a low-forward-voltage yellow-green LED.** C2289 KT-0603YG is Extended, 0603, nominal Vf 2.0–2.2V. This matches the design’s approximately 1.1–1.3mA indication with R25=1k and 3.3V. Basic 0805 green LEDs do not fit; the checked 0603 emerald-green options were Extended and have less voltage headroom. Pin 1 is cathode/GND.

**CAN keeps NXP’s /3 variant.** C38695 is TJA1051T/3/1J, the current ordering code for the SO8 device with pin 5 VIO. The local TJA1051 datasheet confirms the /3 pinout. Do not substitute TJA1051T without /3 or an unverified clone.

**U1 keeps the exact ESP32-S3-MINI-1U-N8.** C2980299 is the Espressif module. JLCPCB’s package label says VFQFN-56-EP; the local Espressif datasheet specifies the actual 15.4×15.4mm module and its land pattern. Match by exact MPN and manufacturer drawing. Confirm JLCPCB assembly-service eligibility and polarity/rotation for the module during quote review.

**USB-C uses GCT USB4105-GF-A-060.** C3025063 is a listed variant of the assigned footprint; it has shorter 0.60mm shell stakes than the unsuffixed part. [GCT drawing](https://gct.co/files/drawings/usb4105.pdf) provides the shared PCB land pattern. Confirm shell engagement for the board thickness. D1 is the exact TI 3-pin SC-70 DCK part, with I/O on pins 1/2 and GND on 3 ([TI datasheet](https://www.ti.com/lit/ds/symlink/tpd2e2u06.pdf)); the 5-pin DRL version is incompatible.

**I2C/Grove uses CAX HY-4A / C722737.** The user selected this Extended THT part after confirming its model displays in JLCPCB. The project-local `Sensors_Local:CONN-TH_HY-4A` footprint is now assigned in schematic and PCB, with the supplier 3D model, a KiCad-compatible pad order and 1.0mm drills. It retains the 4-pin 2mm pin layout. Numbered supplier pads match the KiCad pad order with a -90° CPL rotation offset. Position uses the numbered pin-row midpoint. The linked supplier PDF contains unrelated PH-series information and is not used as mechanical evidence.

**Six connectors need through-hole assembly.** 5V/CAN/BOILER share C158012, the white JST B2B-XH-A(LF)(SN). FLOW/VALVE share C144394, JST B3B-XH-A(LF)(SN). The I2C/Grove connector adds C722737. For full outsourced assembly select a service supporting these THT parts; otherwise fit them manually and remove those references from the assembly upload.

## Reference reconciliation and scope

The old BOM still included J5, used obsolete horizontal J1/J3 connectors, and listed the previous switch footprint. The new BOM follows the saved PCB. UUIDs associate each footprint with its schematic symbol, so connector names are not guessed by value.

| Schematic | PCB/upload |
|---|---|
| J1 | 5V |
| J8 | BOILER |
| SW1 | BOOT |
| J3 | CAN |
| J6 | FLOW |
| J4 | I2C |
| SW2 | RESET |
| J7 | VALVE |

Both schematic sheets now contain hidden `LCSC` fields and updated `MPN` fields matching the sourced JSON: all 45 LCSC IDs are populated, including J4’s CAX Grove connector. Existing references and schematic connectivity are unchanged. Reload the schematics from disk in any open editor before using Update PCB from Schematic with “Update footprint fields from symbols”. Review the reference/value differences listed above when applying the update. PCB fields and geometry were not changed by this schematic update.

Manufacturing outputs are now in `manufacturing/`: `sensors_jlcpcb.zip` contains fabrication, BOM and placement files; `sensors_gerbers.zip` contains fabrication files only. Physical DRC passed with zero violations and zero unconnected items. The project-local `customize-manufacturing.py` accepts only the eight intentional reference-label pairs listed above and the four specific mechanical mounting holes. It verifies renamed footprints against their schematic UUIDs, values, footprints and sourcing fields, and independently compares schematic/PCB connected net membership on every export. Other parity findings still stop exports. Placement corrections in `customize-manufacturing.py` now use numbered pin-row midpoints for XH/Grove connectors. Relative rotation offsets: U1=0°, U2=180°, U4=270°, U5=180°, C3/Q2/Q3=180° and D1=-90° (user-confirmed), and C158012 connectors 5V/CAN/BOILER=180°. Supplier CAD numbered pads were compared against KiCad for all U parts and JST connectors; see `sensors_placement_validation.json`. This is evidence for the selected supplier CAD orientation, not independent confirmation of JLCPCB’s private assembly models. C722737 Grove supplier CAD was retrieved: its midpoint and -90° rotation correction match the four numbered pads. Check Grove alignment and all corrected placements in JLCPCB’s preview. These changes affect only manufacturing placement files; PCB geometry is unchanged.

`generate_schematic.py` is an older schematic seed and still contains earlier design choices; running it would overwrite edited schematics and the sourced BOM. Use `generate_bom.py` for this saved layout.

C722737 PCB synchronization is complete. Source/upload BOMs, placement CSV and manufacturing archives are regenerated from the saved board. Physical DRC passed with zero violations and zero unconnected items; the 20 known reference-label/mounting-hole parity findings were accepted only after component identity and independent connectivity checks. The current archive selects C722737 / HY-4A and the local CONN-TH_HY-4A footprint.

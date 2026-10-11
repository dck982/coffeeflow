# Sensors JLCPCB BOM

Checked against the saved PCB and both schematic sheets on **2026-10-11**.

**34 fitted components: 21 Basic, 13 Extended, in 18 part groups.** Classes and stock come from the JLCPCB catalog; the dated snapshot is in `sensors_sourcing.json`. Stock and classes can change.

- `sensors_bom.csv`: per-component source BOM with schematic references, MPN, LCSC code, class, assembly method and notes.
- `sensors_jlcpcb_bom.csv`: grouped four-column JLCPCB upload BOM.
- `sensors_manual_bom.csv`: header only; JLCPCB assembles every part on the board.
- `sensors_bom_validation.json`: counts and source hashes.

Regenerate with `uv run sensors/generate_bom.py` from `pcb/`. It reads the saved designs, checks component identity, footprints and DNP, and writes only these BOM files. Edit `sensors_sourcing.json` to change a part. Manufacturing files come from `uv run generate_manufacturing.py sensors`, into `manufacturing/`.

## Part selections

| References | Part | JLCPCB | Class | Assembly | Catalog stock* |
|---|---|---|---|---|---:|
| C8, C9, C10, C13, C15 | CC0603KRX7R9BB104 | [C14663](https://jlcpcb.com/partdetail/x/C14663) | Basic | SMT | 56,342,854 |
| C12 | 0603B103K500NT | [C57112](https://jlcpcb.com/partdetail/x/C57112) | Basic | SMT | 6,125,182 |
| C14 | CL10A106KP8NNNC | [C19702](https://jlcpcb.com/partdetail/x/C19702) | Basic | SMT | 10,311,061 |
| D2 | KT-0805G | [C2297](https://jlcpcb.com/partdetail/x/C2297) | Basic | SMT | 2,899,999 |
| J1, J3, J8 | B2B-XH-A(LF)(SN) | [C158012](https://jlcpcb.com/partdetail/x/C158012) | Extended | THT | 270,011 |
| J4, J5 | HY-4A | [C722737](https://jlcpcb.com/partdetail/x/C722737) | Extended | THT | 1,106 |
| J6, J7, J10 | B3B-XH-A(LF)(SN) | [C144394](https://jlcpcb.com/partdetail/x/C144394) | Extended | THT | 165,403 |
| J11, J12 | HX PM2.54-1x7P ZC-Y | [C22438157](https://jlcpcb.com/partdetail/x/C22438157) | Extended | THT | 25,153 |
| Q2 | AO3401A | [C15127](https://jlcpcb.com/partdetail/x/C15127) | Basic | SMT | 709,332 |
| Q3 | 2N7002 | [C8545](https://jlcpcb.com/partdetail/x/C8545) | Basic | SMT | 1,300,293 |
| R11, R17, R21 | 0603WAF1002T5E | [C25804](https://jlcpcb.com/partdetail/x/C25804) | Basic | SMT | 33,703,724 |
| R12 | 1206W4F1200T5E | [C17909](https://jlcpcb.com/partdetail/x/C17909) | Basic | SMT | 494,992 |
| R13, R14 | 0603WAF4701T5E | [C23162](https://jlcpcb.com/partdetail/x/C23162) | Basic | SMT | 20,905,184 |
| R15, R22, R25, R26 | 0603WAF1001T5E | [C21190](https://jlcpcb.com/partdetail/x/C21190) | Basic | SMT | 26,935,418 |
| R19 | 0603WAF1003T5E | [C25803](https://jlcpcb.com/partdetail/x/C25803) | Basic | SMT | 20,955,978 |
| U4 | TJA1051T/3/1J | [C38695](https://jlcpcb.com/partdetail/x/C38695) | Extended | SMT | 289,172 |
| U5 | TMP102AIDRLR | [C99269](https://jlcpcb.com/partdetail/x/C99269) | Extended | SMT | 39,076 |
| U6 | ADS1115IDGSR | [C37593](https://jlcpcb.com/partdetail/x/C37593) | Extended | SMT | 26,684 |

*Catalog `overseasStockCount`, not a reservation. J4/J5 (C722737) has the smallest stock.

## Decisions

**The XIAO plugs into two sockets soldered by JLCPCB.** U1, the XIAO ESP32-S3 from stock, stays in the schematic for its pinout but is not placed on the board (`on_board no`, `in_bom no`). J11 (XIAO pins 1–7) and J12 (pins 8–14) are hanxia HX PM2.54-1x7P ZC-Y female sockets, 2.54 mm, 8.5 mm high, C22438157, on the project footprint `Sensors_Local:XIAO_Socket_1x07_P2.54mm_Vertical`: KiCad's `PinSocket_1x07_P2.54mm_Vertical` with a courtyard 0.25 mm around the body and no outline, since the XIAO outline is board silkscreen. Deselect J11/J12 at order time to solder the sockets by hand.

**STATUS is an emerald-green 0805.** D2 is C2297 KT-0805G, Basic, with R25 = 1k from GPIO2. Pin 1 is the cathode.

**CAN keeps NXP's /3 variant.** C38695 is TJA1051T/3/1J, the SO8 device with pin 5 VIO. Do not substitute TJA1051T without /3 or an unverified clone.

**Both Grove connectors use CAX HY-4A / C722737** on the project footprint `Sensors_Local:CONN-TH_HY-4A`, with the supplier 3D model, KiCad pad order and 1.0 mm drills. J4 and J5 are two sockets on the same I2C bus.

**Ten through-hole parts.** J1/J3/J8 use C158012 (JST B2B-XH-A), J6/J7/J10 C144394 (JST B3B-XH-A), J4/J5 C722737 and J11/J12 C22438157. Select through-hole assembly for them, or fit them by hand and deselect them at order time.

## Placement

`customize-manufacturing.py` places the XH, Grove and socket connectors at their pin-row midpoint and applies these rotation corrections: U4 +270°, U5 +180°, U6 +270°, Q2/Q3 +180°, J1/J3/J8 +180°, J4/J5 −90°, J11/J12 +270°. D2 has none. The JLCPCB preview of 2026-10-11 confirmed every placement. Matching the numbered pads of the LCSC footprints to KiCad's gave the U6 and J11/J12 corrections right, but suggested +180° for D2, which the preview showed reversed: for an LED, check the polarity in the preview rather than trusting the pad numbers.

The PCB keeps its schematic references, so there are no reference aliases and the schematic parity check passes without exceptions.

`generate_schematic.py` is an older schematic seed; running it would overwrite the edited schematics and the sourced BOM.

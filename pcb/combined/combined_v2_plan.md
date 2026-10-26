# Combined v2 — task list

Gear pump only (MPB003, 24 V BLDC). The vibration pump setup stays on
sensors + RBDimmer + M5Stack SSR. Pump interface: `docs/pompe-engrenage.md`,
"Interface électrique". Delete this file once v2 is done.

## Schematic

- [ ] Remove the pump AC channel: U2, Q3, R8–R11, D2 (input stage), Q4,
      R12, R27, R13, C3, J4, net L_PUMP.
- [ ] Remove zero-cross detection: R14, R15, D3, U3, R16, net ZC.
- [ ] Keep the valve channel (U1, Q1, Q2, R1–R7, C2, D1, J2, J3, F1).
- [ ] Add J_24V: HT396R-3.96-2P (C577799), 1 GND, 2 +24V.
- [ ] Add the buck (LMR33630ADDAR, C841384): 2 × 10 µF 50 V 1210 + 100 nF
      on VIN, 47 µF 50 V electrolytic, 100 nF BOOT, 1 µF VCC, 10 µH
      FXL0630-100-M (C167223), 3 × 22 µF 16 V 1206 out, FB 100 k / 24.9 k
      (5.02 V), EN 100 k / 7.15 k from VIN (UVLO ≈ 18 V), optional 5 V LED.
- [ ] J1 (5V POWER, XH 2P) becomes the 5 V output to the screen.
- [ ] Check the XIAO 5 V pin against USB back-feed into the buck output
      (bench: USB without 24 V); add a diode if needed.
- [ ] Add J_PUMP: XH 4P (B4B-XH-A, C144395), 1 GND_P (green), 2 SPEED
      (brown), 3 TACHO (blue), 4 +5V_P (yellow). GND_P and +5V_P are
      isolated nets, never tied to board GND or 5V.
- [ ] Speed opto (EL817C, C2912100): GPIO1 → 330 Ω → LED → GND, 100 k
      GPIO1 to GND. Collector to +5V_P, emitter → 10 k to GND_P, then
      2.2 k / 1.5 µF / 2.2 k / 1.5 µF to SPEED (caps to GND_P).
- [ ] Tacho opto (EL817C): +5V_P → 1.8 k → LED → TACHO. Collector to GPIO3
      with 4.7 k pull-up to 3V3, emitter to GND.
- [ ] Rename nets: DIM → PUMP_SPEED (GPIO1, D0), ZC → PUMP_TACHO (GPIO3, D2).
- [ ] Update BOM (`combined_bom.csv`) and symbol fields (Value, Footprint,
      LCSC) in sync.
- [ ] Regenerate `combined.pdf`.

## Rules and docs

- [ ] `combined.kicad_dru`: drop the L_PUMP / Q4 gate rules; keep valve and
      mains rules. Clearance between the GND_P / +5V_P side and board GND:
      functional only, default rules.
- [ ] `combined_design.md`: drop "Mains side: pump channel", "Zero-cross
      detection", "Pump firing in firmware", the Q4 dissipation; add the
      buck, the pump interface and the new GPIO table.
- [ ] `customize-manufacturing.py`: check references still exist.

## Board (David)

- [ ] Update PCB from schematic, place and route.
- [ ] Buck: tight VIN/GND loop, thermal pad copper and vias.
- [ ] Mounting, outline and enclosure (`boitier_dc`).
- [ ] DRC, then `uv run ../generate_manufacturing.py .`

## Firmware

- [ ] LEDC PWM on GPIO1 at 1 kHz, 16 bits; 0 at boot.
- [ ] PCNT on GPIO3 (inverted signal), with glitch filter.
- [ ] Fault detection and re-arm (setpoint 0 for ≥ 10 ms).

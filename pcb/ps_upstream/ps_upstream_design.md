# CoffeeFlow power supply upstream board

The smallest board that can sit in front of the 24 V, 96 W supply planned
for the MPB003 gear pump (`../../docs/pompe-engrenage.md`, "Alimentation").
The supply has no inrush limiter of its own; this board adds one on L and
nothing else. Its own T2A fuse stays the protection.

`generate_design.py` writes the schematic and `ps_upstream_bom.csv`.
`generate_board.py` placed and routed the first board; it refuses to
overwrite a board that already has tracks unless given `--force`. Once the
board is edited by hand, the board is the source. `ps_upstream.kicad_dru`
holds the clearance rules, `ps_upstream.pdf` is a generated review copy.

## Circuit

| Ref | Part | LCSC | Role |
| --- | --- | --- | --- |
| J1 | Kefa HT396R-3.96-3P, right angle | C577800 | mains in: 1 L, 3 N |
| RT1 | TDK B57236S0100M000 (S236, 10 Ω) | C125458 | inrush limiter on L |
| J2 | Boom VH-3A, vertical, VH-compatible | C20547 | to the supply: 1 L, 3 N |
| H1, H2 | M3 holes | — | nylon screws only |

J2 is the same VH family as the supply's input, so the output is the end
with the same connector. The cable between them is a VH housing at both
ends, with pin 2 left empty as on the supply.

RT1 limits the peak to 325 V / 10 Ω ≈ 33 A while the supply's 81 µF of
input capacitance charges. The choice of RT1 against three other NTCs is in
the pump document.

## Pin 2 slot

Both connectors are three-way with L and N on the outer pins, 7.92 mm
apart. Pin 2 is still on the part. Its footprint pad is replaced by a
non-plated slot, 1.7 × 4.6 mm on J1 and 1.8 × 4.6 mm on J2. The pin drops
into it unsoldered. There is no copper at pin 2, so no floating island sits
between L and N. The slot cuts the surface path between the pins, and the
pads come 1.86 mm (J1) and 1.66 mm (J2) from it. The symbol
`PS_Upstream:Conn_L_N_Pin2Slot` has pins 1 and 3 only, which keeps the
schematic parity exact.

JLCPCB's review sees only the copper, silkscreen and drill files, not the
fabrication layer. A "NO PIN" silkscreen note under each connector body,
beside its slot, tells the reviewer that the missing plated hole is
intended. The top-left corner carries the rating, "230VAC 0.5A".

The footprints are the LCSC ones of each part (via easyeda2kicad), with the
origin and orientation unchanged. Only pin 2 changed. The J1 outline was
redrawn from the Kefa drawing (`../../docs/datasheets/C577800.pdf`): the
mating face is 9.6 mm in front of the pin axis and the body 0.5 mm behind it,
where the LCSC outline drew 11.1 mm and 1.25 mm. Its courtyard also covers
the pin 2 slot. JLCPCB places all three parts without correction.

## Current and track width

- **Steady current**: about 0.35 A RMS at the 35 W planned peak (pump at
  9–11 bar), 0.04 A at the 2–3 W idle. The tracks are sized for the T2A
  fuse of the supply: 2 A on one 1 oz layer needs about 0.8 mm (IPC-2152,
  10 K rise). They are 2.0 mm wide, and 1.5 mm is the DRC floor.
- **Inrush current**: no need to size for it. The peak is a pulse that
  decays with τ = R × C:

  | Case | Peak | τ | I²t |
  | --- | ---: | ---: | ---: |
  | RT1 cold, 10 Ω (nearly every power-on) | 33 A | 0.81 ms | 0.43 A²s |
  | RT1 hot, about 3 Ω (restart after minutes of continuous pumping) | 108 A | 0.24 ms | 1.4 A²s |

  A 2.0 mm × 35 µm track melts at about 400 A²s (Onderdonk, from 25 °C),
  about 300 times more. A wider track would not change anything.
- **Upstream of RT1**: the L_IN track is sized like the others. A short
  on this board itself is cleared only by the house breaker, as for L_IN
  upstream of F1 on the combined board. No track width protects against
  that.

## Isolation

Same assumptions as the combined board: pollution degree 2,
overvoltage category II, FR4 in material group IIIa, IEC 60335-1. Every net
here is at mains potential.

- **L to N and L_IN to N, functional insulation**: 2.0 mm of creepage
  (Table 18), plus 0.2 mm for etching tolerance: 2.2 mm in the DRC. The
  tightest spot of the layout is about 2.24 mm. L_IN and L differ only by
  the RT1 drop, a few volts RMS, but the layout keeps 2.2 mm there too.
- **Copper to board edge: 3.0 mm.** The edge may rest against the supply's
  aluminium bracket or the enclosure. Neither is tied to L or N, so this is
  basic insulation: 2.5 mm of creepage (Table 17), plus 0.5 mm for the
  outline tolerance.
- **Copper to holes and slots: 1.0 mm** (hole clearance, and the edge
  clearance KiCad also applies to non-plated holes), for a sound web of
  FR4.
- In a steamy spot (pollution degree 3), the 2.0 mm would rise to 3.2 mm:
  keep the board in the dry part of the enclosure, like the supply.

## Layout

The board measures 38.6 × 19.0 mm, under the 43.8 mm width of the supply.
x runs from J1 to J2, and the L–N axis runs through the two slots and the
two holes.

- J1 sits on the left edge, its opening flush with it, so the plug comes in
  from the side.
- RT1 stands across the L track. Its leads are 5 mm apart in y, so the
  disc takes 6 mm of length instead of 11.5 mm. L_IN reaches the lead
  nearest the axis, and L leaves the outer lead along the top edge.
- H1 and H2 sit on the axis, before and after RT1. Each courtyard is 6.2 mm
  across, for a nylon pan head of up to 6.0 mm without a washer. The tracks
  come 1.36 mm from the holes, and the head covers part of the tracks'
  solder mask. **A metal screw is not allowed**: its head would be about
  0.2 mm from L and N. The silkscreen says so.
- J2 is at the far end, with its latch ramp towards the board end.
- All copper is on F.Cu; B.Cu is empty.

## Open points

- The position of the board against the supply, and the cable length from
  J2 to the supply input, depend on the new enclosure.
- The 16 mm height of RT1, on kinked leads, sets the clearance needed above
  the board.

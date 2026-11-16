# Profitec GO drip tray with a scale

**Status (2026-11-16):** the firmware still reads the weight from the Acaia Lunar over BLE. The scale parts below are designed but not printed yet, and nothing in the firmware uses the load cell.

This document covers two revisions of the Profitec GO drip tray assembly. Both revisions share the same metal lower plate. Only the tray and grille differ.

Version 1 (all metal) is effectively a failed design, as Profitec acknowledged by replacing it: it rattles when the machine vibrates. Version 2 (molded plastic) is the improvement and the example to follow for a new drip tray. Version 1 is still useful because it is the unit available to measure. Its dimensions define the machine interfaces that any tray must fit: the base plate, the side walls and front lip, the mouth and the hole placement. They also give a reference volume.

The next phase adds a scale to the drip tray. Its design brief is in [Phase 2: drip tray scale](#phase-2-drip-tray-scale-design-brief).

## Shared lower plate

A 2 mm folded metal plate with a perforated floor (a centre hole and two rings of holes). It has three raised borders:

- **Side walls (left and right):** tall. The tray hangs from them by its shoulders.
- **Front lip (user side):** lower than the side walls but taller than the gap under the tray. The tray is tilted to get over it, and once the tray drops into place the lip locks it, so it can't slide out without being lifted.
- **Rear lip (machine side):** the lowest border. Its only role is to carry the screws that fix the plate to the machine.

## Version 1: full metal tray

- **Build:** 1 mm stainless steel sheet for the tray, bent and soldered (or similar). No CNC machining.
- **Shoulders:** they run along the left, right and front edges, but not at the mouth end. The side shoulders rest on top of the side walls, holding the tray's underside 15.5 mm above the plate.
- **Insertion:** the tray goes in at an angle, rear end first into the mouth, over the front lip. It then drops, and its front wall sits right behind the front lip, which locks it in place.
- **Stop:** the tray's rear end is stopped and held by plastic bumpers in the mouth (item 15 on the version 1 exploded diagram).
- **Centring:** the rear end (23 mm long) fits into the mouth, beneath the purge hole of the three-way solenoid valve, which is centred on the machine width. The 178 mm mouth leaves the 174 mm tray body 2 mm of play per side. The shoulder returns come within about 1 mm of each side wall's outer face (2 mm in total), so they limit side play slightly more tightly.
- **Grille:** a metal grille with arrow marks and a round hole at the mouth end. It has half-circle lift cut-outs on its sides. It stands on the tray floor on its bent-down sides, and its top sits 0.5 mm above the shoulders.

On Profitec's version 1 exploded diagram, the grille is item 16, the tray item 17, the base plate item 18 and the plastic bumpers that stop the tray in the mouth item 15.

## Version 2: molded plastic tray

- **Build:** a black molded plastic tray.
- **Fit:** it slides over the same two side walls. The shape ends against the machine face with a deeper pocket that extends into the mouth, so it sits lower and closer to the metal base than version 1.
- **Anti-splash ball:** a molded ball sits directly beneath the release hole. It disperses the hot water coming down under pressure so it does not spray out around the tray.
- **Grille:** a simpler grille that matches a small ridge moulded into the plastic tray. It has a single square hole at the south-east corner, used to slide it out.

On Profitec's version 2 exploded diagram, the lower metal plate is item 7, the plastic tray item 15 and the grille item 16.

## Version 1 dimensions

Coordinates: X runs across the machine width (east-west), with 0 on the centre line in the tray and shoulder figures. Y is measured from the machine face (back of the plate), increasing toward the user; the mouth is at negative Y. Heights are measured from the top (inside) surface of the plate.

### Base plate (shared, stays the same)

| Item | Value |
|---|---|
| Plate thickness | 2 mm: one folded sheet, so the floor, the side walls and the lips are all 2 mm (owner, 2026-11-16; earlier figures were 3 mm for the floor and 1.5 mm for the walls) |
| Inner width between side walls | 193 mm (owner, 2026-11-16; earlier figures were 193.5 and, in August 2026, 192) |
| Outer width across side walls | 197 mm (193 + 2 × 2) |
| Wall thickness | 2 mm |
| Overall length (machine face to front of the front lip) | 143 mm |
| Inner length (between rear and front lips) | 140 mm |
| Side wall height | 47 mm |
| Front lip height | 17 mm (1.5 mm above the tray's underside: it locks the tray) |
| Rear lip height (screws only) | 12 mm |

### Hole placement in the base plate

The holes show where the group head is. Most of the water ends up in this area.

| Item | Value |
|---|---|
| Centre hole | X = 97 mm (from the left inner wall), Y = 54 mm from the machine face, 8 mm diameter, directly beneath the group head |
| Hole ring 1 | radius 20 mm, 6 holes, 8 mm diameter, first hole at −45° |
| Hole ring 2 | radius 35 mm, 12 holes, 8 mm diameter, holes at 0°, 90°, 180°, 270° (every 30°) |

Angles are measured as seen from above with the user in front of the machine: 0° points right (east, +X), 90° points toward the machine face, and −45° points front-right, toward the user. A hole at angle θ sits at X = 97 + r·cos θ and Y = 54 − r·sin θ.

| Hole | Angle | X (mm) | Y from face (mm) |
|---|---|---|---|
| Centre | — | 97.00 | 54.00 |
| Ring 1 | −45° | 111.14 | 68.14 |
| Ring 1 | 15° | 116.32 | 48.82 |
| Ring 1 | 75° | 102.18 | 34.68 |
| Ring 1 | 135° | 82.86 | 39.86 |
| Ring 1 | 195° | 77.68 | 59.18 |
| Ring 1 | 255° | 91.82 | 73.32 |
| Ring 2 | 0° | 132.00 | 54.00 |
| Ring 2 | 30° | 127.31 | 36.50 |
| Ring 2 | 60° | 114.50 | 23.69 |
| Ring 2 | 90° | 97.00 | 19.00 |
| Ring 2 | 120° | 79.50 | 23.69 |
| Ring 2 | 150° | 66.69 | 36.50 |
| Ring 2 | 180° | 62.00 | 54.00 |
| Ring 2 | 210° | 66.69 | 71.50 |
| Ring 2 | 240° | 79.50 | 84.31 |
| Ring 2 | 270° | 97.00 | 89.00 |
| Ring 2 | 300° | 114.50 | 84.31 |
| Ring 2 | 330° | 127.31 | 71.50 |

The pattern has 19 holes in total (1 + 6 + 12), which matches the owner's photos. The closest pair of holes is 16.5 mm apart, centre to centre, leaving at least 8.5 mm of metal between holes. The pattern spans X = 58–136 mm and Y = 15–93 mm. The plate's centre line is at X = 96.5 mm, so the pattern is centred to within 0.5 mm. `scale_coupon` confirmed on 2026-11-16 that the plate holes sit where the parts put them: a strip laid between the side walls, with holes at X = 97 and 132, lines up with the centre hole and the 0° ring 2 hole. The version 1 figures further down (tray, shoulders, mouth clearances) were derived with the earlier widths and are kept as recorded.

### Mouth and tray extension

The mouth is where the release valve's water enters the tray. It stays the same between versions, and the tray must extend into it to capture that water. The mouth fit is also what centres the tray.

| Item | Value |
|---|---|
| Mouth depth (behind the machine face) | 32 mm, Y = −32 to Y = 0 |
| Tray extension into the mouth (no shoulders on this part) | 23 mm, from Y = −23 to Y = 0 |
| Tray stop | plastic bumpers at the back of the mouth (item 15 on the version 1 exploded diagram), press-fit in 4 mm holes and extending 8 mm. The 9 mm left behind the tray's rear end (32 − 23) leaves a 1 mm gap to the bumpers |
| Mouth drain grille | a slotted grille across the floor of the mouth, just behind the base plate's rear lip. It stops the mouth from acting as a tray when no tray is fitted: the water drains through it and ends up under the machine |
| Bumper positions | two 4 mm holes in the mouth's back wall, 42 mm in from each side (±47 mm from the centre line), 32 mm above the base plate floor. At that height the bumpers meet the tray's rear wall about halfway up its 15.5–47 mm span. On the owner's machine, only the left bumper is fitted: the right one has been removed so its hole can take the scale cable |
| Tray fore-aft play | about 1 mm, between the bumpers (Y = −24) and the front lip's inner face (Y = 140) |
| Tray body over the plate | Y = 0 to 140 mm (163 mm outer length − 23 mm), ending right at the front lip's inner face |
| Mouth width | 178 mm measured; the 174 mm tray body leaves 2 mm of clearance per side |
| Purge hole | about 9.7 mm diameter, fed by a 5 × 9 mm silicone tube on a 90° barb to G1/8 fitting. Centred in X; in Y, at the front edge of the mouth's top, against the machine face, so roughly Y = −9.7 to 0. The 23 mm tray extension catches it with about 13 mm to spare. The water falls about 34.5 mm, from the mouth's top (51 mm) to the tray's inside floor (16.5 mm) |
| Mouth height | top at 51 mm above the plate floor; 39 mm opening above the 12 mm rear lip (12–51 mm) |
| Tray body in the mouth | 15.5–47 mm above the plate (underside to top of the body wall): 3.5 mm clearance above the rear lip, 4 mm below the mouth's top |
| Perforated area offset from the grille's mouth edge | 26 mm |

The 7.5 mm of total vertical clearance in the mouth leaves room to tilt the tray: its front only needs to rise about 2 mm to clear the 17 mm front lip. Once the tray drops in, it is boxed in: about 1 mm of play fore and aft (bumpers and front lip), and about 1 mm per side (shoulder returns, with the mouth allowing 2 mm).

The purge outlet comes down through the mouth's top (the raised floor seen from inside the machine) at its front edge, centred on the machine width. The water therefore falls near Y = 0, over the front part of the tray's 23 mm extension.

Seen from the front, the mouth has a slotted drain grille on its floor, one bumper on each side of its back wall, and the base plate's rear lip with its two screws.

### Tray

| Item | Value |
|---|---|
| Sheet thickness | 1 mm |
| Body, inner | 172 (X) × 161 (Y) mm |
| Body, outer | 174 (X) × 163 (Y) mm |
| Opening between the shoulders' inner legs | 170 × 160 mm (172 − 2 × 1, 161 − 1) |
| Total, across shoulders | 210 mm in X (172 + 2 × (1 + 18)); 181 mm in Y (163 + 18) |
| Inside floor to top of shoulder | 31.5 mm |
| Inner depth (floor to underside of shoulder plate) | 30.5 mm |
| Outer height (underside to top of shoulder) | 32.5 mm |
| Underside above plate | 15.5 mm |
| Top of shoulder above plate | 48 mm (47 mm wall + 1 mm shoulder plate) |

#### Shoulder construction

The shoulders run along the left, right and front edges. The mouth end has none. Each one is a bent 1 mm plate:

1. It bends down 10 mm inside the tray, where it is soldered to the tray wall. These inner legs reduce the top opening from 172 × 161 mm to 170 × 160 mm.
2. It runs flat for 18 mm beyond the tray's outer wall. On the sides, this flat part rests on top of the base plate's side walls.
3. It bends down at the outer edge, 105 mm from the centre line. The leg's inner face is 104 mm from centre, 6 mm outside the side wall's outer face at 98 mm (half of the measured 196 mm).
4. A 6 mm return turns back inward at the bottom of the leg, ending 99 mm from centre. The two return tips are 210 − 2 × 6 = 198 mm apart, against the 196 mm base, leaving a 1 mm gap on each side (2 mm in total) with no contact. The returns limit side play and also act as a safety hem against cuts when carrying the tray.

The front shoulder extends from the front wall (Y = 140 mm) to Y = 158 mm, overhanging the front lip. The tray body has 9.75 mm of clearance to each side wall's inner face.

### Grille

| Item | Value |
|---|---|
| Size | 169 × 159 mm: 1 mm smaller than the 170 × 160 mm opening between the shoulders' inner legs |
| Sheet thickness | 1 mm |
| Height | 32 mm, standing on the tray floor; its top is 0.5 mm above the shoulder |
| Perforated area | 140 (X) × 120 (Y) mm, centred in X only |
| Perforated area offset from side edges | 14.5 mm |
| Perforated area offset from the mouth edge | 26 mm |
| Perforated area offset from the front edge | 13 mm (159 − 26 − 120) |
| Hole grid | 12 (X) × 11 (Y), 126 holes: the 2 holes next to the lift cut-outs and 4 at the front-right corner, where the "P" logo is, are omitted |
| Hole | 8.25 mm square |
| Hole gap | 3.6 mm (X), 2.6 mm (Y) |
| Hole pitch (centre to centre) | 11.85 mm (X), 10.85 mm (Y) |
| Grid span | 138.6 mm (X) × 116.75 mm (Y), centred in the 140 × 120 mm area (0.7 mm margin in X, 1.625 mm in Y) |
| Lift cut-outs | two half circles, r = 14 mm, on the east and west edges, centred on the middle row, 86 mm from the grille's mouth edge |

The gaps differ between X and Y (3.6 mm and 2.6 mm), as specified by the owner. With 8.25 mm squares, the 12 × 11 grid fits the 140 × 120 mm area in both directions.

Hole centres, measured from the area's south-west corner:

- X: 4.83, 16.68, 28.53, 40.38, 52.23, 64.08, 75.93, 87.78, 99.63, 111.48, 123.33, 135.18
- Y: 5.75, 16.60, 27.45, 38.30, 49.15, 60.00, 70.85, 81.70, 92.55, 103.40, 114.25

The middle row is at Y = 60.00, which lines up with the lift cut-outs. On that row, the outermost hole on each side is left out so that no hole sits too close to a cut-out.

#### STEP replica of the grille

A third-party STEP file is a replica of the version 1 grille, drawn flat (without the bent sides) to record where the holes are. Measured from the file:

| Item | Replica value |
|---|---|
| Plate | 169 × 158 mm, 1 mm thick |
| Holes | 8.5 mm squares on a 12 mm (X) × 11 mm (Y) pitch, so 3.5 mm and 2.5 mm gaps |
| Perforated span | 140.5 × 118.5 mm; 14 mm from the left edge, 14.5 mm from the right, 26 mm from the mouth edge and 13.5 mm from the front edge |
| Missing holes | the outer hole on each side of the middle row (beside the lift cut-outs) and 4 holes at the front-right corner (the "P" logo). The 2 holes just behind the logo are shortened to 8.0 mm |
| Lift cut-outs | Ø19 mm circles centred 6.5 mm in from each side edge, 85 mm from the mouth edge. Each cuts 16 mm into the plate and opens 13.9 mm along the edge |
| Purge hole | Ø15 mm, centred in X, 12 mm from the mouth edge |

The replica agrees with the owner's measurements to within a few tenths of a millimetre per hole. Over the full grid, the replica's spans are about 2 mm larger (140.5 × 118.5 mm against 138.6 × 116.75 mm). Measuring one full span with calipers, from the first hole's edge to the last hole's far edge, would settle which is right.

On the flat replica, each lift cut-out is a Ø19 mm circle. The owner measured r = 14 mm half circles on the real part, where the cut-out continues into the bent side, so the two may describe the same cut seen differently.

The purge hole's centre sits about 9.5 mm behind the machine face, assuming the grille's mouth edge is at Y ≈ −21.5. That suggests the purge outlet is slightly further back than the Y ≈ −4.85 centre assumed above.

### Tray capacity (reference)

| Item | Value |
|---|---|
| Gross cavity, full to the brim (172 × 161 × 30.5 mm) | 845 cm³ |
| Shoulders' inner legs | −4.5 cm³ |
| Grille side walls (1 mm sheet) | −20 cm³ |
| Net capacity, full to the brim | about 820 cm³ (0.82 L) |
| Net capacity, filled to 5 mm below the brim | about 687 cm³ (0.69 L) |

The 5 mm below-the-brim figure is the more realistic one to compare with version 2.

### Open points

None remain for the version 1 reference dimensions. Open points for the scale design are listed in its own section below. They matter for the version 2 tray, because the mouth fit is what centres it.

## Phase 2: drip tray scale (design brief)

The goal is a drip tray with a built-in scale, inspired by the La Marzocco design, so that a cup standing under the group head can be weighed while brewing. The parts are modelled with nurb in `print/parts/` (prefix `scale_`). This section describes the assembly and its constraints; the choices made on 2026-11-16 are under [Design decisions](#design-decisions-2026-11-16), and each part's card carries its own detail.

### Design direction: follow version 2, not version 1

- **Version 1** is a bent and soldered stainless box. Its metal-on-metal fit rattles, its full-height grille is heavy, and sheet metal limits the detail it can have. Do not copy its construction.
- **Version 2** is a molded plastic tray. It does not rattle, and molding allows small details: a ridge for the grille to rest on, an anti-splash ball under the purge hole and a deeper pocket in the mouth. That allows a lighter, simpler grille that lies on the ridge instead of standing on the floor.
- **Purge water:** version 2's molded ball under the purge hole is the preferred approach, because it is easier to clean than a chimney or hood. The third-party STL tray (see References) uses a hood instead.
- **Capacity:** the owner accepts a smaller water capacity than version 1's 0.69 L. That is the trade-off for an integrated scale.
- **The new tray** follows version 2: a plastic tray with molded details and grilles resting on ridges. Version 1's measurements are used only for the machine interfaces, listed under [Fixed interfaces](#fixed-interfaces).

### References

- La Marzocco drip tray with scales: the drip tray (1) and outer grille (2) have one opening per group. Each opening takes a rubber chimney grommet (3), a removable tower with magnets (4) and a separate cup grille (5). The scale modules (6) sit under the tray, one per group.
- La Marzocco scale module, exploded: the load cell (9) is mounted on a spacer (6), with a top block (5) and a top plate (2, 3) that receives the tower. The module also has an enclosure (10) and a PCB (17) on a bracket (19).
- Load cell: a 1 kg bar load cell, 12.7 × 12.7 × 75 mm, with four M4 through holes and four loose wires. The seller's drawing for this format gives hole spacings of 44 and 64 mm, centred on the 75 mm length: holes at 5.5, 15.5, 59.5 and 69.5 mm from one end. One end (holes at 5.5 and 15.5 mm) is the fixed end. The other end takes the load, in the direction of the arrow on its label. The same sheet gives 1.0 ± 0.1 mV/V, 1000 Ω and a 5 V recommended excitation, but neither the overload limits nor the deflection.
- Version 1 grille replica (STEP): a flat replica of the version 1 grille, giving exact hole positions. It is described under [STEP replica of the grille](#step-replica-of-the-grille).
- Third-party drip tray (STL): someone else's attempt at a tray. It is 192.5 mm wide, 169.3 mm deep and 49 mm tall overall, with a 2 mm floor and walls about 19 mm high. It sits directly on the base plate, which is not the approach wanted here, but it shows another way to handle the purge water:
  - A 6 mm thick shelf (Z = 13 to 19) reaches into the mouth, clearing the 12 mm rear lip by 1 mm.
  - On that shelf, a D-shaped hood (17 mm wide, 1 mm walls, about 30 mm long) rises to Z = 49, just 2 mm under the mouth's top, enclosing the purge outlet.
  - Inside the hood, a curved ramp catches the jet and turns it forward, down into the main tub. The water leaves under the hood's front wall, which stops at Z = 19.

  This works, but it is harder to clean than version 2's ball.

### Parts to design

All parts are printed: Bambu PETG Basic, except the soft parts in Bambu TPU 90A. A later version may have the two grilles cut from sheet metal (see decision 12).

| # | Part (`print/parts/`) | Material | Role |
|---|---|---|---|
| 1 | `scale_base` | PETG | One piece on the base plate floor: the raised pad that holds the load cell's fixed end, the landings for the overload stops, the bridge that limits upward travel, and the open-top box for the scale PCB. Its underside is flat. Screwed from below through three of the base plate's holes. |
| 1b | `scale_washer` (×3) | PETG | Stepped washer under the base plate: its collar fills an Ø8 hole and centres the M3 screw, and so `scale_base`. |
| 2 | `scale_lid` | PETG | Skirted lid of the PCB box. |
| 3 | `scale_adapter` | PETG | Bolted on the load cell's loaded end. Carries the seat and the weak magnets for the tower, and the two adjustable overload stop screws. |
| 4 | `scale_tower` | PETG | Carries the inner grille's load down through the tray window to the adapter. Weak magnets below, strong magnets above. |
| 5 | `scale_grille_inner` | PETG | Cup grille, 90 mm square, centred on the group head axis (97, 54). Solid centre over the chimney, drain slots around the edge, a drip ring underneath, two finger holes. |
| 6 | `scale_grille_outer` | PETG | Version 2 style: a flat plate lying on rails in the tray, with a cut-out for the inner grille, a purge opening, a pull hole at the south-east corner and a front apron that hooks over the tray's front wall. |
| 7 | `scale_tray` | PETG | Version 2 style. Hangs from the base plate's side walls, extends into the mouth and locks behind the front lip. A plateau over the scale, a deep well at the front, a kerbed window for the tower, a sloping rear that catches the purge water. Prints standing on its front wall. |
| 8 | `scale_chimney` | TPU 90A | Sleeve pushed over the tray's window kerb. Rises above the highest water level around the tower without touching it. |
| 9 | `scale_bumper` | TPU 90A | Rear stop on the tray's right side, in place of the machine's right bumper, whose hole now takes the cable. |
| 10 | `scale_coupon` | PETG | Fit gauge printed before the large parts: checks the base plate's inner width, the side wall height and two hole positions. |
| 11 | `scale_assembly` | — | nurb assembly of all the parts on a model of the base plate and mouth, to check clearances. |

### How the assembly works

**Load path.** Cup → inner grille → strong magnets → tower → weak magnets → load cell's loaded end → load cell's fixed end → scale base → base plate. Nothing else may touch the load path. The tray, the gasket and the outer grille must all keep clear of the inner grille and the tower. Any contact or friction would carry part of the weight and make the reading wrong. The 1–2 mm gap between the grilles serves this purpose as well as preventing capillary bridging.

**Water path.** Drips fall on the inner grille and run off its skirt into the tray. The skirt overhangs the top of the chimney like an umbrella, so water does not run down the tower. The outer grille drains straight into the tray. The chimney rises above the highest water level, keeping water away from the tower and the load cell without touching the tower. Two features keep water from forming a bridge between the grilles, which would carry part of the weight and spoil the reading:

- a 1–2 mm horizontal gap between the inner and outer grilles;
- the inner grille sitting 1–2 mm higher than the outer grille, as in the La Marzocco design.

**Removal and refitting.**

1. Lift the inner grille by its two finger holes. Because the grille-to-tower magnets are stronger than the tower-to-adapter magnets, the tower comes away with the grille. The weak bottom magnets keep the upward pull on the load cell below its limit.
2. Pull the tower off the grille by hand, to clean the grille.
3. Pull the outer grille toward the user by its south-east hole, lifting its front apron over the tray's front wall.
4. Tilt the tray up over the front lip and slide it out of the mouth. The chimney stays on the tray and pulls off its kerb for cleaning.

Refitting is the reverse. The tray window has to line up over the adapter so the tower can drop back onto its seat.

**Cable.** Four AWG 24 wires, about 15 cm, come from the combined board inside the machine through the right bumper hole (Ø4, in the mouth's back wall). They run down the gap behind the tray, over the rear lip in a saddle that is part of `scale_base`, and into the PCB box from below its lid's skirt. The wires get their ferrules after passing the hole and go into the PCB's screw terminal J1.

### Fixed interfaces

Coordinates for this section, in mm: X from the base plate's left inner wall face (the centre line is at X = 96.5); Y from the machine face, increasing toward the user (the mouth is at negative Y); Z from the top surface of the base plate floor.

| Interface | Value |
|---|---|
| Base plate floor | 193 (X) × 140 (Y) inside, 2 mm thick; 10 mm free space below it for screw heads |
| Base plate holes | Ø8 mm: centre hole at (97, 54); 6 holes on r = 20 starting at −45°; 12 holes on r = 35 every 30° (full table under [Hole placement in the base plate](#hole-placement-in-the-base-plate)) |
| Group head axis | X = 97, Y = 54 |
| Side walls | Z = 0 to 47, 2 mm thick, 197 mm across the outside; a tray hangs from them |
| Front lip | Z = 0 to 17, at Y = 140–142. It locks the tray: the tray's front wall must reach below Z = 17 behind it, and the tray is tilted over it to fit or remove |
| Rear lip | Z = 0 to 12, at Y = −2 to 0. Screws only |
| Mouth | X = 7.5 to 185.5 (178 wide, centred), Y = −32 to 0, opening Z = 12 to 51 |
| Purge hole | about Ø9.7 in the mouth's top (Z = 51), centred in X, at Y ≈ −9.7 to 0 |
| Mouth drain grille | the mouth's floor is slotted, so water drains under the machine when no tray is fitted |
| Left bumper | in the mouth's back wall (Y = −32) at X = 49.5, Z = 32, 8 mm proud: the tray's rear end can reach Y = −24 at most |
| Cable hole | the right bumper hole, Ø4, at X = 143.5, Z = 32 in the mouth's back wall |
| Group head footprint | the base plate's hole rosace: 78 mm across (ring radius 35 + hole radius 4), centred at (97, 54). The group head is centred on the tray in X but not in Y |
| Inner grille footprint | slightly larger than the rosace, centred at (97, 54). For example, a 100 mm square spans X = 47 to 147 and Y = 4 to 104, and a 90 mm square spans X = 52 to 142 and Y = 9 to 99 |
| Reference capacity | version 1 holds about 0.69 L when filled to 5 mm below the brim |

### Derived constraints and observations

1. **Load cell orientation.** The tower sits over the loaded end, so that end goes under the group head at (97, 54). The fixed end is then 55 mm away, and the cell's body spans the following area, depending on orientation:
   - Along X, fixed end to the right: the cell spans X = 87 to 162, and the fixed-end holes sit at X = 147 and 157, close to the cable hole.
   - Along X, fixed end to the left: the cell spans X = 32 to 107.
   - Along Y, fixed end toward the user: the cell spans Y = 44 to 119.
   - Toward the machine: not possible, because the fixed end would be in the mouth.
2. **Height under the tray.** The load cell is 12.7 mm tall before any mount. Version 1's underside is only 15.5 mm above the plate, and version 2 sits even lower. So the tray floor must be raised above the load cell, its tower adapter and the gasket's bottom lip.
3. **Insertion path.** The tray does not slide in over its whole length: it comes down tilted, its rear end travels only about 25 mm into the mouth, and the front then drops behind the front lip. A low part of the tray's underside therefore only passes over what lies within about 25 mm in front of its final position, and it does so raised by the tilt. Two rules follow:
   - Everything from the scale back to the mouth is under one raised floor, so that no low part of the tray ever travels over the scale.
   - The first low part in front of the scale (the well's rear face) ends at least 3 mm in front of the scale's front edge. The left bumper stops the tray before the two can meet.

   The front wall must still reach below Z = 17 to keep the front lip lock.
4. **Capacity.** The raised areas and the chimney both take volume, and the chimney top caps the usable water level. A capacity below version 1's 0.69 L is accepted, but the design should still state its usable volume.
5. **Screws from below.** The base plate holes are Ø8 mm, and an M4 socket head is only 7 mm across, so it would pass through. Use washers of at least 12 mm, or larger heads, within the 10 mm under the plate. The base plate's own two rear screws come horizontally out of the rear lip: Ø7 heads, 4 mm proud (Y = 0 to 4), axis 5 mm above the floor, 50 mm each side of the centre line (measured in August 2026, `print/measurements.toml`). Nothing may sit there. The base plate's hole spacing does not match the load cell's 10 mm hole spacing. So the load cell bolts to the scale base, and the scale base bolts to the plate.
6. **Load cell protection.** The free end needs a small gap underneath it to deflect. A printed overload stop under it (and possibly over it) would protect the 1 kg cell. Weight budget: a mug of 100–250 g plus under 100 g of coffee, so at most 350 g, plus the inner grille, tower and magnets. That stays well within the 1 kg rating, leaving margin for someone pressing on the grille.
7. **Magnets.** The grille-to-tower pull-off force must be higher than the tower-to-cell pull-off force. The tower-to-cell force must be well below the load cell's upward overload limit. Alignment features, such as a cone or pins, make the tower re-seat in the same place every time. Magnets in stock, both round and N48:
   - Ø8 × 3 mm: pull force not stated.
   - Ø5 × 2 mm: rated 7 N.
8. **Tray stop.** With the right bumper removed, only the left bumper stops the tray at the back. The new tray could carry its own stop, or the cable's grommet could act as one.
9. **Purge water next to the inner grille.** The purge hole drops water at Y ≈ −9.7 to 0, just behind the inner grille. A 100 mm inner grille starts at Y = 4, so with the 1–2 mm gap, the outer grille keeps only a 2–3 mm strip between the purge opening and the inner grille's cut-out. A 90 mm inner grille (starting at Y = 9) leaves about 7–8 mm. The tray needs a splash feature under the purge hole: version 2's molded ball is preferred. The outer grille needs an opening above it; version 1's grille has a Ø15 mm round hole there.
10. **Wet area.** The scale base and the PCB enclosure sit in the area that collects any leaks and splashes, and that drains through the base plate holes. The PCB enclosure needs to be protected against water, and the cable needs a drip loop.

### Design decisions (2026-11-16)

These settle the former open points. Values are the modelling defaults; each is a parameter of its part, and the part's card records any later change. Coordinates as in [Fixed interfaces](#fixed-interfaces). That frame (X from the left wall, Y toward the user, Z up) is left-handed: the parts are modelled in it and each ends with a reversal of Y (`scale_frame` in `print/system.py`), so that the printed object is not the mirror image of the intended one. In the viewer the machine is therefore at the top (north) and the user at the bottom.

1. **Load cell along X, fixed end to the right.** The tower axis is midway between the loaded end's two holes, so the cell spans X = 86.5 to 161.5 at Y = 54 ± 6.35, with the loaded-end holes at X = 92 and 102 and the fixed-end holes at X = 146 and 156. Reasons: the whole scale stays behind Y = 72, which leaves the front of the tray free for a full-width well; the fixed end, where the cell's wires come out, is next to the PCB box and the cable hole. Along Y, the cell would cut the well in two and put the PCB far from the cable.

2. **Vertical stack**, Z above the base plate floor:

   | Level | Z (mm) |
   |---|---|
   | `scale_base` floor | 0 to 4 |
   | Fixed-end pad, so cell underside (1 mm gap under the rest of the cell) | 5.0 |
   | Cell top | 17.7 |
   | Upward-stop bridge over the cell (0.6 mm gap) | 18.3 to 21.3 |
   | PCB box with lid (PCB underside at 8.0, terminal blocks up to 18.0) | up to 20.6 |
   | Adapter seat / top of its centring boss | 20.2 / 23.2 |
   | Tray raised floor, underside / top | 25.0 / 27.5 |
   | Window kerb top | 35.5 |
   | Highest usable water level | 43.5 |
   | Chimney top | 45.0 |
   | Rails, so outer grille underside | 46.0 |
   | Side walls of the base plate (tray shoulders rest on them) | 47.0 |
   | Inner grille underside (its drip ring hangs down to 43.0) | 47.5 |
   | Outer grille top | 48.5 (as version 1) |
   | Inner grille top | 50.0 (1.5 mm above the outer grille) |
   | Mouth top | 51.0 |

   The PCB box fits under the tray: the cell and its adapter already force the tray floor up to Z = 25, and the box needs 20.6. The mouth is not needed for it, and the tray keeps its full-width extension into the mouth.

3. **Base fixing (revised 2026-11-16).** `scale_base` has a flat underside, so it prints floor down, and three Ø3.4 clearance holes through its 4 mm floor, over base plate holes: (102.18, 34.68) and (111.14, 68.14) on ring 1, (127.31, 36.50) on ring 2. Each has a hex pocket, 2 mm deep in the floor's top, for an M3 locknut (5.4 mm across flats, 4 mm tall), which keeps the nut from turning. Under the plate, each hole takes a `scale_washer`: a Ø14 × 1.5 mm flange with a Ø7.7 × 1.8 mm collar that fills the Ø8 hole, so the washer centres the screw and the screw locates the base. An M3 × 10 flat-head screw goes up through washer, plate and floor into the nut: 1.5 + 2 + 2 + 4 = 9.5 mm, so the screw's end stands 0.5 mm above the nut, through its nylon ring, 6.5 mm above the plate. That is why no hole is under the cell, which is 1 mm above the floor; the centre hole (97, 54) of the first model is not used. Nothing is screwed into the plastic, so the base comes off and goes back as often as needed. Spigots under the base were dropped: they would lift the base off the print bed.

4. **Cell fixing**, with the screws in stock (M4 × 10 countersunk, M4 × 8 set screws).
   - Fixed end: two M4 × 10 countersunk from under `scale_base`, through the 5 mm pad, so 5 mm of thread in the cell. The cell is mounted on the bench before the base goes into the machine.
   - Loaded end: one M4 × 10 countersunk from above, in the inner hole (X = 102), holds `scale_adapter`. The adapter straddles the cell's sides, so one screw is enough to keep it square. The outer hole (X = 92) is kept for the stop below.

5. **Overload stops.** The 1 kg cell is the weak part: pressing on the grille with a hand exceeds 1 kg easily.
   - **Down:** an M4 × 8 set screw in the loaded end's outer hole, socket up, standing about 1 mm proud under the cell, above a landing on `scale_base`. The hole is at the cell's tip, where the travel is largest. The screw is reached with a 2 mm hex key through a Ø3 hole in the adapter, tray and tower off. Setting, using the scale as its own indicator: turn the screw in until the reading moves (contact), then back it off by the cell's deflection at about 1.2 kg (M4 pitch 0.7 mm: 0.1 mm is 51°). The deflection is found the same way, once, with a known weight on the adapter. A drop of varnish holds the setting. A fixed printed gap would not do: the deflection is a few tenths of a millimetre, of the order of the print tolerance. The fixing screw and the set screw cannot share one hole: about 7 mm of each do not fit in 12.7 mm.
   - **Up:** a bridge, part of `scale_base`, crosses over the cell at X = 116 to 119, just clear of the tower's foot (which reaches X = 115 and sits lower than the beam's top), with a 0.6 mm gap. The cell slides under it lengthwise at assembly. The real protection is upstream: the tower-to-adapter magnets let go before the cell is overloaded (decision 6), so nothing can pull harder than they hold.

6. **Magnets.** Both joints seat plastic on plastic; the magnets never touch.
   - Tower to adapter (weak): two pairs of Ø5 × 2 mm, 1.0 mm apart. Rated 7 N each in contact, about a third of that at 1 mm, so about 5 N in total, half of the cell's 1 kg rating. The gap is a parameter of `scale_tower`: a wider gap weakens the joint.
   - Grille to tower (strong): two pairs of Ø8 × 3 mm, 0.5 mm apart, several times stronger.
   - Eight magnets in total, four of each size. Two per joint is what fits beside the clamp screw and the stop screw's access hole on the adapter. Wells open toward the joint; a drop of varnish over each magnet protects it from water.
   - In a sheet-metal version, a ferritic (magnetic) stainless grille needs no magnets of its own.

7. **Tower, seats and tipping.** A cup is rarely centred. 350 g at 25 mm off the axis is a tipping moment of 0.086 N·m, and at the grille's edge (45 mm) 0.15 N·m. A slim tower held by weak magnets would tip off the adapter, so:
   - the tower is wide, Ø36 at its foot and top (seat radius 18 mm), and hollow;
   - both joints seat on a square truncated pyramid (24 mm at its base, 30° flanks, 3 mm high): a boss on the adapter under a socket in the tower's foot, and a boss under the grille in a socket in the tower's top. The pyramid centres, sets the angle so the square grille stays parallel to its cut-out, has no play, and resists tipping without relying on the magnets.

8. **Window and chimney.** The tray window leaves 3 mm all round the Ø36 tower, more than the tray's play of about 1 mm in X and Y. A kerb, part of the tray, surrounds the window and rises 8 mm above the plateau. `scale_chimney` (TPU) pushes over the kerb and rises to Z = 45.0. The seal therefore sits 8 mm above the floor. Window, kerb and chimney share one outline: a circle drawn out to a point toward the user and toward the machine, with 45° flanks and rounded tips (4 mm radius on the kerb's outer face, which reaches 30.9 mm from the axis). The tray prints standing on its front wall (decision 9): the point toward the user lets the kerb start on the floor instead of in the air, and the point toward the machine roofs the window. The rounded tips are for the TPU sleeve, which would not follow a sharp corner. Under the tip toward the user, a 10 mm web carries the tray's 45° ramp up to the top of the kerb, so that the rounded tip does not start in the air; the chimney has a notch there and does not grip the kerb over those 10 mm. The inner grille's solid centre (Ø74) and its drip ring (Ø70 inside) cover the chimney like an umbrella, with 2.5 mm between the chimney top and the grille, against less than 0.5 mm of cell travel.

9. **Tray.** It prints standing on its front wall, 163 mm tall, without supports (owner, 2026-11-16). Floors and walls are then vertical walls for the printer, which is also the most watertight way to lay them, and every change of level is a slope. Two earlier stances were dropped: upright, where the raised floor is one large overhang, and tilted at 45° on a flattened edge, which leaned 30 mm off its footing.
   - Support: side shoulders resting on the base plate's side walls, as in versions 1 and 2. No shoulder in the mouth. A corbel under each shoulder centres the tray between the walls with 0.5 mm of play per side.
   - Plateau, from X = 72 to the right wall: the floor's underside is at Z = 25.0 over the scale, back from Y = 89; in front of that a 45° ramp comes down to the well (underside Z = 2).
   - Left zone, X below 72, where nothing stands under the tray: well at the front, then one gentle slope up to Z = 13.5 at the rear lip, so the water runs forward by itself.
   - Rear: no vertical rear wall, which would be a ceiling for the printer. Each floor ends in a 45° slope up to the rim at Y = −23. The purge water falls on it and runs forward; no splash ball.
   - Rails for the outer grille along both sides, at Z = 46.0; none at the rear.
   - Size: 202 mm across the shoulders, for the 256 mm bed (P2S).
   - Capacity and print figures are on the part's card.

10. **Grilles.** Inner grille 90 mm square (X = 52 to 142, Y = 9 to 99), which keeps a 7 mm strip between it and the purge opening; one row of drain slots along each side, outside the drip ring. Gap to the outer grille's cut-out: 2 mm all round. Both 2.5 mm thick, printed top face down. Outer grille: purge opening Ø16 centred in X, pull hole 20 mm square at the south-east corner, front apron. It fits the 180 mm bed.

11. **Rear stop and rattle.** The cable bundle is 3.7 mm in a Ø4 hole, so nothing else fits in that hole. Two pegs stand out of the tray's rear slope, at the bumpers' height. The left one meets the machine's remaining bumper. The right one, about 14 mm outboard of the cable hole, carries `scale_bumper`, a TPU cap, and ends 8 mm behind the tray's rear like the bumper it replaces. The cable comes down in the 8 mm left behind the tray. No TPU pads under the shoulders for now: printed PETG on steel does not ring as version 1's steel on steel does. Pads can be added later without changing the tray.

12. **Sheet metal later.** The two grilles are flat plates with cut openings and at most one straight bend (the outer grille's apron); the inner grille's drip ring and seat would become a separate printed part under a flat sheet. The tray stays printed or molded.

13. **PCB box.** For the 19.3 × 24.7 mm board of `pcb/scale`.
   - Mounting: two M2 × 3 heat-set inserts under the board's two holes, along the box's machine-side wall, (`add_heat_insert_against` and `INSERT_M2` from `print/system.py`), M2 × 5 screws. The inserts need 3.6 mm, so the board's underside is 4 mm above the box floor, as in the combined board's box. A ledge along the user-side wall and a rib under each short edge, beneath the terminal blocks, carry the rest of the board; the ribs reach 1.2 mm under it, short of the terminal pads 2.5 mm in.
   - Room: 10 mm in front of each terminal block (J2, load cell, on one short side; J1, bus, on the other) for the ferrules, which stand 5 mm out of the blocks, and the wire bend. J1 (bus) is at the right end, on the side where the bus cable comes in, and J2 (load cell) at the left: the cell's 20 cm wires come from its fixed end on the right and run along the box, held by two cable tie arches in the corridor between box and cell. That keeps the I2C cable short. The terminal screws are reached from above with the lid off and the tray out.
   - Lid: `scale_lid` is a cap whose skirt comes down the full height of the box and holds by a snug fit, without screws; the tray above keeps it from lifting. The wires enter through notches at the bottom of the skirt, 5 mm wide for the four loose wires (a piece of heat-shrink around them can tighten the entry later). Water falling on the lid runs off.
   - No vents: the board dissipates about 40 mW (7.7 mA at 5 V), and a vent would only let water in.

### Assembly

Hardware: three M4 × 10 countersunk screws, one M4 × 8 set screw, three M3 × 10 flat-head screws and three M3 nylon-insert locknuts, two M2 × 3 heat-set inserts, two M2 × 5 screws, four Ø5 × 2 and four Ø8 × 3 magnets, three cable ties 2.5 mm wide.

On the bench:

1. `scale_base`: set the two M2 inserts in the box's bosses.
2. Slide the load cell in from the right, under the bridge, its wires at the fixed end (right) and its arrow pointing down at the left. Fix it with two M4 × 10 from under the base.
3. Turn the M4 × 8 set screw into the cell's outermost hole at the loaded end, socket up, until it stands about 1 mm proud under the cell.
4. Put `scale_adapter` over the loaded end and fix it with one M4 × 10 from above, in the inner hole.
5. Press the magnets in: two Ø5 × 2 in the adapter's top, two in the tower's foot, so that the two parts attract; two Ø8 × 3 in the tower's top, two under the inner grille, likewise. A drop of varnish over each.
6. Screw the PCB in the box, J1 at the right end, toward the cell's fixed end. Run the cell's wires along the box to J2 at the left end, tie them to the two arches between box and cell, and wire them to J2.

On the machine:

7. Pass the four wires from the combined board through the right bumper hole, crimp the ferrules.
8. Set the base on the base plate, its three holes over the plate's holes, and drop an M3 nut in each hex pocket. Tip the machine, push a `scale_washer` into each of those three plate holes from below, collar first, and drive an M3 × 10 through each into its nut, holding the nut with a finger from above if it lifts.
9. Lay the wires in the saddle (the notched post at the base's rear edge, which holds them clear of the rear lip's edge) and tie them to it through its window. Wire them to J1 and press `scale_lid` on.
10. Set the overload stop (decision 5), with the tower and a known weight on the adapter.
11. Push `scale_chimney` over the tray's kerb and `scale_bumper` on the tray's right peg. Fit the tray, then the outer grille, the tower and the inner grille.

### Settled with the owner (2026-11-16)

- Tray floor: a well at the front; the tray printed standing on its front wall, with ramps and a pointed window (owner's proposal).
- Screws: what is in stock, see decision 4.
- Base plate: 193 mm inside, 197 mm outside, 2 mm sheet.
- Load cell: the seller's drawing, see References. No overload figure, hence the adjustable stop.
- Cable: four loose wires, 3.7 mm as a bundle, ferrules at the terminal block.
- Magnets: Ø8 × 3 and Ø5 × 2 from stock are enough. The owner can also buy Ø5 × 10, Ø10 × 4, 6 × 3 × 2 blocks and 5 mm cubes (N42) if a joint needs to be stronger.

## Notes

- The version 1 dimensions are as supplied by the owner. They were checked against each other for consistency, not measured again. Values marked "derived" or listed under open points should be measured on the part.
- The ring angles can't be checked from the owner's photos to better than about 10°, because of perspective.

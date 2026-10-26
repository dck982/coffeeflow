"""Assembly notes for the ps_upstream board.

J1, J2 and RT1 use the LCSC footprints of the parts themselves, origin and
orientation unchanged; only pin 2 of J1 and J2 became a slot. Their
placement therefore needs no correction, as for the HT396R headers of the
combined board.
"""

PLACEMENT_REVISION = 'v1: LCSC footprints, no correction'

README_NOTES = """Select through-hole assembly for J1, J2 and RT1.
J1 (HT396R-3.96-3P) and J2 (VH-3A): pin 2 is not soldered. It drops into a
non-plated slot between L and N; if the assembly review asks, this is
intended.
RT1 (TDK B57236S0100M000) is not polarised.
H1 and H2: M3 holes between L and N, for nylon screws only.
Check the J1 opening (towards the board edge) and the J2 latch ramp
(towards the board end) in the JLCPCB preview.
"""

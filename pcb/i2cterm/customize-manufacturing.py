"""Assembly conventions for the two vertical Grove connectors in i2cterm."""
import math
ROTATION_OFFSETS={'J1':-90.0,'J2':-90.0}
PLACEMENT_REVISION='Vertical C722737 Grove pin-row midpoints; -90deg supplier-CAD rotation offset'
README_NOTES='''Top-side assembly: R1, R2, R3, D1 and two through-hole Grove connectors.
J1 and J2: CAX HY-4A / C722737, vertical, 2mm pitch, 1.0mm drills.
Placement centres use the numbered pin-row midpoint; rotation offset -90deg
matches the supplier CAD and the sensors project's verified convention.
J1 and J2 are connected pin for pin: 1=SCL, 2=SDA, 3=3V3, 4=GND.
One shared pair of 4.7k pull-ups; green LED indicates 3.3V power.
Mounting holes are 2.7mm NPTH for M2.5, excluded from assembly files.
All routing is on F.Cu, without vias. No GND copper plane.
Check both Grove orientations and LED pad 1 (cathode, GND) in the assembly preview.
'''
def customize_placement(ref,footprint,x,y,angle):
    if ref not in ('J1','J2'):return x,y,angle
    assert footprint[1]=='I2CTerm_Local:CONN-TH_HY-4A'
    pads=[e for e in footprint if isinstance(e,list) and e[0]=='pad' and e[1]]
    ats=[next(e for e in p if isinstance(e,list) and e[0]=='at') for p in pads]
    cx=sum(float(a[1]) for a in ats)/len(ats);cy=sum(float(a[2]) for a in ats)/len(ats)
    th=math.radians(angle)
    return x+cx*math.cos(th)+cy*math.sin(th),y-cx*math.sin(th)+cy*math.cos(th),angle

# /// script
# dependencies = []
# ///
"""Add a front GND zone to the saved design without moving parts or tracks.
Refill using kicad-cli pcb drc --refill-zones --save-board afterwards.
"""
import math,uuid,shutil,json
from pathlib import Path
from sexpr import parse,dump,key,get,node,Sym
HERE=Path(__file__).resolve().parent
path=HERE/'i2cterm.kicad_pcb';source=path.read_text();board=parse(source)
def children(a,k):return [e for e in a if key(e)==k]
assert not children(board,'zone'),'Existing zones found; review rather than replace them.'
backup=HERE/'.history/user-layout-before-ground.kicad_pcb';shutil.copy2(path,backup)
lines=[e for e in children(board,'gr_line') if get(e,'layer')[1]=='Edge.Cuts']
points=[get(e,k)[1:3] for e in lines for k in ['start','end']]
x0=min(p[0] for p in points);x1=max(p[0] for p in points)
y0=min(p[1] for p in points);y1=max(p[1] for p in points)
def poly(points):return node('polygon',node('pts',*[node('xy',round(x,6),round(y,6)) for x,y in points]))
def zid():return node('uuid',str(uuid.uuid4()))
z=node('zone',node('net','GND'),node('layer','F.Cu'),zid(),node('name','GND top'),node('hatch',Sym('edge'),.5),node('connect_pads',node('clearance',.25)),node('min_thickness',.2),node('fill',Sym('yes'),node('thermal_gap',.25),node('thermal_bridge_width',.3),node('island_removal_mode',0)),poly([(x0+.3,y0+.3),(x1-.3,y0+.3),(x1-.3,y1-.3),(x0+.3,y1-.3)]))
newzones=[z]
for f in children(board,'footprint'):
    ref=next(e[2] for e in children(f,'property') if e[1]=='Reference')
    if ref not in ['H1','H2']:continue
    cx,cy=get(f,'at')[1:3]
    keep=node('zone',node('net',''),node('layers','F.Cu','B.Cu'),zid(),node('name',ref+' screw head clearance'),node('hatch',Sym('edge'),.5),node('keepout',node('tracks',Sym('not_allowed')),node('vias',Sym('not_allowed')),node('pads',Sym('allowed')),node('copperpour',Sym('not_allowed')),node('footprints',Sym('allowed'))),poly([(cx+2.8*math.cos(i*math.tau/64),cy+2.8*math.sin(i*math.tau/64)) for i in range(64)]))
    newzones.append(keep)
new=source.rstrip()[:-1]+'\n'+'\n'.join(dump(z) for z in newzones)+'\n)\n'
after=parse(new)
assert after[:len(board)]==board,'Unexpected source geometry change'
path.write_text(new)
print('Added F.Cu GND plane and M2.5 screw-head clearances; parts and tracks preserved.')

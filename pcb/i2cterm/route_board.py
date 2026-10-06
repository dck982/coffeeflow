# /// script
# dependencies = ["kicad-python==0.8.0"]
# ///
"""Route the open i2cterm pass-through board using KiCad 10 IPC.
Run after the one-time prepare_passthrough.py migration and PCB reload.
Preserves the saved outline and holes; replaces placement and routing with a starter layout.
Do not run this over a finished user layout; use ../generate_manufacturing.py for exports.
"""
import csv,json,math
from pathlib import Path
from kipy import KiCad
from kipy.board_types import BoardLayer,Track,Zone,ZoneType,ZoneConnectionStyle
from kipy.common_types import PolygonWithHoles
from kipy.geometry import Vector2,PolyLine,PolyLineNode,Angle
from kipy.util import from_mm

HERE=Path(__file__).resolve().parent
k=KiCad();version=k.get_version();k.check_version();b=k.get_board()
assert b.document.board_filename=='i2cterm.kicad_pcb'
assert Path(b.document.project.path).resolve()==HERE
fp={f.reference_field.text.value:f for f in b.get_footprints()}
assert set(fp)=={'J1','J2','R1','R2','R3','D1','H1','H2'},set(fp)
V=Vector2.from_xy_mm
placements={'J1':(58.5,53.3,90),'J2':(58.5,66.7,90),'R1':(60.5,61,0),'R2':(61.5,58.5,0),'R3':(64.5,58.5,0),'D1':(65.5,61,0)}
rows={r['Reference']:r for r in csv.DictReader((HERE/'i2cterm_bom.csv').open())}
commit=b.begin_commit()
for ref,(x,y,angle) in placements.items():
    f=fp[ref];f.orientation=Angle.from_degrees(angle);f.position=V(x,y)
    f.description_field.text.value=rows[ref]['Description']
    f.description_field.visible=False
b.update_items([fp[ref] for ref in placements])
fp={f.reference_field.text.value:f for f in b.get_footprints()}
pads={(r,p.number):p for r,f in fp.items() for p in f.definition.pads}
nets={n.name:n for n in b.get_nets()}
expected={'1':'SCL','2':'SDA','3':'+3V3','4':'GND'}
for ref in ['J1','J2']:
    for pn,net in expected.items():assert pads[ref,pn].net.name==net,(ref,pn,pads[ref,pn].net)
old=[*b.get_tracks(),*b.get_zones()]
if old:b.remove_items(old)
items=[]
def pad(ref,n):
    p=pads[ref,n].position
    return p.x/1e6,p.y/1e6

def route(net,points,width=.25):
    for a,c in zip(points,points[1:]):
        t=Track();t.start=V(*a);t.end=V(*c);t.width=from_mm(width);t.layer=BoardLayer.BL_F_Cu;t.net=nets[net];items.append(t)

for pn,net in expected.items():route(net,[pad('J1',pn),pad('J2',pn)],.3 if pn in ['3','4'] else .25)
route('SCL',[(58.5,61),pad('R1','1')])
route('+3V3',[pad('R1','2'),(62.5,61)],.3)
route('SDA',[(60.5,58.5),pad('R2','1')])
route('+3V3',[pad('R2','2'),(62.5,58.5),pad('R3','1')],.3)
route('LED_A',[pad('R3','2'),(66.2875,59.4625),pad('D1','2')])
route('GND',[(64.5,61),pad('D1','1')],.3)
# Move existing annotations to follow the moved components; preserve their style.
text_positions={'R1':(60.5,62.6,0),'R2':(59,58.5,0),'R3':(63,61,0),'SCL':(58.5,56.6,0),'SDA':(60.5,56.6,0),'3V3':(62.5,56.6,0),'GND':(64.5,56.6,0),'PWR':(65.5,62.3,0)}
texts=b.get_text()
for t in texts:
    if t.value in text_positions:
        x,y,a=text_positions[t.value];t.position=V(x,y);t.attributes.angle=a
b.update_items(texts)

def polygon(points):
    line=PolyLine()
    for x,y in points:line.append(PolyLineNode.from_xy(from_mm(x),from_mm(y)))
    p=PolygonWithHoles();p.outline=line
    return p

outline=[s for s in b.get_shapes() if s.layer==BoardLayer.BL_Edge_Cuts]
points=[p for s in outline for p in [s.start,s.end]]
xmin=min(p.x for p in points)/1e6;xmax=max(p.x for p in points)/1e6
ymin=min(p.y for p in points)/1e6;ymax=max(p.y for p in points)/1e6
for ref in ['H1','H2']:
    cx=fp[ref].position.x/1e6;cy=fp[ref].position.y/1e6
    z=Zone();z.type=ZoneType.ZT_RULE_AREA;z.name=ref+' screw head clearance'
    z.layers=[BoardLayer.BL_F_Cu,BoardLayer.BL_B_Cu]
    z.outline=polygon([(cx+2.8*math.cos(i*math.tau/64),cy+2.8*math.sin(i*math.tau/64)) for i in range(64)])
    z.proto.rule_area_settings.keepout_copper=True
    z.proto.rule_area_settings.keepout_tracks=True
    z.proto.rule_area_settings.keepout_vias=True
    items.append(z)
z=Zone();z.name='GND top';z.layers=[BoardLayer.BL_F_Cu];z.net=nets['GND']
z.outline=polygon([(xmin+.3,ymin+.3),(xmax-.3,ymin+.3),(xmax-.3,ymax-.3),(xmin+.3,ymax-.3)])
z.clearance=from_mm(.25);z.min_thickness=from_mm(.2)
z.connection.zone_connection=ZoneConnectionStyle.ZCS_THERMAL
z.connection.thermal_spokes.width=from_mm(.3);z.connection.thermal_spokes.gap=from_mm(.25)
items.append(z)
created=b.create_items(items)
b.push_commit(commit,'Route Grove I2C pass-through with shared pull-ups')
b.refill_zones([z.id for z in created if isinstance(z,Zone) and not z.is_rule_area()]);b.save()
report={'kicad':str(version),'api_client':'kicad-python 0.8.0','document':b.document.board_filename,'dimensions_mm':[xmax-xmin,ymax-ymin],'grove_pinout':expected,'grove_centres_mm':[list(pad(ref,'1')) for ref in ['J1','J2']],'track_count':len(b.get_tracks()),'track_layers':sorted({BoardLayer.Name(t.layer) for t in b.get_tracks()}),'ground_plane':'F.Cu','through_connections':'J1.1-J2.1; J1.2-J2.2; J1.3-J2.3; J1.4-J2.4'}
(HERE/'i2cterm_api_validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))

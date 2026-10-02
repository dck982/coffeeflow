# /// script
# dependencies = []
# ///
"""Regenerate the sensors schematics and preliminary BOM; KiCad 10 libraries required.
Run with: uv run generate_schematic.py
The saved KiCad files are editable; regeneration overwrites schematic edits.
"""
import copy, csv, json, math, re, uuid
from pathlib import Path

class Sym(str): pass
def parse(t):
    toks=re.findall(r'"(?:\\.|[^"\\])*"|[^\s()]+|[()]', t)
    stack=[]; root=None
    for tok in toks:
        if tok=='(': stack.append([])
        elif tok==')':
            a=stack.pop()
            if stack: stack[-1].append(a)
            else: root=a
        else:
            if tok.startswith('"'): val=json.loads(tok)
            else:
                try: val=float(tok) if '.' in tok else int(tok)
                except ValueError: val=Sym(tok)
            stack[-1].append(val)
    return root
def dump(a):
    if isinstance(a,list): return '('+' '.join(dump(v) for v in a)+')'
    if isinstance(a,Sym): return str(a)
    if isinstance(a,str): return json.dumps(a,ensure_ascii=False)
    return str(a)
def key(a): return str(a[0]) if isinstance(a,list) and a else ''
def get(a,k): return next((b for b in a if key(b)==k), None)
def uid(name): return str(uuid.uuid5(uuid.NAMESPACE_URL,'coffeeflow/sensors/'+name))
def node(k,*v): return [Sym(k),*v]
def effects(size=1.0,justify=None):
    a=node('effects',node('font',node('size',size,size)))
    if justify: a.append(node('justify',*(Sym(x) for x in justify.split())))
    return a

HERE=Path(__file__).resolve().parent
LIB=Path('/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols')
FP=LIB.parent/'footprints'
cache={}; symbols={}; bom=[]
def library(libid):
    if libid in symbols: return copy.deepcopy(symbols[libid])
    lib,name=libid.split(':')
    if lib not in cache:
        cache[lib]={a[1]:a for a in parse((LIB/(lib+'.kicad_sym')).read_text()) if key(a)=='symbol'}
    def flatten(n):
        a=copy.deepcopy(cache[lib][n]); ex=get(a,'extends')
        if ex:
            parent=flatten(ex[1]); inherited=[p for p in parent[2:] if key(p)!='property']
            props={p[1]:p for p in parent if key(p)=='property'}
            props.update({p[1]:p for p in a if key(p)=='property'})
            a=[Sym('symbol'),n,*inherited,*props.values()]
            for b in a:
                if key(b)=='symbol': b[1]=b[1].replace(ex[1]+'_',n+'_')
        return a
    a=flatten(name); a[1]=libid; symbols[libid]=a
    return copy.deepcopy(a)


ROOT=uid('root');CHILD=uid('interfaces');SHEET=uid('interfaces-sheet')
SHARED={'+5V','+3V3','GND','I2C_SDA','I2C_SCL','CAN_TX','CAN_RX','FLOW_PULSE','VALVE_CMD','HEATER_CMD'}
class Page:
    def __init__(self,root,title,page,path):
        self.root=root;self.title=title;self.page=page;self.path=path;self.items=[];self.libs={};self.n=0
    def add(self,a): self.items.append(a)
    def ident(self): self.n+=1; return uid(self.root+'/'+str(self.n))
    def wire(self,p,q):
        if p==q:return
        self.add(node('wire',node('pts',node('xy',*p),node('xy',*q)),node('stroke',node('width',0),node('type',Sym('default'))),node('uuid',self.ident())))
    def label(self,name,p,angle=0):
        # Local labels attach to the end of each short wire; global labels connect sheets.
        if name in SHARED:
            self.add(node('global_label',name,node('shape',Sym('bidirectional')),node('at',*p,angle),effects(1.0,'left' if angle==0 else 'right'),node('uuid',self.ident()),node('property','Intersheetrefs','${INTERSHEET_REFS}',node('at',*p,angle),node('hide',Sym('yes')),effects())))
        else:
            self.add(node('label',name,node('at',*p,0),effects(1.0,'right bottom' if angle==180 else 'left bottom'),node('uuid',self.ident())))
    def text(self,t,x,y,size=1.25):self.add(node('text',t,node('at',x,y,0),effects(size,'left top'),node('uuid',self.ident())))
    def part(self,libid,ref,val,x,y,nets,fp=None,angle=0,dnp=False,desc='',mpn=''):
        x=round(round(x/1.27)*1.27,4);y=round(round(y/1.27)*1.27,4)
        a=library(libid);self.libs[libid]=a
        if fp is None:fp=get(a,'property') # corrected below
        defaults={p[1]:p[2] for p in a if key(p)=='property'}
        if not isinstance(fp,str):fp=defaults.get('Footprint','')
        if fp:
            l,n=fp.split(':'); f=(FP/(l+'.pretty'))/(n+'.kicad_mod')
            if not f.exists():raise ValueError('Missing footprint '+fp)
        iid=uid(ref); sx=node('symbol',node('lib_id',libid),node('at',x,y,angle),node('unit',1),node('exclude_from_sim',Sym('no')),node('in_bom',Sym('yes' if not ref.startswith('#') else 'no')),node('on_board',Sym('yes' if not ref.startswith('#') else 'no')),node('dnp',Sym('yes' if dnp else 'no')),node('uuid',iid))
        # IC names above bodies; passive and transistor names beside bodies.
        if ref.startswith(('U','J','D')):
            rects=[p for sub in a if key(sub)=='symbol' for p in sub if key(p)=='rectangle']
            top=max([max(get(p,'start')[2],get(p,'end')[2]) for p in rects] or [5])
            rx,ry=x,y-top-6;vx,vy=x,y-top-3
        elif ref.startswith(('R','C')) and angle in (90,270):rx,ry=x,y-4;vx,vy=x,y+4
        else:rx,ry=x+5,y-2;vx,vy=x+5,y+1
        for k,v,px,py,hidden in [('Reference',ref,rx,ry,ref.startswith('#')),('Value',val,vx,vy,ref.startswith('#')),('Footprint',fp,x,y,True),('Datasheet',defaults.get('Datasheet',''),x,y,True),('Description',desc or defaults.get('Description',''),x,y,True),('MPN',mpn or val,x,y,True)]:
            pr=node('property',k,v,node('at',px,py,90 if ref.startswith(('R','C')) and angle in (90,270) else 0),effects(1.1 if k=='Reference' else 1.0, 'left' if not ref.startswith(('U','J','D')) and angle not in (90,270) else None))
            if hidden:pr.append(node('hide',Sym('yes')))
            sx.append(pr)
        pins=[p for sub in a if key(sub)=='symbol' for p in sub if key(p)=='pin']
        done=set()
        for pin in pins:
            num=get(pin,'number')[1];at=get(pin,'at');px,py=at[1:3]
            th=math.radians(angle);p=(round(x+px*math.cos(th)-py*math.sin(th),4),round(y-px*math.sin(th)-py*math.cos(th),4))
            sx.append(node('pin',num,node('uuid',uid(ref+'/'+num))))
            net=nets.get(num)
            if p in done:continue
            done.add(p)
            if net is None:
                self.add(node('no_connect',node('at',*p),node('uuid',self.ident())));continue
            direction=(int(at[3])+angle)%360
            vec={0:(-1,0),90:(0,1),180:(1,0),270:(0,-1)}[direction]
            q=(round(p[0]+vec[0]*5.08,4),round(p[1]+vec[1]*5.08,4))
            self.wire(p,q)
            self.label(net,q,0 if vec[0]>=0 else 180)
        sx.append(node('instances',node('project','sensors',node('path',self.path,node('reference',ref),node('unit',1)))))
        self.add(sx)
        if not ref.startswith('#'):
            bom.append({'Reference':ref,'Value':val,'Footprint':fp,'DNP':'1' if dnp else '0','LCSC':'','MPN':mpn or val,'Description':desc or defaults.get('Description',''),'Sheet':self.page})
        return sx
    def write(self,name):
        a=node('kicad_sch',node('version',20260306),node('generator',Sym('eeschema')),node('generator_version','10.0'),node('uuid',self.root),node('paper','A3'),node('title_block',node('title',self.title),node('date','2026-10-01'),node('rev','A'),node('company','CoffeeFlow'),node('comment',1,'CAN-only / USB flash power OR external 5V')),node('lib_symbols',*self.libs.values()),*self.items,node('sheet_instances',node('path','/',node('page',str(self.page)))),node('embedded_fonts',Sym('no')))
        # One top-level item per line keeps generated diffs readable.
        (HERE/name).write_text('('+dump(a[0])+'\n'+'\n'.join(dump(i) for i in a[1:])+'\n)\n')

p=Page(ROOT,'Sensors - power, ESP32-S3 and USB flashing',1,'/'+ROOT)
q=Page(CHILD,'Sensors - CAN, I2C, flowmeter and SSR outputs',2,'/'+ROOT+'/'+SHEET)
R='Device:R';C='Device:C';RF='Resistor_SMD:R_0603_1608Metric';CF='Capacitor_SMD:C_0603_1608Metric';CB='Capacitor_SMD:C_0805_2012Metric'
def r(page,ref,val,x,y,a,b,angle=0,dnp=False):
    package='1206' if ref=='R12' else '0603'
    footprint='Resistor_SMD:R_1206_3216Metric' if ref=='R12' else RF
    page.part(R,ref,val,x,y,{'1':a,'2':b},footprint,angle,dnp,mpn='Generic '+val+' 1% '+package)
def c(page,ref,val,x,y,a,b,fp=CF):page.part(C,ref,val,x,y,{'1':a,'2':b},fp,mpn='Generic '+val+' X7R >=10V')
def conn(page,ref,val,x,y,nets,size,grove=False):
    fp='Connector:NS-Tech_Grove_1x04_P2mm_Vertical' if grove else f'Connector_JST:JST_XH_B{size}B-XH-A_1x0{size}_P2.50mm_Vertical'
    page.part('Connector_Generic:Conn_01x0'+str(size),ref,val,x,y,{str(i+1):n for i,n in enumerate(nets)},fp,mpn='NS-Tech Grove vertical HY2.0-4P' if grove else f'JST B{size}B-XH-A')

p.text('1. POWER - XH for production OR USB-C for flashing',18,15,1.7)
conn(p,'J1','POWER / 5V RECOM',40,45,['GND','+5V'],2)
p.part('Regulator_Linear:AMS1117-3.3','U2','AMS1117-3.3',105,45,{'1':'GND','2':'+3V3','3':'+5V'})
c(p,'C1','4.7uF',65,83,'+5V','GND',CB)
c(p,'C2','100nF',100,83,'+5V','GND')
p.part('Device:C_Polarized','C3','22uF tantalum / 10V',145,83,{'1':'+3V3','2':'GND'},'Capacitor_Tantalum_SMD:CP_EIA-3528-21_Kemet-B',mpn='22uF 10V solid tantalum, case B')
p.text('C3: solid tantalum for AMS1117 stability.\nProvide copper heatsinking: 0.85W at 500mA; verify enclosure temperature.\nUSB VBUS and J1 feed +5V: NEVER connect both power sources.',18,106)
for i,net in enumerate(['+5V','GND']):p.part('power:PWR_FLAG','#FLG0'+str(i+1),'PWR_FLAG',35+i*45,125,{'1':net},fp='')

p.text('2. ESP32-S3-MINI-1U-N8 - CAN only, no antenna fitted',225,15,1.7)
nets={'3':'+3V3','4':'BOOT_N','8':'FLOW_PULSE','9':'I2C_SDA','10':'I2C_SCL','11':'CAN_TX','12':'CAN_RX','13':'VALVE_CMD','14':'HEATER_CMD','23':'USB_MCU_DM','24':'USB_MCU_DP','41':'STRAP45','44':'STRAP46','45':'EN'}
for n in [1,2,42,43,*range(46,66)]:nets[str(n)]='GND'
p.part('RF_Module:ESP32-S3-MINI-1U','U1','ESP32-S3-MINI-1U-N8',310,73,nets,mpn='ESP32-S3-MINI-1U-N8')
c(p,'C4','10uF',240,125,'+3V3','GND',CB);c(p,'C5','100nF',275,125,'+3V3','GND')
r(p,'R2','10k',350,125,'+3V3','EN');c(p,'C6','1uF',385,125,'EN','GND')
r(p,'R3','10k',235,72,'+3V3','BOOT_N')
r(p,'R23','10k',280,155,'STRAP45','GND')
r(p,'R24','10k',310,155,'STRAP46','GND')
p.part('Switch:SW_Push','SW1','BOOT',235,92,{'1':'BOOT_N','2':'GND'},'Button_Switch_SMD:SW_SPST_TL3342',mpn='E-Switch TL3342F160QG')
p.part('Switch:SW_Push','SW2','RESET',350,155,{'1':'EN','2':'GND'},'Button_Switch_SMD:SW_SPST_TL3342',mpn='E-Switch TL3342F160QG')
p.text('GPIO45/46 low: 3.3V flash / download strap. GPIO3 unused.\nHold BOOT, tap RESET, release BOOT for ROM USB download.\nMINI-1U body: 15.4 x 15.4mm. Wi-Fi and BLE disabled in firmware.',225,179)

p.text('3. USB-C - flashing and power; disconnect XH power before connecting USB',18,147,1.7)
un={'A1':'GND','A12':'GND','B1':'GND','B12':'GND','SH':'GND','A4':'+5V','A9':'+5V','B4':'+5V','B9':'+5V','A5':'USB_CC1','B5':'USB_CC2','A6':'USB_HOST_DP','B6':'USB_HOST_DP','A7':'USB_HOST_DM','B7':'USB_HOST_DM'}
p.part('Connector:USB_C_Receptacle_USB2.0_16P','J2','USB-C FLASH',43,191,un,'Connector_USB:USB_C_Receptacle_GCT_USB4105-xx-A_16P_TopMnt_Horizontal',mpn='GCT USB4105-GF-A')
r(p,'R4','5.1k',90,179,'USB_CC1','GND');r(p,'R5','5.1k',90,208,'USB_CC2','GND')
r(p,'R6','22',155,185,'USB_HOST_DP','USB_MCU_DP',90)
r(p,'R7','22',155,215,'USB_HOST_DM','USB_MCU_DM',90)
p.part('Power_Protection:TPD2E2U06DCK','D1','TPD2E2U06DCKR',225,220,{'1':'USB_HOST_DP','2':'USB_HOST_DM','3':'GND'})
p.text('USB flashing: disconnect J1 power, then connect Mac USB-C.\nProduction: unplug USB-C, then connect J1 to RECOM 5V.\nBoth feed the same +5V rail; no power OR-ing or reverse blocking.',18,245,1.2)
p.text('ESP32 provides its USB data pull-up internally. CC1/CC2: 5.1k to GND.\nRoute USB as a 90-ohm pair; D1 at J2, R6/R7 at U1.\nC1: 4.7uF input bulk to reduce USB plug-in inrush.',18,265,1.1)
# A real hierarchical sheet, globals carry the ten board-wide nets.
p.add(node('sheet',node('at',290,210),node('size',102,28),node('stroke',node('width',0.1524),node('type',Sym('default'))),node('fill',node('color',0,0,0,0)),node('uuid',SHEET),node('property','Sheetname','Sensors interfaces',node('at',290,209,0),effects(1.5,'left bottom')),node('property','Sheetfile','sensors_interfaces.kicad_sch',node('at',290,239,0),effects(1.2,'left top')),node('instances',node('project','sensors',node('path','/'+ROOT,node('page','2'))))))

q.text('4. CAN - TJA1051T/3, 5V bus driver / 3.3V logic',18,15,1.7)
q.part('Interface_CAN_LIN:TJA1051T-3','U4','TJA1051T/3',85,55,{'1':'CAN_TX','2':'GND','3':'+5V','4':'CAN_RX','5':'+3V3','6':'CAN_L','7':'CAN_H','8':'GND'},mpn='TJA1051T/3,118')
conn(q,'J3','CAN BUS',155,50,['CAN_L','CAN_H'],2)
c(q,'C8','100nF',40,92,'+5V','GND');c(q,'C9','100nF',80,92,'+3V3','GND')
r(q,'R11','10k',120,92,'+3V3','CAN_TX')
r(q,'R12','120 / 0.25W',182,86,'CAN_H','CAN_L')
q.text('S/SLNT hard-wired low. R11 keeps TX recessive during reset.\nR12 permanently terminates this bus-end node (120 ohms).\nJ3 has no GND pin: power GND must run alongside CAN pair.',18,112)

q.text('5. SHARED I2C - 3.3V, Grove/HY2.0 cable order',230,15,1.7)
conn(q,'J4','DIMMER / GROVE',378,45,['I2C_SCL','I2C_SDA','+3V3','GND'],4,True)
conn(q,'J5','XDB401 / GROVE',378,85,['I2C_SCL','I2C_SDA','+3V3','GND'],4,True)
r(q,'R13','4.7k',243,47,'+3V3','I2C_SDA');r(q,'R14','4.7k',283,47,'+3V3','I2C_SCL')
q.part('Sensor_Temperature:TMP102xxDRL','U5','TMP102AIDRLR',280,95,{'1':'I2C_SCL','2':'GND','4':'GND','5':'+3V3','6':'I2C_SDA'},mpn='TMP102AIDRLR')
c(q,'C10','100nF',325,115,'+3V3','GND');c(q,'C11','100nF',370,119,'+3V3','GND')
q.text('Grove opening up: left-to-right GND / 3V3 / SDA / SCL.\nNumbered pads: 1=SCL yellow, 2=SDA white, 3=3V3 red, 4=GND black.\nAddresses: dimmer 0x50 / XDB401 0x7F / TMP102 0x48.\nTMP102 ADD0 grounded; ALERT unused. Mount away from LDO heat.\nPull-ups always fitted: 2.35k effective with XDB401 attached.\n100kHz initial bus speed; C11 bypasses connector supply.',230,139,1.1)

q.text('6. DIGMESA - open collector, existing RC filter',18,147,1.7)
conn(q,'J6','DIGMESA / FLOW',48,177,['GND','+5V','FLOW_PULSE'],3)
r(q,'R15','1k',110,174,'+3V3','FLOW_PULSE');c(q,'C12','10nF',155,174,'FLOW_PULSE','GND');c(q,'C13','100nF',190,174,'+5V','GND')
q.text('GPIO4 input; internal pull-up disabled. 1k x 10nF = 10us.\nDigmesa powered from 5V; signal pulled to 3.3V only.\nPin order: GND brown / 5V green / SIGNAL yellow.\nRC discharge speed also depends on sensor/cable impedance.',18,194,1.1)

q.text('7. VALVE SSR - 3.3V signal / 5V supply',230,168,1.7)
conn(q,'J7','M5STACK VALVE SSR',378,186,['GND','+5V','VALVE_CMD'],3)
r(q,'R17','10k',323,190,'VALVE_CMD','GND')
q.text('GPIO9, 3.3V signal / 5V VCC; default LOW.',230,206,1.1)
q.text('8. BOILER SSR - powered 5V output, HIGH command = ON',18,220,1.7)
# GPIO10 drives Q3; Q3 sinks the high-side P-MOSFET gate directly.
q.part('Transistor_FET:2N7002','Q3','2N7002',70,250,{'1':'HEATER_CMD','2':'GND','3':'HEATER_GATE'})
r(q,'R19','100k',25,250,'HEATER_CMD','GND')
q.part('Transistor_FET:AO3401A','Q2','AO3401A',225,245,{'1':'HEATER_GATE','2':'+5V','3':'BOILER_5V'},angle=180)
r(q,'R21','10k',175,268,'+5V','HEATER_GATE')
r(q,'R22','1k',255,268,'BOILER_5V','GND')
conn(q,'J8','BOILER SSR / 5V OUT',288,242,['GND','BOILER_5V'],2)
q.text('J8: SSR (-) -> pin1 GND; SSR (+) -> pin2 switched 5V.\nQ3 pulls Q2 gate LOW; Q2 supplies the SSR current.\nR19/R21/R22 ensure off with GPIO floating or 3V3 absent. Shared GND.\nGate pull-up current about 0.5mA when ON; load <=100mA; no mains.',18,279,1.0)

p.write('sensors.kicad_sch');q.write('sensors_interfaces.kicad_sch')
with (HERE/'sensors_bom.csv').open('w',newline='') as f:
    w=csv.DictWriter(f,lineterminator='\n',fieldnames=['Reference','Value','Footprint','DNP','LCSC','MPN','Description','Sheet']);w.writeheader();w.writerows(bom)
print(f'Generated 2 sheets and {len(bom)} components; LCSC sourcing remains unassigned.')

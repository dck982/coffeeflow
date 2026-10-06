# /// script
# dependencies = []
# ///
"""Regenerate the pass-through schematic and source BOM; PCB is not modified.
Run with: uv run generate_design.py
The saved files are editable; regeneration overwrites schematic edits only.
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
def uid(name): return str(uuid.uuid5(uuid.NAMESPACE_URL,'coffeeflow/i2cterm/'+name))
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


ROOT=uid('root')
SHARED={'+3V3','GND','SCL','SDA','LED_A'}
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
            l,n=fp.split(':'); base = HERE if l == 'I2CTerm_Local' else FP; f=(base/(l+'.pretty'))/(n+'.kicad_mod')
            if not f.exists():raise ValueError('Missing footprint '+fp)
        iid=uid(ref); sx=node('symbol',node('lib_id',libid),node('at',x,y,angle),node('unit',1),node('exclude_from_sim',Sym('no')),node('in_bom',Sym('yes' if not ref.startswith(('#','H')) else 'no')),node('on_board',Sym('yes' if not ref.startswith('#') else 'no')),node('dnp',Sym('yes' if dnp else 'no')),node('uuid',iid))
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
        sx.append(node('instances',node('project','i2cterm',node('path',self.path,node('reference',ref),node('unit',1)))))
        self.add(sx)
        if not ref.startswith('#'):
            bom.append({'Reference':ref,'Value':val,'Footprint':fp,'DNP':'1' if dnp else '0','LCSC':'','MPN':mpn or val,'Description':desc or defaults.get('Description',''),'Sheet':self.page})
        return sx
    def write(self,name):
        a=node('kicad_sch',node('version',20260306),node('generator',Sym('eeschema')),node('generator_version','10.0'),node('uuid',self.root),node('paper','A4'),node('title_block',node('title',self.title),node('date','2026-10-06'),node('rev','A'),node('company','CoffeeFlow'),node('comment',1,'3.3V I2C pull-ups / top-side routing')),node('lib_symbols',*self.libs.values()),*self.items,node('sheet_instances',node('path','/',node('page',str(self.page)))),node('embedded_fonts',Sym('no')))
        # One top-level item per line keeps generated diffs readable.
        (HERE/name).write_text('('+dump(a[0])+'\n'+'\n'.join(dump(i) for i in a[1:])+'\n)\n')



p=Page(ROOT,'Grove I2C pass-through with 3.3V pull-ups',1,'/'+ROOT)
RF='Resistor_SMD:R_0603_1608Metric'
GF='I2CTerm_Local:CONN-TH_HY-4A'
p.text('Grove I2C pass-through / 3.3V pull-ups',25,20,2)
for ref,y in [('J1',55),('J2',85.09)]:
    p.part('Connector_Generic:Conn_01x04',ref,'GROVE VERTICAL',49.53,y,{'1':'SCL','2':'SDA','3':'+3V3','4':'GND'},GF,mpn='HY-4A')
p.part('Device:R','R1','4.7k',105,45,{'1':'SCL','2':'+3V3'},RF,mpn='0603 4.7k 1%')
p.part('Device:R','R2','4.7k',140,45,{'1':'SDA','2':'+3V3'},RF,mpn='0603 4.7k 1%')
p.part('Device:R','R3','1k',185,45,{'1':'+3V3','2':'LED_A'},RF,mpn='0603 1k 1%')
p.part('Device:LED','D1','GREEN',185,80,{'1':'GND','2':'LED_A'},'LED_SMD:LED_0603_1608Metric',angle=90,mpn='KT-0603YG')
for ref,x in [('H1',45),('H2',100)]:
    p.part('Mechanical:MountingHole',ref,'M2.5',x,110,{},'MountingHole:MountingHole_2.7mm_M2.5',mpn='M2.5 / 2.7mm')
p.text('J1/J2: 1=SCL, 2=SDA, 3=3V3, 4=GND.\nBoth ports are wired in parallel, pin for pin.\nOne shared pair of 4.7k pull-ups.\nLED indicates 3.3V power, not I2C traffic.\nTwo M2.5 mounting holes, 2.7mm NPTH.',25,140)
sourcing={'J1':'C722737','J2':'C722737','R1':'C23162','R2':'C23162','R3':'C21190','D1':'C2289'}
for item in p.items:
    if key(item)!='symbol':continue
    props={e[1]:e for e in item if key(e)=='property'}
    ref=props['Reference'][2]
    if ref not in sourcing:continue
    x,y=get(item,'at')[1:3]
    item.append(node('property','LCSC',sourcing[ref],node('at',x,y,0),effects(),node('hide',Sym('yes'))))
    if ref in ('J1','J2'):props['Datasheet'][2]='https://www.lcsc.com/product-detail/C722737.html'
p.write('i2cterm.kicad_sch')
rows=[b for b in bom if b['Reference'] in sourcing]
for b in rows:
    b['LCSC']=sourcing[b['Reference']]
    b['Datasheet']='https://www.lcsc.com/product-detail/C722737.html' if b['Reference'] in ('J1','J2') else ''
with (HERE/'i2cterm_bom.csv').open('w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

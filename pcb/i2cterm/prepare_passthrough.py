# /// script
# dependencies = []
# ///
"""Prepare the saved board for pass-through connectors; route with route_board.py.
Preserves the outline, mounting holes and all other footprints. PCB must be saved
and reloaded in KiCad after this stage. Original files are backed up in .history.
"""
import copy,csv,uuid,shutil
from pathlib import Path
from sexpr import parse,dump,key,get,node,Sym,effects,uid
HERE=Path(__file__).resolve().parent
FPID='I2CTerm_Local:CONN-TH_HY-4A'
J2ID=uid('J2')

def children(a,k):return [e for e in a if key(e)==k]
def properties(a):return {e[1]:e for e in children(a,'property')}
def write(path,tree):path.write_text('('+dump(tree[0])+'\n'+'\n'.join(dump(i) for i in tree[1:])+'\n)\n')
def field(a,k,v,pcb=False):
    prop=properties(a).get(k)
    if prop:prop[2]=v
    else:
        x,y=get(a,'at')[1:3]
        prop=node('property',k,v,node('at',0 if pcb else x,0 if pcb else y,0),effects())
        if pcb:prop.append(node('layer','F.Fab'))
        prop.append(node('hide',Sym('yes')));a.append(prop)

def newuuids(a):
    if not isinstance(a,list):return
    if key(a)=='uuid':a[1]=str(uuid.uuid4())
    for v in a:newuuids(v)

backup=HERE/'.history'/'before-passthrough'
backup.mkdir(parents=True,exist_ok=True)
for name in ['i2cterm.kicad_sch','i2cterm.kicad_pcb','i2cterm_bom.csv','customize-manufacturing.py']:
    if not (backup/name).exists():shutil.copy2(HERE/name,backup/name)
boardpath=HERE/'i2cterm.kicad_pcb';schpath=HERE/'i2cterm.kicad_sch'
board=parse(boardpath.read_text());sch=parse(schpath.read_text())
symbols={properties(s)['Reference'][2]:s for s in children(sch,'symbol')}
assert 'J1' in symbols and 'J2' not in symbols, 'Preparation is a one-time migration; do not rerun it.'
j1=symbols['J1'];j2=copy.deepcopy(j1);newuuids(j2);get(j2,'uuid')[1]=J2ID
# Clone J1 and its four net-labelled wires 30.48mm lower on the schematic.
x,y=get(j1,'at')[1:3];dy=30.48
for prop in children(j2,'property'):get(prop,'at')[2]+=dy
get(j2,'at')[2]+=dy
field(j2,'Reference','J2')
for project in children(get(j2,'instances'),'project'):
    for path in children(project,'path'):get(path,'reference')[1]='J2'
selected=[]
for e in sch:
    if key(e)=='wire':
        pts=children(get(e,'pts'),'xy')
        if all(x-22<=p[1]<=x and y-5.08<=p[2]<=y+12.7 for p in pts):
            c=copy.deepcopy(e);newuuids(c)
            for p in children(get(c,'pts'),'xy'):p[2]+=dy
            selected.append(c)
    if key(e)=='global_label':
        at=get(e,'at')
        if x-22<=at[1]<=x and y-5.08<=at[2]<=y+12.7:
            c=copy.deepcopy(e);newuuids(c);get(c,'at')[2]+=dy
            for p in children(c,'property'):get(p,'at')[2]+=dy
            selected.append(c)
assert len(selected)==8,len(selected)
sch.extend([j2,*selected])
for j in [j1,j2]:
    for k,v in {'Value':'GROVE VERTICAL','Footprint':FPID,'MPN':'HY-4A','LCSC':'C722737','Datasheet':'https://www.lcsc.com/product-detail/C722737.html'}.items():field(j,k,v)
for t in children(sch,'text'):
    if 'square pad 1=SCL' in t[1]:
        t[1]='J1 and J2: 1=SCL, 2=SDA, 3=3V3, 4=GND.\nBoth connectors are wired in parallel, pin for pin.\nR1/R2: one shared pair of 4.7k pull-ups.\nLED indicates 3.3V power, not I2C traffic.\nTwo M2.5 mounting holes, 2.7mm NPTH.'
    if t[1]=='I2C pull-ups / 3.3V only':t[1]='Grove I2C pass-through / 3.3V pull-ups'
get(sch,'title_block')[1][1]='Grove I2C pass-through with 3.3V pull-ups'
oldj=next(f for f in children(board,'footprint') if properties(f)['Reference'][2]=='J1')
net_by_pin={p[1]:get(p,'net') for p in children(oldj,'pad')}
assert set(net_by_pin)=={'1','2','3','4'}
lib=parse((HERE/'I2CTerm_Local.pretty/CONN-TH_HY-4A.kicad_mod').read_text())
for ref,row in [('J1',53.3),('J2',66.7)]:
    f=copy.deepcopy(lib);f[0]=Sym('footprint');f[1]=FPID
    f=[e for e in f if key(e) not in ('property','uuid','at') and not (key(e)=='fp_text' and str(e[1]) in ('reference','value'))]
    f.extend([node('uuid',get(oldj,'uuid')[1] if ref=='J1' else J2ID),node('at',58.5,row,0),node('path',get(oldj,'path')[1] if ref=='J1' else get(oldj,'path')[1].rsplit('/',1)[0]+'/'+J2ID),node('sheetname',''),node('sheetfile','i2cterm.kicad_sch')])
    for k,p in properties(oldj).items():f.append(copy.deepcopy(p))
    for k,v in {'Reference':ref,'Value':'GROVE VERTICAL','MPN':'HY-4A','LCSC':'C722737','Datasheet':'https://www.lcsc.com/product-detail/C722737.html'}.items():field(f,k,v,True)
    for p in children(f,'property'):
        get(p,'at')[1:]=[0,0,0]
    for pad in children(f,'pad'):pad.append(copy.deepcopy(net_by_pin[str(pad[1])]))
    if ref=='J1':board[board.index(oldj)]=f
    else:board.append(f)
# Remove obsolete routes; IPC reconstructs all four through buses and pull-up taps.
board[:]=[e for e in board if key(e) not in ('segment','via','zone')]
write(schpath,sch);write(boardpath,board)
with (HERE/'i2cterm_bom.csv').open(newline='') as s:r=csv.DictReader(s);fields=r.fieldnames;rows=list(r)
newrow=copy.deepcopy(next(r for r in rows if r['Reference']=='J1'));newrow['Reference']='J2';rows.append(newrow)
for r in rows:
    if r['Reference'] in ('J1','J2'):r.update(Value='GROVE VERTICAL',Footprint=FPID,LCSC='C722737',MPN='HY-4A',Datasheet='https://www.lcsc.com/product-detail/C722737.html')
with (HERE/'i2cterm_bom.csv').open('w',newline='') as s:w=csv.DictWriter(s,fieldnames=fields);w.writeheader();w.writerows(rows)
print('Prepared J1/J2 vertical Grove C722737, preserving outline and mounting-hole positions.')

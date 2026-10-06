# /// script
# dependencies = []
# ///
"""Bootstrap the i2cterm project; KiCad 10 libraries required.
Run with: uv run generate_design.py, then open the PCB and run route_board.py
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
def uid(name): return str(uuid.uuid5(uuid.NAMESPACE_URL,'coffeeflow/i2cterm/'+name))
def node(k,*v): return [Sym(k),*v]
def effects(size=1.0,justify=None):
    a=node('effects',node('font',node('size',size,size)))
    if justify: a.append(node('justify',*(Sym(x) for x in justify.split())))
    return a

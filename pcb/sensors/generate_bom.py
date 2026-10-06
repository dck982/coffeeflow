# /// script
# dependencies = []
# ///
"""Rebuild sourced BOMs from saved designs without changing KiCad documents.

Run: uv run sensors/generate_bom.py
Selections and dated catalog evidence live in sensors_sourcing.json.
This exports BOMs only, not fabrication or placement files.
"""
import csv
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from generate_manufacturing import children, first, parse_sexpr


def write_csv(path, fields, rows):
    with path.open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)


def main():
    sourcing = json.loads((HERE / 'sensors_sourcing.json').read_text())
    selections = sourcing['PartsBySymbolUUID']
    symbols = {}
    sources = [HERE / 'sensors.kicad_pcb', HERE / 'sensors_sourcing.json', Path(__file__)]
    for path in sorted(HERE.glob('*.kicad_sch')):
        sources.append(path)
        for symbol in children(parse_sexpr(path.read_text()), 'symbol'):
            props = {p[1]: p[2] for p in children(symbol, 'property')}
            if props['Reference'].startswith('#'):
                continue
            symbols[first(symbol, 'uuid')[1]] = (symbol, props, path.name)

    board = parse_sexpr(sources[0].read_text())
    rows, seen, seen_refs = [], set(), set()
    for fp in children(board, 'footprint'):
        if not any(p[1] for p in children(fp, 'pad')):
            continue  # Mechanical mounting holes have no numbered electrical pads.
        props = {p[1]: p[2] for p in children(fp, 'property')}
        uid = first(fp, 'path')[1].split('/')[-1]
        symbol, sch_props, sheet = symbols[uid]
        part = selections[uid]
        ref = props['Reference']
        if uid in seen or not ref or ref in seen_refs:
            raise ValueError(f'Duplicate or empty electrical reference: {ref}')
        if sch_props['Reference'] != part['SchematicReference']:
            raise ValueError(f'{ref}: schematic reference changed; review sourcing')
        if props['Value'] != part['ExpectedValue'] or fp[1] != part['ExpectedFootprint']:
            raise ValueError(f'{ref}: value/footprint changed; review sourcing')
        if sch_props['Footprint'] != fp[1]:
            raise ValueError(f'{ref}: PCB/schematic footprint mismatch')
        if part['Assembly'] != 'Manual' and not re.fullmatch(r'C\d+', part['LCSC']):
            raise ValueError(f'{ref}: missing JLCPCB part')
        dnp = bool(children(fp, 'dnp') and first(fp, 'dnp')[1] == 'yes')
        if dnp != (first(symbol, 'dnp')[1] == 'yes'):
            raise ValueError(f'{ref}: PCB/schematic DNP mismatch')
        rows.append(dict(Reference=ref, Value=props['Value'], Footprint=fp[1],
                         DNP=str(dnp), LCSC=part['LCSC'], MPN=part['MPN'],
                         Description=sch_props.get('Description', ''), Sheet=sheet,
                         SchematicReference=sch_props['Reference'],
                         Manufacturer=part['Manufacturer'], JLCClass=part['JLCClass'],
                         Assembly=part['Assembly'], CheckedDate=sourcing['CheckedDate'],
                         SourceURL=('https://jlcpcb.com/partdetail/x/' + part['LCSC'])
                         if part['LCSC'] else '', Notes=part['Notes']))
        seen.add(uid)
        seen_refs.add(ref)
    if seen != set(symbols) or seen != set(selections):
        raise ValueError('PCB, schematic and sourcing component UUID sets differ')

    def natural(ref):
        return [int(s) if s.isdigit() else s for s in re.split(r'(\d+)', ref)]
    rows.sort(key=lambda row: natural(row['Reference']))
    fields = list(rows[0])
    manual = [r for r in rows if r['Assembly'] == 'Manual' and r['DNP'] == 'False']
    grouped = {}
    assembly_aliases = sourcing.get('AssemblyReferenceAliases', {})
    assembly_refs = set()
    for row in rows:
        if row['DNP'] == 'True' or row['Assembly'] == 'Manual':
            continue
        assembly_ref = assembly_aliases.get(row['Reference'], row['Reference'])
        if not re.fullmatch(r'[A-Za-z0-9_-]+', assembly_ref) or assembly_ref in assembly_refs:
            raise ValueError(f'Invalid or duplicate assembly reference: {assembly_ref!r}')
        assembly_refs.add(assembly_ref)
        key = (row['LCSC'], row['Footprint'])
        if key not in grouped:
            grouped[key] = {'Comment': row['MPN'], 'Designator': [],
                            'Footprint': row['Footprint'].split(':', 1)[-1],
                            'LCSC Part #': row['LCSC']}
        grouped[key]['Designator'].append(assembly_ref)
    upload = [dict(r, Designator=','.join(r['Designator'])) for r in grouped.values()]
    write_csv(HERE / 'sensors_bom.csv', fields, rows)
    write_csv(HERE / 'sensors_jlcpcb_bom.csv',
              ['Comment', 'Designator', 'Footprint', 'LCSC Part #'], upload)
    write_csv(HERE / 'sensors_manual_bom.csv', fields, manual)
    report = {'Scope': 'BOM only; not a manufacturing release or placement validation',
              'CheckedDate': sourcing['CheckedDate'], 'Components': len(rows),
              'LibraryCounts': dict(Counter(r['JLCClass'] for r in rows)),
              'JLCUniqueParts': len(upload), 'ManualReferences': [r['Reference'] for r in manual],
              'AssemblyReferenceAliases': assembly_aliases,
              'ReferenceAliases': {r['SchematicReference']: r['Reference'] for r in rows
                                   if r['SchematicReference'] != r['Reference']},
              'SourceSHA256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}}
    (HERE / 'sensors_bom_validation.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'SourceSHA256'}, indent=2))


if __name__ == '__main__':
    main()

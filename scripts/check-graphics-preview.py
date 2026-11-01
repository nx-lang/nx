"""Deterministic end-to-end preview checks and parameter/measurement walkthrough fixtures.
Build nx-graphics-preview with --features preview first. No model service or network is used.
"""
import argparse, json, pathlib, subprocess, tempfile
from jsonschema import Draft202012Validator
parser=argparse.ArgumentParser()
parser.add_argument('--binary',default='target/debug/nx-graphics-preview')
parser.add_argument('--write-fixtures',action='store_true')
args=parser.parse_args()
root=pathlib.Path(__file__).resolve().parent.parent
binary=(root/args.binary).resolve()
validator=Draft202012Validator(json.loads((root/'schemas/graphics-preview.schema.json').read_text()))
def run(*flags,success=True):
    p=subprocess.run([str(binary),*map(str,flags)],capture_output=True,text=True)
    assert (p.returncode==0)==success,(p.stdout,p.stderr)
    result=json.loads(p.stdout)
    if not success: validator.validate(result)
    return result
def png_pixel(path,x,y):
    # Minimal test-only decoder for Skia's noninterlaced 8-bit RGB/RGBA PNG output.
    import struct,zlib
    data=pathlib.Path(path).read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n'
    offset=8;compressed=b''
    while offset<len(data):
        length=struct.unpack('>I',data[offset:offset+4])[0];kind=data[offset+4:offset+8];chunk=data[offset+8:offset+8+length];offset+=length+12
        if kind==b'IHDR': width,height,depth,color,_,_,interlace=struct.unpack('>IIBBBBB',chunk)
        if kind==b'IDAT': compressed+=chunk
    assert depth==8 and color in (2,6) and interlace==0
    channels=4 if color==6 else 3;stride=width*channels;raw=zlib.decompress(compressed);previous=bytearray(stride);offset=0
    for row_index in range(height):
        mode=raw[offset];row=bytearray(raw[offset+1:offset+1+stride]);offset+=stride+1
        for i in range(stride):
            left=row[i-channels] if i>=channels else 0;above=previous[i];corner=previous[i-channels] if i>=channels else 0
            estimate=left+above-corner
            choices=(abs(estimate-left),abs(estimate-above),abs(estimate-corner))
            paeth=(left,above,corner)[choices.index(min(choices))]
            row[i]=(row[i]+(0,left,above,(left+above)//2,paeth)[mode])%256
        if row_index==y: return tuple(row[x*channels:x*channels+3])
        previous=row
    raise AssertionError('pixel outside image')

def measure(label):
    return run('--measure-text',json.dumps(dict(text=label,family='Fixture',size=18,weight=400,italic=False)))['metrics']
with tempfile.TemporaryDirectory(prefix='nx-preview-') as d:
    d=pathlib.Path(d); entry=d/'main.nx';output=d/'preview'
    for flags in [('--widht','100'),('--width',),('--scale=2','--unknown=1')]:
        run(*flags,success=False)
    for change,reason in [({'size':0},'font size'),({'weight':0},'font weight'),({'text':'a\nb'},'multiline'),({'letterSpacing':5},'unknown field')]:
        request=dict(text='a',family='Fixture',size=18,weight=400,italic=False);request.update(change)
        failure=run('--measure-text',json.dumps(request),success=False)
        assert reason in failure['diagnostics'][0]['message'],failure
    font_manifest=root/'crates/nx-graphics-skia/fixtures/fonts/manifest.json'
    supplied=run('--fonts',font_manifest,'--measure-text',json.dumps(dict(text='a',family='Fixture',size=18,weight=400,italic=False)))
    assert supplied['renderer'].startswith('skia-')

    original=(root/'examples/graphics/main.nx').read_text()
    revisions=[]
    for count,label in [(5,'NX graphics'),(7,'Measured seven-tip revision')]:
        metrics=measure(label);x=(400-metrics['advance'])/2
        source=original.replace('pointCount=5 rotation=-90',f'pointCount={count} rotation=-90').replace('<g.Text x=250 y=165',f'<g.Text x={x} y=225').replace('>NX graphics</g.Text>',f'>{label}</g.Text>')
        entry.write_text(source)
        result=run('--entry',entry,'--output',output,'--width',400,'--height',240)
        manifest=json.loads(pathlib.Path(result['manifest']).read_text())
        validator.validate(manifest)
        schema=json.loads((root/'schemas/graphics-preview.schema.json').read_text())
        assert set(schema['oneOf'][0]['required'])<=manifest.keys()
        assert png_pixel(result['image'],70,160)==(255,255,255)
        assert png_pixel(result['image'],100,160)==(37,99,235)
        scene=manifest['scene']
        def check_node(node):
            assert set(schema['$defs']['node']['required'])<=node.keys()
            for child in node['children']: check_node(child)
        check_node(scene['root'])
        label_node=scene['root']['children'][-1]
        assert abs(label_node['text']['x']-x)<1e-9
        assert abs(label_node['text']['run']['advance']-metrics['advance'])<1e-9
        revisions.append(result['revision'])
        assert run('--inspect',result['manifest'],'--revision',result['revision'],'--id',label_node['id'])['id']==label_node['id']
        run('--inspect',result['manifest'],'--revision','old','--id',label_node['id'],success=False)
        dest=root/'crates/nx-graphics-skia/fixtures/revisions'/str(count)
        manifest['image']='image.png'
        if args.write_fixtures:
            dest.mkdir(parents=True,exist_ok=True)
            (dest/'main.nx').write_text(source)
            (dest/'image.png').write_bytes(pathlib.Path(result['image']).read_bytes())
            manifest['image']='image.png'
            (dest/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        else:
            message='Graphics walkthrough fixtures changed; run python3 scripts/check-graphics-preview.py --write-fixtures'
            assert (dest/'main.nx').read_text()==source,message
            assert (dest/'image.png').read_bytes()==pathlib.Path(result['image']).read_bytes(),message
            assert json.loads((dest/'manifest.json').read_text())==manifest,message
    assert revisions[0]!=revisions[1]
    previous=(output/'current.json').read_bytes()
    for source in [
        'import "@nx/math" as m\nlet root() = { m.sqrt(-1.0) }',
        'import "@nx/math" as m\nlet root() = { m.sqrt(1.0e309) }',
        'import "@nx/math" as m\nlet root() = { m.hypot(1.7e308,1.7e308) }',
        'import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.Rect x=0 y=0 width=100 height=100/>><g.Rectangle width=10 height=10/><g.Path data="M 0 0 L"/></g.Drawing>',
        'import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.Rect x=0 y=0 width=100 height=100/>><g.Text fontFamily="Fixture" text={"a\nb"}/></g.Drawing>',
        'import "@nx/graphics" as g\nexternal component <Rectangle width:float64 height:float64/>\nlet root() = <g.Drawing viewBox=<g.Rect x=0 y=0 width=100 height=100/>><Rectangle width=10 height=10/></g.Drawing>',
    ]:
        entry.write_text(source)
        try: error=run('--entry',entry,'--output',output,success=False)
        except AssertionError: print('Accepted source:',repr(source));raise
        assert error['diagnostics']
        assert error['requestRevision']
        if 'm.sqrt(-1' in source: assert error['diagnostics'][0]['code']=='nx-math-domain'
        if 'm.sqrt(1.0e309)' in source: assert error['diagnostics'][0]['code']=='nx-math-non-finite-input'
        if 'm.hypot' in source: assert error['diagnostics'][0]['code']=='nx-math-overflow'
        assert (output/'current.json').read_bytes()==previous

    prefix='import "@nx/graphics" as g\nlet root() = <g.Drawing viewBox=<g.Rect x=0 y=0 width=100 height=100/>>'
    gradient='<g.Paint.linearGradient x1=0 y1=0 x2=100 y2=0 stops={<g.GradientStop offset=0 color=#ff0000/><g.GradientStop offset=1 color=#0000ff00/>}/>'
    entry.write_text(prefix+'<g.Rectangle width=100 height=100 fill='+gradient+'/></g.Drawing>')
    limits=d/'gradient-limits.json';limits.write_text(json.dumps({'gradient_stops':1}))
    error=run('--entry',entry,'--output',output,'--limits',limits,success=False)
    assert 'gradient_stops' in error['diagnostics'][0]['message'],error
    assert (output/'current.json').read_bytes()==previous
    for bad in [gradient.replace('x2=100','x2=0'),gradient.replace('offset=1','offset=-1')]:
        entry.write_text(prefix+'<g.Rectangle width=100 height=100 fill='+bad+'/></g.Drawing>')
        assert run('--entry',entry,'--output',output,success=False)['diagnostics']
        assert (output/'current.json').read_bytes()==previous
    for helper,reason in [
        ('<g.Arrow x1=0 y1=0 x2=0 y2=0/>','endpoints coincide'),
        ('<g.Arrow x1=0 y1=0 x2=10 y2=0 headLength=11/>','exceeds endpoint distance'),
        ('<g.RegularPolygon cx=0 cy=0 radius=1 pointCount=2/>','pointCount'),
        ('<g.Star cx=0 cy=0 outerRadius=1 innerRadius=2 pointCount=5/>','innerRadius exceeds'),
        ('<g.Arc cx=0 cy=0 radius=1 sweepAngle=361/>','sweepAngle'),
        ('<g.RingSector cx=0 cy=0 outerRadius=1 innerRadius=2/>','innerRadius exceeds'),
    ]:
        entry.write_text(prefix+helper+'</g.Drawing>')
        failure=run('--entry',entry,'--output',output,success=False)
        assert failure['diagnostics'][0]['code']==('constraint-violation' if reason in ['pointCount','sweepAngle'] else 'graphics-helper-invalid'),failure
        assert reason in failure['diagnostics'][0]['message'],failure
        assert (output/'current.json').read_bytes()==previous
    entry.write_text(prefix+'<g.RegularPolygon cx=0 cy=0 radius=1.0e309 pointCount=3/></g.Drawing>')
    failure=run('--entry',entry,'--output',output,success=False)
    assert failure['diagnostics'][0]['code']=='constraint-violation',failure
    assert 'radius' in failure['diagnostics'][0]['message'],failure
    for helper,parameter in [
        ('<g.Arc cx=0 cy=0 radius={1.0e309 - 1.0e309}/>','radius'),
        ('<g.Arc cx=1.0e309 cy=0 radius=1/>','cx'),
        ('<g.RingSector cx=0 cy=0 outerRadius=1.0e309/>','outerRadius'),
        ('<g.RingSector cx=0 cy=0 outerRadius={1.0e309 - 1.0e309}/>','outerRadius'),
        ('<g.Star cx=0 cy=0 outerRadius={1.0e309 - 1.0e309} innerRadius=1 pointCount=5/>','outerRadius'),
        ('<g.Arrow x1=1.0e309 y1=0 x2=10 y2=0/>','x1'),
    ]:
        entry.write_text(prefix+helper+'</g.Drawing>');failure=run('--entry',entry,'--output',output,success=False)
        assert failure['diagnostics'][0]['code']=='constraint-violation',failure
        assert parameter in failure['diagnostics'][0]['message'],failure
    entry.write_text(prefix+'<g.Rectangle width={1.0e309 - 1.0e309} height=10/></g.Drawing>')
    failure=run('--entry',entry,'--output',output,success=False)
    assert failure['diagnostics'][0]['code']=='constraint-violation',failure
    limits=d/'limits.json';limits.write_text('{"nodes":5}')
    entry.write_text(prefix+'<g.Rectangle width=10 height=10/></g.Drawing>')
    run('--entry',entry,'--output',d/'partial-limits','--limits',limits)
    imported=d/'lib.nx';entry.write_text('import "./lib" as lib\nlet root() = { lib.bad() }')
    failures=[]
    for number in [-1,-2]:
        imported.write_text(f'import "@nx/math" as math\nexport let bad() = {{ math.sqrt({number}.0) }}')
        failures.append(run('--entry',entry,'--output',output,success=False)['requestRevision'])
    assert failures[0]!=failures[1]
    hashes=[]
    for name in ['hash-a','hash-b']:
        folder=d/name;folder.mkdir()
        source=folder/'main.nx';source.write_text('import "@nx/math" as m\nlet root() = { m.sqrt(-1.0) }')
        for spelling in [str(folder),str(folder)+'/.' ]:
            hashes.append(run('--entry',source,'--source-root',spelling,'--output',d/'hash-out',success=False)['requestRevision'])
    assert len(set(hashes))==1,hashes
    entry.write_text(prefix+'<g.Rectangle width=10 height=10/></g.Drawing>')
    for array in ['[]','[1]']:
        limits.write_text(array)
        failure=run('--entry',entry,'--output',output,'--limits',limits,success=False)
        assert 'JSON object' in failure['diagnostics'][0]['message'],failure
    failure=run('--measure-text','["a","Fixture",18,400,false]',success=False)
    assert 'JSON object' in failure['diagnostics'][0]['message'],failure
    assert not list(output.glob('.stage-*'))
print('Preview, font measurement, revisions, stale inspection, spoof rejection and failure-safe publication passed.')

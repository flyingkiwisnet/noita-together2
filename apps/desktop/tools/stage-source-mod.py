"""Stage current mod sources and locally built DLLs; never install or approve."""
from pathlib import Path, PurePosixPath
import argparse, json, shutil, struct

ROOT=Path(__file__).resolve().parents[3]

def local_path(value):
    parts=PurePosixPath(value)
    if parts.is_absolute() or not parts.parts or any(p in ('..','.') for p in parts.parts) or '\\' in value or ':' in value:
        raise ValueError('Unsafe source-layout path')
    return parts

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native',default='build/native-source',help='Locally compiled normal native output')
    parser.add_argument('--output',default='build/source-mod',help='New staging directory inside the repository')
    args=parser.parse_args()
    out=(ROOT/args.output).resolve()
    if not out.is_relative_to(ROOT) or out==ROOT or out.exists():
        raise ValueError('Choose a fresh staging directory within this repository')
    native=(ROOT/args.native).resolve()
    layout=json.loads((Path(__file__).parent/'source-mod-layout.json').read_text(encoding='utf-8'))
    selected=[]
    seen=set()
    for row in layout['files']:
        path=local_path(row['path'])
        if not str(path).startswith('mods/noita_together2/') or str(path) in seen:
            raise ValueError('Unsafe or duplicate mod target')
        seen.add(str(path))
        if 'native' in row:
            if row['native'] not in ['nt2_g3_ipc.dll','nt2_g3_transport.dll','nt2_g3_social.dll']:
                raise ValueError('Unknown native source output')
            source=native/row['native']
            image=source.read_bytes()
            pe=struct.unpack_from('<I',image,60)[0]
            if image[pe:pe+4]!=b'PE\0\0' or struct.unpack_from('<H',image,pe+4)[0]!=0x14c:
                raise ValueError('Expected a source-built Windows x86 native image')
        else:
            source=ROOT/str(local_path(row['source']))
            if not source.resolve().is_relative_to(ROOT) or source.is_symlink() or not source.is_file():
                raise ValueError('Missing or redirected mod source')
        selected.append((source,out/str(path)))
    # Validate every input before creating the fresh staging tree.
    for source,target in selected:
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(source,target)
    print(json.dumps({'staged':len(selected),'path':str(out),'approved':False,'installed':False}))

if __name__=='__main__':
    main()

# Usage: python3 scripts/prd581-object-cache.py <perry> <scratch-dir>
import os, re, subprocess, sys
from pathlib import Path
compiler, scratch = map(Path,sys.argv[1:])
scratch.mkdir(parents=True,exist_ok=True)
source=scratch/'main.ts'
source.write_text('const obj: any = { x: 42 }; console.log(obj.x);\n')
copy=scratch/'perry-copy'
subprocess.run(['cp','--reflink=auto',str(compiler),str(copy)],check=True)
with copy.open('ab') as f: f.write(b'PRD581_DRIVER_ONLY_BUILD_PROBE')
ids=[]
for n,binary in enumerate([compiler,copy]):
 env={**os.environ,'PERRY_NO_TELEMETRY':'1','PERRY_CACHE_DEBUG_KEY':'1'}
 r=subprocess.run([str(binary),'compile',str(source),'--no-link','--no-auto-optimize','--cache-dir',str(scratch/'cache'),'-o',str(scratch/f'out-{n}.o')],env=env,capture_output=True,text=True)
 (scratch/f'compile-{n}.log').write_text(r.stdout+r.stderr)
 assert r.returncode==0,r.stderr[-2000:]
 m=re.search(r'cache-key-field (?:codegen_build_id|build_id)="([^\"]+)"',r.stderr)
 assert m,'missing module-cache fingerprint observation'
 ids.append(m[1])
 assert (scratch/f'out-{n}.o').is_file()
assert ids[0]==ids[1],f'non-codegen executable change invalidated objects: {ids}'
objects=list((scratch/'cache'/'objects').rglob('*.o'))
assert len(objects)==1,f'expected one reused module object, got {len(objects)}'
assert (scratch/'out-0.o').read_bytes()==(scratch/'out-1.o').read_bytes()
print(f'PASS: changed executable bytes retain codegen ID {ids[0]} and one identical cached object')

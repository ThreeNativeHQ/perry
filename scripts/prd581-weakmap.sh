#!/usr/bin/env bash
# Usage: scripts/prd581-weakmap.sh [before-executable after-executable]
# Default: compile current source, then run three alternating Perry/Node pairs.
set -euo pipefail
cd "$(dirname "$0")/.."
export LLVM_SYS_221_PREFIX=${LLVM_SYS_221_PREFIX:-/home/joao/.cache/perry-spike/llvm22/prefix}
export PERRY_NO_TELEMETRY=1 CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4
export PERRY_RUNTIME_DIR="$PWD/target/release"
result=${PRD581_RESULT_DIR:-$(mktemp -d /tmp/prd581-weakmap.XXXXXX)}
mkdir -p "$result"
node_bin=${NODE20:-$(command -v node)}
[[ $("$node_bin" --version) == v20.* ]] || { echo 'NODE20 must point to Node v20' >&2; exit 1; }
esbuild=${ESBUILD:-/home/joao/projects/threenative/threenative-engine/.claude/worktrees/ne-compile-async/node_modules/.pnpm/esbuild@0.27.0/node_modules/esbuild/bin/esbuild}
"$esbuild" test-files/test_prd581_weakmap.ts --format=cjs --platform=node --outfile="$result/ops.cjs" </dev/null
if [[ $# == 0 ]]; then
    timeout 690 target/release/perry compile test-files/test_prd581_weakmap.ts \
        -o "$result/perry" --no-auto-optimize --no-cache --cache-dir "$result/cache" </dev/null >"$result/compile.log" 2>&1
    set -- "$result/perry"
fi
[[ $# == 1 || $# == 2 ]] || { echo 'Expected zero, one, or two executable paths' >&2; exit 1; }
python3 - "$result" "$node_bin" "${PRD581_CPU:-}" "$@" <<'PY'
import hashlib, json, math, os, pathlib, statistics, subprocess, sys
out=pathlib.Path(sys.argv[1]); node=sys.argv[2]
cpu=int(sys.argv[3]) if sys.argv[3] else min(os.sched_getaffinity(0))
executables=dict(zip(('before','after'),sys.argv[4:]))
records={k:[] for k in (*executables,'node')}; loads=[]
def run(label, command, repetition):
    load=pathlib.Path('/proc/loadavg').read_text().strip()
    loads.append(dict(run=repetition,label=label,cpu=cpu,load=load))
    result=subprocess.run(['taskset','-c',str(cpu),*command],stdin=subprocess.DEVNULL,
                          capture_output=True,text=True,timeout=690,check=False)
    (out/f'{label}-{repetition}.txt').write_text(result.stdout)
    (out/f'{label}-{repetition}.stderr').write_text(result.stderr)
    result.check_returncode()
    values={}; durations={}
    for line in result.stdout.splitlines():
        parts=line.split()
        if parts and parts[0]=='op':
            _,name,ns,checksum=parts
            ns=float(ns); checksum=float(checksum)
            if name in values or not math.isfinite(ns) or ns<=0 or not math.isfinite(checksum):
                raise ValueError(f'invalid row: {line}')
            values[name]=(ns,checksum)
        elif parts and parts[0]=='sample':
            _,name,ms,count=parts
            if float(ms)<50 or float(count)<=0: raise ValueError(f'short sample: {line}')
            durations[name]=float(ms)
    if not values or values.keys()!=durations.keys(): raise ValueError('missing ops or duration evidence')
    records[label].append(values)
for repetition in range(1,4):
    # Alternate A/B order to counter temperature/load drift; Node runs between them.
    order=list(executables)
    if repetition%2==0: order.reverse()
    run(order[0],[executables[order[0]]],repetition)
    run('node',[node,str(out/'ops.cjs')],repetition)
    if len(order)==2: run(order[1],[executables[order[1]]],repetition)
reference=records['node'][0]
for label,runs in records.items():
    for run_values in runs:
        if run_values.keys()!=reference.keys(): raise ValueError(f'{label}: different op set')
        for name,(_,checksum) in run_values.items():
            if not math.isclose(checksum,reference[name][1],rel_tol=1e-10,abs_tol=1e-8):
                raise ValueError(f'{label}: checksum mismatch {name}: {checksum} vs {reference[name][1]}')
tables={}; means={}
for label in executables:
    rows=[]
    print(f'\n{label}: median of three, CPU {cpu}, Node {subprocess.check_output([node,"--version"],text=True).strip()}')
    print('| op | perry ns | node ns | ratio |\n|---|---:|---:|---:|')
    for name in reference:
        perry=statistics.median(r[name][0] for r in records[label])
        v8=statistics.median(r[name][0] for r in records['node'])
        rows.append(dict(op=name,perry_ns=perry,node_ns=v8,ratio=perry/v8))
        print(f'| {name} | {perry:.3f} | {v8:.3f} | {perry/v8:.3f} |')
    means[label]=math.exp(statistics.mean(math.log(r['ratio']) for r in rows))
    print(f'Geometric mean ratio: {means[label]:.3f}')
    tables[label]=rows
metadata=dict(cpu=cpu,loads=loads,tables=tables,geometric_means=means,
              source_sha256=hashlib.sha256(pathlib.Path('test-files/test_prd581_weakmap.ts').read_bytes()).hexdigest(),
              executable_sha256={k:hashlib.sha256(pathlib.Path(v).read_bytes()).hexdigest() for k,v in executables.items()},
              raw=records)
(out/'results.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(f'\nResults and load averages: {out}/results.json')
PY

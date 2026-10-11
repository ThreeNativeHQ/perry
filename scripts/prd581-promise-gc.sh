#!/usr/bin/env bash
# Usage: scripts/prd581-promise-gc.sh [runs]
# Compiles test-files/test_prd581_promise_gc.ts with the release perry and
# checks the output of N default-GC runs against Node (B19 promise-pump GC).
set -euo pipefail
cd "$(dirname "$0")/.."
export LLVM_SYS_221_PREFIX=${LLVM_SYS_221_PREFIX:-/home/joao/.cache/perry-spike/llvm22/prefix}
export PERRY_NO_TELEMETRY=1 PERRY_RUNTIME_DIR="$PWD/target/release"
runs=${1:-5}
out=$(mktemp -d "${TMPDIR:-/tmp}/prd581-promise-gc.XXXXXX")
src=test-files/test_prd581_promise_gc.ts
esbuild=${ESBUILD:-/home/joao/projects/threenative/threenative-engine/.claude/worktrees/ne-compile-async/node_modules/.pnpm/esbuild@0.27.0/node_modules/esbuild/bin/esbuild}
"$esbuild" "$src" --format=cjs --platform=node --outfile="$out/ref.cjs" </dev/null >/dev/null 2>&1
want=$(node "$out/ref.cjs" </dev/null)
timeout 690 target/release/perry compile "$src" -o "$out/perry" --no-auto-optimize --no-cache \
    --cache-dir "$out/cache" </dev/null >"$out/compile.log" 2>&1
for i in $(seq "$runs"); do
    got=$(timeout 120 "$out/perry" </dev/null 2>"$out/stderr.$i") || { echo "run $i: exit $? (want 0)"; tail -3 "$out/stderr.$i"; exit 1; }
    [[ "$got" == "$want" ]] || { echo "run $i: got '$got', want '$want'"; exit 1; }
done
echo "OK: $runs runs match node: $want"

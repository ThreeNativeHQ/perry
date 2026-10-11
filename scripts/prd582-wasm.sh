#!/usr/bin/env bash
# Usage: scripts/prd582-wasm.sh [test-files/test_prd582_*.ts ...]
# Compiles each file with the release perry `--target wasm`, runs it under
# Node's WebAssembly and checks stdout against Node running the same source.
# A module that does not validate fails the compile (PERRY_WASM_INVALID).
set -euo pipefail
cd "$(dirname "$0")/.."
export PERRY_NO_TELEMETRY=1 NO_COLOR=1
unset FORCE_COLOR
esbuild=${ESBUILD:-/home/joao/projects/threenative/threenative-engine/.claude/worktrees/ne-compile-async/node_modules/.pnpm/esbuild@0.27.0/node_modules/esbuild/bin/esbuild}
out=$(mktemp -d "${TMPDIR:-/tmp}/prd582-wasm.XXXXXX")
[[ $# -gt 0 ]] || set -- test-files/test_prd582_*.ts
perry=${PERRY:-target/release/perry}
fail=0
for src in "$@"; do
    name=$(basename "$src" .ts)
    "$esbuild" "$src" --format=cjs --platform=node --outfile="$out/$name.cjs" </dev/null >/dev/null 2>&1
    want=$(node "$out/$name.cjs" </dev/null)
    if ! timeout 300 "$perry" compile "$src" --target wasm -o "$out/$name.html" \
        </dev/null >"$out/$name.compile.log" 2>&1; then
        echo "FAIL $name: compile"; tail -3 "$out/$name.compile.log"; fail=1; continue
    fi
    got=$(timeout 60 node scripts/prd582-run-wasm.cjs "$out/$name.html" </dev/null 2>&1) || true
    if [[ "$got" == "$want" ]]; then echo "OK $name"; else
        echo "FAIL $name"; diff <(echo "$want") <(echo "$got") | head -10; fail=1; fi
done
exit $fail

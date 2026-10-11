// Headless dynamic-operation suite. Each kernel runs for >=80 ms after warmup.
// ns/op includes loop/control overhead; checksums describe one fixed-size batch.
const N = 16384;
function bench(name: string, fn: any, units: number) {
    let checksum = 0;
    const warm = performance.now();
    while (performance.now() - warm < 60) checksum = fn();
    let batches = 0;
    const start = performance.now();
    let elapsed = 0;
    do { checksum = fn(); batches++; elapsed = performance.now() - start; } while (elapsed < 80);
    console.log("op", name, elapsed * 1000000 / (batches * units), checksum);
    console.log("sample", name, elapsed, batches * units);
}
class Cell { x: number; constructor(x: number) { this.x = x; } }
function weakGet(r: WeakMap<Cell, number>, keys: Cell[], count: number): number { let sum = 0; for (let i = 0; i < N; i++) sum += r.get(keys[i % count]); return sum; }
function weakGetAny(r: any, keys: Cell[], count: number): number { let sum = 0; for (let i = 0; i < N; i++) sum += r.get(keys[i % count]); return sum; }
function weakHas(r: WeakMap<Cell, number>, keys: Cell[], count: number): number { let sum = 0; for (let i = 0; i < N; i++) { if (r.has(keys[i % count])) sum++; } return sum; }
function weakSet(r: WeakMap<Cell, number>, keys: Cell[], count: number): number { for (let i = 0; i < N; i++) r.set(keys[i % count], i % count); return N; }
function mapGet(r: any, keys: any, count: number): number { let sum = 0; for (let i = 0; i < N; i++) sum += r.get(keys[i % count]); return sum; }
function suite(count: number) {
    const keys: Cell[] = []; const strings: string[] = [];
    const weak = new WeakMap<Cell, number>();
    const objects = new Map<Cell, number>(); const text = new Map<string, number>();
    for (let i = 0; i < count; i++) { const key = new Cell(i); keys.push(key); strings.push("key_" + i); weak.set(key, i); objects.set(key, i); text.set(strings[i], i); }
    bench("weakmap-get-typed-" + count, () => weakGet(weak, keys, count), N);
    bench("weakmap-get-any-" + count, () => weakGetAny(weak, keys, count), N);
    bench("weakmap-has-" + count, () => weakHas(weak, keys, count), N);
    bench("weakmap-set-" + count, () => weakSet(weak, keys, count), N);
    bench("map-get-object-" + count, () => mapGet(objects, keys, count), N);
    bench("map-get-string-" + count, () => mapGet(text, strings, count), N);
}
suite(1); suite(100); suite(10000);

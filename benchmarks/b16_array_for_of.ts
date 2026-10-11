const n = 1000000;
const allocations: number[] = [];
const lattice: { count: number; values: Float32Array }[] = [];
const cell = { count: 2, values: new Float32Array([3, 4]) };
for (let i = 0; i < n; i++) {
  allocations.push(i);
  lattice.push(cell);
}
let sum = 0;
let start = Date.now();
for (const at of allocations.splice(0)) sum += at;
console.log('splice', (Date.now() - start) * 1000000 / n, sum);
sum = 0;
start = Date.now();
for (const [octave, { count, values }] of lattice.entries()) sum += octave + count + values[0];
console.log('entries', (Date.now() - start) * 1000000 / n, sum);

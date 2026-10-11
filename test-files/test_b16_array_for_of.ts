const allocations: number[] = [1, 2, 3, 4];
let freed = 0;
for (const at of allocations.splice(1)) freed += at;
console.log('splice', freed, allocations.length);
const lattice: { count: number; values: Float32Array }[] = [
  { count: 2, values: new Float32Array([3, 4]) },
  { count: 1, values: new Float32Array([5]) }
];
let total = 0;
for (const [octave, { count, values }] of lattice.entries()) total += octave + count + values[0];
console.log('entries', total);
let keys = 0;
for (const k of lattice.keys()) keys += k;
let values = 0;
for (const v of lattice.values()) values += v.count;
console.log('keys-values', keys, values);
const growing: number[] = [1, 2];
let seen = '';
for (const v of growing.values()) {
  seen += v + ',';
  if (v === 1) growing.push(3, 4);
}
console.log('grow', seen);
const shrinking: number[] = [1, 2, 3];
seen = '';
for (const [i, v] of shrinking.entries()) {
  seen += i + ':' + v + ',';
  if (i === 0) shrinking.length = 1;
}
console.log('shrink', seen);
const holes: any[] = [1, , 3];
seen = '';
for (const v of holes.values()) seen += String(v) + ',';
console.log('holes', seen);
const manual = [7, 8].values();
const first = manual.next();
const second = manual.next();
console.log('manual-results', first === second, first.value, second.value);
const iterProto: any = Object.getPrototypeOf([].values());
const next = iterProto.next;
let nextCalls = 0;
iterProto.next = function () { nextCalls++; return next.call(this); };
seen = '';
for (const v of [4, 5].values()) seen += v + ',';
iterProto.next = next;
console.log('patched-next', seen, nextCalls);
const plainDecorated: any = [9, 10].values();
plainDecorated.label = 321;
seen = '';
for (const v of plainDecorated) seen += v + ',';
console.log('decorated', seen, plainDecorated.label);
const decorated: any = [7, 8].values();
decorated.label = 123;
const decoratedNext = decorated.next.bind(decorated);
let getterCalls = 0;
Object.defineProperty(decorated, 'next', { get() { getterCalls++; return decoratedNext; } });
seen = '';
for (const v of decorated) seen += v + ',';
console.log('own-next-getter', seen, getterCalls > 0, decorated.label);
const arrayProto: any = Array.prototype;
const iterator = arrayProto[Symbol.iterator];
arrayProto[Symbol.iterator] = function () {
  let i = 0;
  return { next() { return i++ < 2 ? { value: 40 + i, done: false } : { done: true }; } };
};
let patched = 0;
for (const v of [1, 2].splice(0)) patched += v;
arrayProto[Symbol.iterator] = iterator;
console.log('patched-prototype', patched);

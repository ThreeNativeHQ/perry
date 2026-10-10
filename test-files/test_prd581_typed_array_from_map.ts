const a = Uint32Array.from({ length: 3 }, (_, i) => i * 2);
console.log("A", a[2]);
const b = Uint32Array.from([1, 2, 3], (v) => v + 1);
console.log("B", b[2]);
const e = Float32Array.from([1, 2], (v) => v * 2);
console.log("E", e[1]);
console.log("thisArg", Uint8Array.from([1], function (this: { k: number }, v) { return v + this.k; }, { k: 2 })[0]);
const array0 = Int8Array.from([1, 2, 3], (v, i) => v + i);
const like0 = Int8Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Int8Array", array0[2] === 5, like0[2] === 5);
const array1 = Uint8Array.from([1, 2, 3], (v, i) => v + i);
const like1 = Uint8Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Uint8Array", array1[2] === 5, like1[2] === 5);
const array2 = Uint8ClampedArray.from([1, 2, 3], (v, i) => v + i);
const like2 = Uint8ClampedArray.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Uint8ClampedArray", array2[2] === 5, like2[2] === 5);
const array3 = Int16Array.from([1, 2, 3], (v, i) => v + i);
const like3 = Int16Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Int16Array", array3[2] === 5, like3[2] === 5);
const array4 = Uint16Array.from([1, 2, 3], (v, i) => v + i);
const like4 = Uint16Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Uint16Array", array4[2] === 5, like4[2] === 5);
const array5 = Int32Array.from([1, 2, 3], (v, i) => v + i);
const like5 = Int32Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Int32Array", array5[2] === 5, like5[2] === 5);
const array6 = Uint32Array.from([1, 2, 3], (v, i) => v + i);
const like6 = Uint32Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Uint32Array", array6[2] === 5, like6[2] === 5);
const array7 = Float16Array.from([1, 2, 3], (v, i) => v + i);
const like7 = Float16Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Float16Array", array7[2] === 5, like7[2] === 5);
const array8 = Float32Array.from([1, 2, 3], (v, i) => v + i);
const like8 = Float32Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Float32Array", array8[2] === 5, like8[2] === 5);
const array9 = Float64Array.from([1, 2, 3], (v, i) => v + i);
const like9 = Float64Array.from({ length: 3, 0: 1, 1: 2, 2: 3 }, (v, i) => v + i);
console.log("Float64Array", array9[2] === 5, like9[2] === 5);
const big0 = BigInt64Array.from([1n, 2n], (v, i) => v + BigInt(i));
const bigLike0 = BigInt64Array.from({ length: 2, 0: 1n, 1: 2n }, (v, i) => v + BigInt(i));
console.log("BigInt64Array", big0[1] === 3n, bigLike0[1] === 3n);
const big1 = BigUint64Array.from([1n, 2n], (v, i) => v + BigInt(i));
const bigLike1 = BigUint64Array.from({ length: 2, 0: 1n, 1: 2n }, (v, i) => v + BigInt(i));
console.log("BigUint64Array", big1[1] === 3n, bigLike1[1] === 3n);
const C = Uint32Array;
const from = C.from;
console.log("reified", C.from([2], (v, i) => v + i)[0], from.call(C, [3], (v) => v + 1)[0]);
console.log("no-map", Uint32Array.from({ length: 3 })[2], Uint32Array.from([4], undefined)[0]);
console.log("array-control", Array.from([1, 2], (v, i) => v + i)[1]);
for (const invalid of [null, 0, {}]) {
    try { Uint32Array.from([], invalid as any); console.log("invalid", false); }
    catch (error: any) { console.log("invalid", error instanceof TypeError); }
}
try { Uint32Array.from([1], () => { throw new Error("mapper"); }); }
catch (error: any) { console.log("mapper-throw", error.message); }

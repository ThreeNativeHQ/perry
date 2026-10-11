// Kernel 1: three.js-shaped Vector3/Quaternion/Matrix4 on class fields.
class Vector3 {
  x: number; y: number; z: number;
  constructor(x: number, y: number, z: number) { this.x = x; this.y = y; this.z = z; }
  applyMatrix4(m: Matrix4): Vector3 {
    const x = this.x, y = this.y, z = this.z;
    const e = m.elements;
    const w = 1 / (e[3] * x + e[7] * y + e[11] * z + e[15]);
    this.x = (e[0] * x + e[4] * y + e[8] * z + e[12]) * w;
    this.y = (e[1] * x + e[5] * y + e[9] * z + e[13]) * w;
    this.z = (e[2] * x + e[6] * y + e[10] * z + e[14]) * w;
    return this;
  }
}
class Quaternion {
  x: number; y: number; z: number; w: number;
  constructor(x: number, y: number, z: number, w: number) { this.x = x; this.y = y; this.z = z; this.w = w; }
}
class Matrix4 {
  elements: number[];
  constructor() { this.elements = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]; }
  compose(p: Vector3, q: Quaternion, s: Vector3): Matrix4 {
    const te = this.elements;
    const x = q.x, y = q.y, z = q.z, w = q.w;
    const x2 = x + x, y2 = y + y, z2 = z + z;
    const xx = x * x2, xy = x * y2, xz = x * z2;
    const yy = y * y2, yz = y * z2, zz = z * z2;
    const wx = w * x2, wy = w * y2, wz = w * z2;
    const sx = s.x, sy = s.y, sz = s.z;
    te[0] = (1 - (yy + zz)) * sx; te[1] = (xy + wz) * sx; te[2] = (xz - wy) * sx; te[3] = 0;
    te[4] = (xy - wz) * sy; te[5] = (1 - (xx + zz)) * sy; te[6] = (yz + wx) * sy; te[7] = 0;
    te[8] = (xz + wy) * sz; te[9] = (yz - wx) * sz; te[10] = (1 - (xx + yy)) * sz; te[11] = 0;
    te[12] = p.x; te[13] = p.y; te[14] = p.z; te[15] = 1;
    return this;
  }
}
const N = 200000;
const m = new Matrix4();
const p = new Vector3(1, 2, 3);
const q = new Quaternion(0.1, 0.2, 0.3, 0.927);
const s = new Vector3(1, 1, 1);
const v = new Vector3(0, 0, 0);
let acc = 0;
const t0 = performance.now();
for (let i = 0; i < N; i++) {
  p.x = i * 0.001;
  m.compose(p, q, s);
  v.x = 1; v.y = 2; v.z = 3;
  v.applyMatrix4(m);
  acc += v.x + v.y + v.z;
}
const t1 = performance.now();
console.log("math acc=" + Math.round(acc));
console.log("math ms=" + (t1 - t0).toFixed(1));

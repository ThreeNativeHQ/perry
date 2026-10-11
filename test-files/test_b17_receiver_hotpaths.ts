class PrivateOwner {
  #x = 7;
  read(): number { return this.#x; }
  static has(o: any): boolean { return #x in o; }
}
class PrivateChild extends PrivateOwner {}
class Other { #x = 9; }
class BufferChild extends Buffer {}
const buffer: any = Buffer.from([3, 4]);
const typed: any = new Float32Array([5, 6]);
const child: any = new BufferChild(2);
child[0] = 8;
const owner = new PrivateOwner();
const backend: any = { step: () => 11, other: () => 12 };
console.log('ordinary', owner.read(), backend.step());
console.log('brands', PrivateOwner.has(owner), PrivateOwner.has(new PrivateChild()),
  PrivateOwner.has(new Other()), PrivateOwner.has({}), PrivateOwner.has(buffer),
  PrivateOwner.has(typed), PrivateOwner.has(child));
console.log('buffer', buffer.includes(4), buffer.at(0), buffer.length);
console.log('typed', typed.includes(6), typed.at(0), typed.length);
console.log('subclass', child.includes(8), child.at(0), child.length, Buffer.isBuffer(child));
backend.step = () => 19;
console.log('replace', backend.step());
delete backend.step;
Object.setPrototypeOf(backend, { step: () => 23 });
console.log('prototype', backend.step());

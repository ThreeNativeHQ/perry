class Base { type = 1009; }
function bind() {
    class Tex extends Base { isTexture = true; }
    return { Tex };
}
const b = bind();
const created: any = Object.create(b.Tex.prototype);
console.log("create", typeof created.constructor, created.constructor === b.Tex);
console.log("prototype", b.Tex.prototype.constructor === b.Tex);
console.log("new", new b.Tex().constructor.name);
const plain: any = {};
Object.setPrototypeOf(plain, b.Tex.prototype);
console.log("adopt-object", typeof plain.constructor, plain.constructor === b.Tex);
const texture: any = new Base();
Object.setPrototypeOf(texture, b.Tex.prototype);
console.log("adopt-instance", typeof texture.constructor, texture.constructor === b.Tex);
const intermediate = Object.create(b.Tex.prototype);
const deep: any = Object.create(intermediate);
console.log("deep", typeof deep.constructor, deep.constructor === b.Tex);
const key = "constructor";
console.log("computed", typeof created[key], created[key] === b.Tex);
const second = bind();
const secondObject: any = Object.create(second.Tex.prototype);
console.log("fresh", created.constructor === b.Tex, secondObject.constructor === second.Tex);
function bindCaptured(seed: number) {
    class Tex extends Base { value = seed; }
    return { Tex };
}
const capturedA = bindCaptured(1);
const capturedB = bindCaptured(2);
const capturedObjectA: any = Object.create(capturedA.Tex.prototype);
const capturedObjectB: any = Object.create(capturedB.Tex.prototype);
console.log("captured", capturedObjectA.constructor === capturedA.Tex, capturedObjectB.constructor === capturedB.Tex, capturedA.Tex !== capturedB.Tex);
const capturedTexture: any = new Base();
Object.setPrototypeOf(capturedTexture, capturedA.Tex.prototype);
console.log("adopt-captured", typeof capturedTexture.constructor, capturedTexture.constructor === capturedA.Tex);
class TopLevel {}
const top: any = Object.create(TopLevel.prototype);
console.log("top-level", typeof top.constructor, top.constructor === TopLevel);
function FnCtor() {}
const fnInstance: any = new Base();
Object.setPrototypeOf(fnInstance, FnCtor.prototype);
console.log("function", typeof fnInstance.constructor, fnInstance.constructor === FnCtor);
Object.defineProperty(created, "constructor", { value: undefined, writable: true, configurable: true });
console.log("own-undefined", created.constructor === undefined);
Object.defineProperty(intermediate, "constructor", { value: 42, writable: true, configurable: true });
console.log("nearer", deep.constructor === 42);
const accessorProto = Object.create(b.Tex.prototype);
Object.defineProperty(accessorProto, "constructor", { get() { return this.marker; } });
const accessor: any = Object.create(accessorProto);
accessor.marker = b.Tex;
console.log("accessor", accessor.constructor === b.Tex);
Object.setPrototypeOf(texture, null);
console.log("null", texture.constructor === undefined);

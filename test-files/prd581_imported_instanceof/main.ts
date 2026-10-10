import { Object3D, wrapScene } from "./three";
const wrapped = (wrapScene as () => object)();
function plain(root: unknown): boolean {
  return root instanceof (Object3D as any);
}
function typedUnknown(root: unknown): boolean {
  const cls: unknown = Object3D;
  return root instanceof (cls as any);
}
function rawUnknown(root: unknown): boolean {
  // @ts-ignore -- probe: RHS statically unknown, as the facade export is
  return root instanceof Object3D;
}
function asCtor(root: unknown): boolean {
  return root instanceof (Object3D as new () => object);
}
console.log("any (want true)", plain(wrapped));
console.log("local unknown->any (want true)", typedUnknown(wrapped));
console.log("raw unknown (want true)", rawUnknown(wrapped));
console.log("ctor cast (want true)", asCtor(wrapped));

import DefaultValue, { RealClass as ImportedRealClass } from "./three";
class LocalClass {}
function runtimeCheck(v: unknown) { return v instanceof Object3D; }
function importedCheck(v: unknown) { return v instanceof ImportedRealClass; }
function localCheck(v: unknown) { return v instanceof LocalClass; }
console.log("runtime", runtimeCheck(wrapped), runtimeCheck({}));
console.log("default", wrapped instanceof DefaultValue, ({}) instanceof DefaultValue);
console.log("imported", importedCheck(new ImportedRealClass()), importedCheck({}));
console.log("local", localCheck(new LocalClass()), localCheck({}));
console.log("builtins", new Error() instanceof Error, new Map() instanceof Map, new Uint8Array(1) instanceof Uint8Array);

import { PropertyBinding } from "./three";
interface IBinding extends PropertyBinding { targetObject: unknown }
function audit(root: unknown, path: string) {
  const propertyBinding = new PropertyBinding(root, path);
  // Invoke first: reading/calling the method as a value must not prime dispatch.
  console.log("bind", propertyBinding.bind());
  console.log("call", propertyBinding.call(7));
  console.log("apply", propertyBinding.apply(8));
  const proto = Object.getPrototypeOf(propertyBinding);
  console.log("prototype", proto === PropertyBinding.prototype, Object.hasOwn(proto, "bind"));
  console.log("explicit", proto.bind.call(propertyBinding));
  Object.defineProperty(propertyBinding, "bind", { value(this: any) { return "own:" + this.path; }, configurable: true });
  console.log("own", propertyBinding.bind());
  Object.defineProperty(propertyBinding, "bind", { value: 1, configurable: true });
  try { propertyBinding.bind(); }
  catch (error: any) { console.log("invalid", error instanceof TypeError); }
}
audit({}, "a.b");
function sum(this: { k: number }, v: number) { return this.k + v; }
console.log("functions", sum.bind({ k: 2 })(3), sum.call({ k: 2 }, 3), sum.apply({ k: 2 }, [3]));
try { Function.prototype.bind.call({}); }
catch (error: any) { console.log("brand", error instanceof TypeError); }

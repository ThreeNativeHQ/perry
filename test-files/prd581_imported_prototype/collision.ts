import { Material } from "./rebound";
import { Material as RealMaterial } from "./engine";
const M = Material;
console.log("identity", Material === RealMaterial, M === Material);
console.log("marker", Material.marker, M.marker);
console.log("prototype", Material.prototype === RealMaterial.prototype, M.prototype === Material.prototype);
console.log("instance", new Material().onBeforeCompile());

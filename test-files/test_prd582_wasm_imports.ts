// PRD-582: a call to an imported `const` function, and a dynamic call with four or more arguments.
import { vec2, vec4 } from "./prd582_wasm_const_fns";
import { vec2 as viaStar } from "./prd582_wasm_reexport";
// A longer module path ending in the same name must not capture the star re-export above.
import { decoy } from "./prd582_wasm_decoy/prd582_wasm_const_fns/index";

console.log(vec2(1, 2));
console.log(vec4(1, 2, 3, 4));
console.log("star " + viaStar(3, 4) + " " + decoy);
const local = (a: number, b: number, c: number, d: number, e: number): number => a + b + c + d + e;
const pick = [local][0];
console.log("five " + pick(1, 2, 3, 4, 5));

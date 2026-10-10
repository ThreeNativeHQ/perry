import { bind } from "./engine";
const bound = bind();
export const Object3D = bound.Object3D;
export const Mesh = bound.Mesh;
export const Scene = bound.Scene;
export const wrapScene = bound.wrapScene;

export class RealClass {}
export default Object3D;

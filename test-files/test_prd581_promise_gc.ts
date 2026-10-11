async function decode(id: number): Promise<object> {
  const nodes: object[] = [];
  for (let i = 0; i < 50000; i++) nodes.push({ index: i, owner: id, name: "node" + i });
  return { nodes, id };
}
function parse(id: number, done: (result: object) => void, fail: (error: unknown) => void) {
  decode(id).then(done, fail);
}
async function load(id: number): Promise<object> {
  const [value] = await Promise.all([Promise.resolve(id), Promise.resolve("loader")]);
  return await new Promise<object>((resolve, reject) => parse(value as number, resolve, reject));
}
Promise.all(Array.from({length: 25}, (_, i) => load(i))).then((values) => {
 console.log("TN_ASYNC_NO_AWAIT_GC:" + values.length); process.exit(0);
}, error => { console.error(String(error)); process.exit(1); });

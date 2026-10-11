// PRD-582: async bodies run as Wasm step closures; the runtime only chains promises.
async function inc(n: number): Promise<number> {
  await Promise.resolve();
  return n + 1;
}

async function sum(count: number): Promise<number> {
  let total = 0;
  for (let i = 0; i < count; i++) {
    total += await inc(i);
  }
  return total;
}

async function noAwait(): Promise<string> {
  return "plain";
}

async function fails(): Promise<number> {
  await inc(0);
  throw new Error("boom");
}

async function recovers(): Promise<string> {
  try {
    await fails();
    return "missed";
  } catch (e) {
    return `caught ${(e as Error).message}`;
  }
}

class Loader {
  base = 10;
  async load(n: number): Promise<number> {
    const v = await inc(n);
    return v + this.base;
  }
}

const arrow = async (s: string) => `${await noAwait()}-${s}`;

async function main(): Promise<void> {
  console.log(`inc ${await inc(1)}`);
  console.log(`sum ${await sum(4)}`);
  console.log(`noAwait ${await noAwait()}`);
  console.log(`recovers ${await recovers()}`);
  console.log(`method ${await new Loader().load(1)}`);
  console.log(`arrow ${await arrow("x")}`);
  const all = await Promise.all([inc(1), inc(2), inc(3)]);
  console.log(`all ${all.join(",")}`);
}

main().then(
  () => console.log("done"),
  (e: unknown) => console.log(`main failed ${String(e)}`),
);
fails().catch((e: Error) => console.log(`rejected ${e.message}`));

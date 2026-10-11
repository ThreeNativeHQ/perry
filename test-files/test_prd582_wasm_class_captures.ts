// PRD-582: a class declared inside a function reads the function's locals from instance
// methods and from static methods (ClassCaptureValue / RegisterClassCaptures).
function measure(prefix: string, scale: number): string[] {
  const unit = "m";
  class Kind {
    constructor(readonly value: number) {}
    label(): string {
      return `${prefix}${this.value * scale}${unit}`;
    }
    static create(value: number): Kind {
      return new Kind(value + scale);
    }
    static describe(): string {
      return `${prefix}x${scale}`;
    }
  }
  return [new Kind(3).label(), Kind.create(1).label(), Kind.describe()];
}
console.log(measure("len:", 2).join(" "));

// The class escapes its function as a value: `new` and static calls go through the value.
function makeKind(prefix: string) {
  class Tagged {
    constructor(readonly value: number) {}
    label(): string {
      return prefix + this.value;
    }
    static of(value: number): Tagged {
      return new Tagged(value * 10);
    }
  }
  return Tagged;
}
const Tagged = makeKind("t:");
console.log(new Tagged(1).label() + " " + Tagged.of(2).label());

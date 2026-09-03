/** Mirrors MAUI Easing (subset): Ease(t) with t in 0..1. */
export class Easing {
  constructor(private readonly fn: (x: number) => number) {}
  Ease(x: number): number { return this.fn(x); }

  static readonly Linear = new Easing((x) => x);
  static readonly CubicIn = new Easing((x) => x * x * x);
  static readonly CubicOut = new Easing((x) => (x - 1) ** 3 + 1);
  static readonly CubicInOut = new Easing((x) => (x < 0.5 ? 4 * x ** 3 : (x - 1) * (2 * x - 2) ** 2 + 1));
  static readonly Default = Easing.CubicInOut;
}

// Mirrors DrawnUi scrolling physics: DecelerationTimingParameters + ScrollFlingAnimator (fling),
// Spring + SpringTimingParameters + SpringWithVelocityAnimator (bounce), VelocityAccumulator, RubberBandUtils.

import { SkiaValueAnimator } from "./Animators";
import type { SkiaControl } from "./SkiaControl";
import { SKRect } from "./Types";

/** UIKit-style exponential deceleration: value(t) = v0 * (rate^(1000 t) - 1) / k, k = 1000 ln(rate). */
export class DecelerationTimingParameters {
  InitialValue: number;
  InitialVelocity: number;
  readonly DecelerationRate: number;
  readonly DecelerationK: number;
  Threshold: number;

  constructor(initialValue: number, initialVelocity: number, decelerationRate: number, threshold: number) {
    if (decelerationRate <= 0 || decelerationRate >= 1) throw new RangeError("Deceleration rate must be in (0, 1)");
    this.InitialValue = initialValue;
    this.InitialVelocity = initialVelocity;
    this.Threshold = threshold;
    this.DecelerationRate = decelerationRate;
    this.DecelerationK = 1000 * Math.log(decelerationRate);
  }

  /** Initial velocity that lands on targetValue after durationSecs (DrawnUi second ctor). */
  static ToDestination(currentValue: number, targetValue: number, durationSecs: number, decelerationRate: number, threshold = 0.1): DecelerationTimingParameters {
    if (durationSecs <= 0) throw new RangeError("Duration must be > 0");
    const p = new DecelerationTimingParameters(currentValue, 0, decelerationRate, threshold);
    const distance = targetValue - currentValue;
    if (Math.abs(distance) < 1e-5) return p;
    const denominator = Math.pow(decelerationRate, 1000 * durationSecs) - 1;
    p.InitialVelocity = Math.abs(denominator) < 1e-5 ? distance / durationSecs : (distance * p.DecelerationK) / denominator;
    return p;
  }

  get Destination(): number { return this.InitialVelocity === 0 ? this.InitialValue : this.InitialValue - this.InitialVelocity / this.DecelerationK; }

  get DurationSecs(): number {
    if (this.InitialVelocity === 0) return 0;
    const divisor = (-this.DecelerationK * this.Threshold) / Math.abs(this.InitialVelocity);
    return divisor <= 0 ? 0 : Math.log(divisor) / this.DecelerationK;
  }

  ValueAt(secs: number): number {
    if (this.DecelerationK === 0) return this.InitialValue;
    return this.InitialValue + this.InitialVelocity * ((Math.pow(this.DecelerationRate, 1000 * secs) - 1) / this.DecelerationK);
  }

  VelocityAt(secs: number): number { return this.InitialVelocity * Math.pow(this.DecelerationRate, 1000 * secs); }

  /** Seconds until the curve reaches value (used to cut a fling at the content edge). */
  DurationToValue(value: number): number {
    if (this.DecelerationK === 0 || this.InitialVelocity === 0) return 0;
    const distance = Math.abs(value - this.InitialValue);
    return distance === 0 ? 0 : Math.log(1 + (this.DecelerationK * distance) / Math.abs(this.InitialVelocity)) / this.DecelerationK;
  }
}

/** DrawnUi ScrollFlingAnimator: drives an offset along a deceleration curve; stops when the change rate stalls. */
export class ScrollFlingAnimator extends SkiaValueAnimator {
  SelfFinished = false;
  ValueThreshold = 0.1;
  Parameters?: DecelerationTimingParameters;
  CurrentVelocity = 0;
  private lastValue = 0;
  private lastUpdateTime = 0;
  private belowThresholdFrames = 0;
  private static readonly FramesBelowThresholdToStop = 3;

  constructor(parent: SkiaControl) { super(parent); }

  InitializeWithVelocity(position: number, velocity: number, deceleration = 0.998, valueThreshold = 1.85): void {
    this.Parameters = new DecelerationTimingParameters(position, velocity, deceleration, 0.001);
    this.Speed = this.Parameters.DurationSecs;
    this.ValueThreshold = valueThreshold;
    this.lastValue = position;
    this.belowThresholdFrames = 0;
  }

  /** Reaches target exactly at timeSecs; the curve is stopped there (its asymptote lies beyond the target). */
  InitializeWithDestination(position: number, target: number, timeSecs: number, deceleration = 0.998, valueThreshold = 0.1): void {
    this.Parameters = DecelerationTimingParameters.ToDestination(position, target, timeSecs, deceleration, 0.001);
    this.Speed = timeSecs;
    this.ValueThreshold = valueThreshold;
    this.lastValue = position;
    this.belowThresholdFrames = 0;
  }

  override Start(delayMs = 0): void {
    this.SelfFinished = false;
    this.belowThresholdFrames = 0;
    this.lastUpdateTime = 0;
    super.Start(delayMs);
  }

  /** Speed here is SECONDS (DurationSecs), unlike SkiaValueAnimator's ms — same as the C# class. */
  protected override UpdateValue(deltaT: number, deltaFromStart: number): boolean {
    const p = this.Parameters;
    if (!p) return true;
    const secs = deltaFromStart / 1e9;
    if (secs > this.Speed) {
      // Land exactly where the planned duration ends (edge or destination), not a frame past it.
      this.mValue = p.ValueAt(this.Speed);
      this.CurrentVelocity = p.VelocityAt(this.Speed);
      this.SelfFinished = true;
      return true;
    }
    this.mValue = p.ValueAt(secs);
    this.CurrentVelocity = p.VelocityAt(secs);
    if (this.lastUpdateTime > 0) {
      const dt = deltaT / 1e9;
      const changeRate = dt > 0 ? Math.abs(this.mValue - this.lastValue) / dt : 0;
      if (changeRate < this.ValueThreshold) {
        if (++this.belowThresholdFrames >= ScrollFlingAnimator.FramesBelowThresholdToStop) { this.SelfFinished = true; return true; }
      } else this.belowThresholdFrames = 0;
    }
    this.lastValue = this.mValue;
    this.lastUpdateTime = deltaFromStart;
    return false;
  }
}

/** DrawnUi Spring. */
export class Spring {
  constructor(public Mass: number, public Stiffness: number, public DampingRatio: number) {}
  static get Damped() { return new Spring(1, 200, 1); }
  static get Default() { return new Spring(1, 200, 0.5); }
}

/** DrawnUi SpringTimingParameters (critically damped or underdamped). Displacement decays to 0. */
export class SpringTimingParameters {
  private readonly beta: number;
  private readonly wd: number;
  private readonly c1: number;
  private readonly c2: number;
  readonly DurationSecs: number;

  constructor(readonly Spring: Spring, readonly Displacement: number, readonly InitialVelocity: number, readonly Threshold: number) {
    if (Spring.DampingRatio <= 0 || Spring.DampingRatio > 1) throw new RangeError("dampingRatio should be in (0, 1]");
    const w0 = Math.sqrt(Spring.Stiffness / Spring.Mass);
    this.beta = Spring.DampingRatio * w0 * 2;
    this.c1 = Displacement;
    if (Spring.DampingRatio >= 1) {
      this.wd = 0;
      this.c2 = InitialVelocity + this.beta * Displacement;
      if (Displacement === 0 && InitialVelocity === 0) this.DurationSecs = 0;
      else {
        const t1 = (1 / this.beta) * Math.log((2 * Math.abs(this.c1)) / Threshold);
        const t2 = (2 / this.beta) * Math.log((4 * Math.abs(this.c2)) / (Math.E * this.beta * Threshold));
        this.DurationSecs = Math.max(t1, t2);
      }
    } else {
      this.wd = w0 * Math.sqrt(1 - Spring.DampingRatio * Spring.DampingRatio);
      this.c2 = (InitialVelocity + this.beta * Displacement) / this.wd;
      this.DurationSecs = Displacement === 0 && InitialVelocity === 0 ? 0 : Math.log((Math.abs(this.c1) + Math.abs(this.c2)) / Threshold) / this.beta;
    }
  }

  ValueAt(t: number): number {
    if (this.wd === 0) return Math.exp(-this.beta * t) * (this.c1 + this.c2 * t);
    return Math.exp(-this.beta * t) * (this.c1 * Math.cos(this.wd * t) + this.c2 * Math.sin(this.wd * t));
  }
}

/** DrawnUi SpringWithVelocityAnimator: returns an offset to restOffset from a displacement with initial velocity. */
export class SpringWithVelocityAnimator extends SkiaValueAnimator {
  private origin = 0;
  Parameters?: SpringTimingParameters;

  constructor(parent: SkiaControl) { super(parent); }

  Initialize(restOffset: number, position: number, velocity: number, spring: Spring, thresholdStop = 0.5): void {
    this.origin = restOffset;
    this.Parameters = new SpringTimingParameters(spring, position, velocity, thresholdStop);
  }

  protected override UpdateValue(_deltaT: number, deltaFromStart: number): boolean {
    const p = this.Parameters;
    if (!p) return true;
    const secs = deltaFromStart / 1e9;
    if (secs > p.DurationSecs) { this.mValue = this.origin; return true; }
    this.mValue = this.origin + p.ValueAt(secs);
    return false;
  }
}

/** DrawnUi VelocityAccumulator: weighted average of the last samples inside a 150 ms window. */
export class VelocityAccumulator {
  private readonly samples: { X: number; Y: number; time: number }[] = [];
  private static readonly MaxSampleSize = 5;
  private static readonly ConsiderationTimeframeMs = 150;

  Clear(): void { this.samples.length = 0; }

  CaptureVelocity(x: number, y: number, arrivedTimeNanos = 0): void {
    const time = arrivedTimeNanos > 0 ? arrivedTimeNanos / 1e6 : performance.now();
    if (this.samples.length === VelocityAccumulator.MaxSampleSize) this.samples.shift();
    this.samples.push({ X: x, Y: y, time });
  }

  CalculateFinalVelocity(clampAbsolute = 0): { X: number; Y: number } {
    const now = performance.now();
    const relevant = this.samples.filter((s) => now - s.time <= VelocityAccumulator.ConsiderationTimeframeMs);
    if (relevant.length === 0) return { X: 0, Y: 0 };
    let sx = 0, sy = 0, weights = 0;
    relevant.forEach((s, i) => { sx += s.X * (i + 1); sy += s.Y * (i + 1); weights += i + 1; });
    let x = sx / weights, y = sy / weights;
    if (clampAbsolute !== 0) { x = Math.max(-clampAbsolute, Math.min(clampAbsolute, x)); y = Math.max(-clampAbsolute, Math.min(clampAbsolute, y)); }
    return { X: x, Y: y };
  }
}

/** DrawnUi RubberBandUtils. */
export const RubberBandUtils = {
  /** Clamps coord into limits with an iOS-style rubber band beyond them; dim = the viewport dimension. */
  RubberBandClamp(coord: number, dim: number, start: number, end: number, coeff = 0.275, onEmpty = 40): number {
    if (start > end) return coord;
    const clamped = Math.max(start, Math.min(end, coord));
    const overscroll = coord < start ? coord - start : coord > end ? coord - end : 0;
    if (overscroll === 0) return clamped;
    if (dim === 0) dim = onEmpty;
    const rubber = (1 - 1 / ((Math.abs(overscroll) * coeff) / dim + 1)) * dim;
    return clamped + Math.sign(overscroll) * rubber;
  },
  ClampOnTrack(x: number, y: number, track: SKRect, coeff: number, dimX: number, dimY: number): { X: number; Y: number } {
    return {
      X: RubberBandUtils.RubberBandClamp(x, dimX, track.Left, track.Right, coeff),
      Y: RubberBandUtils.RubberBandClamp(y, dimY, track.Top, track.Bottom, coeff),
    };
  },
};

import { SKRect, SkiaBevel, SkiaShape, type Color, type SkiaControl } from "drawnui-react/core";

// Ported from DrawnUi src/Shared/Samples/Pong.Shared/Game (GameExtensions.cs, IWithHitBox.cs, Sprites/*.cs).

export const BALL_SIZE = 14;
export const PADDLE_WIDTH = 80;
export const PADDLE_HEIGHT = 16;
export const BALL_SPEED = 300;

/** GameExtensions.GetHitBox: the collision rect in field space (Left/Top + size), where all the game logic lives. */
export function GetHitBox(sprite: SkiaControl): SKRect {
  const size = sprite.MeasuredSize.Units;
  return new SKRect(sprite.Left, sprite.Top, sprite.Left + size.Width, sprite.Top + size.Height);
}

/** SkiaSharp SKRect.IntersectsWith. */
export function IntersectsWith(a: SKRect, b: SKRect): boolean {
  return a.Left < b.Right && b.Left < a.Right && a.Top < b.Bottom && b.Top < a.Bottom;
}

/** SkiaSharp SKRect.MidX / MidY. */
export const MidX = (r: SKRect) => r.Left + r.Width / 2;
export const MidY = (r: SKRect) => r.Top + r.Height / 2;

/** IWithHitBox. */
export interface IWithHitBox {
  UpdateState(time: number, force?: boolean): void;
  readonly HitBox: SKRect;
}

export class PaddleSprite extends SkiaShape implements IWithHitBox {
  HitBox = SKRect.Empty;
  private stateUpdated = 0;

  constructor(color: Color) {
    super();
    this.UseCache = "GPU";
    this.HeightRequest = PADDLE_HEIGHT;
    this.WidthRequest = PADDLE_WIDTH;
    this.CornerRadius = PADDLE_HEIGHT / 2;
    this.HorizontalOptions = "Start";
    this.VerticalOptions = "Start";
    this.Type = "Rectangle";
    this.BackgroundColor = color;
    this.StrokeColor = "#CCCCFF";
    this.StrokeWidth = 2;
    this.BevelType = "Bevel";
    this.Bevel = new SkiaBevel({ Depth: 4, LightColor: "#FFFFFF", ShadowColor: "#333333", Opacity: 0.33 });
  }

  UpdateState(time: number, force = false): void {
    if (force || this.stateUpdated !== time) {
      this.HitBox = GetHitBox(this);
      this.stateUpdated = time;
    }
  }
}

export class BallSprite extends SkiaShape implements IWithHitBox {
  HitBox = SKRect.Empty;
  IsMoving = false;
  Speed = BALL_SPEED;

  private stateUpdated = 0;
  private angle = 0;
  private h1 = NaN;
  private h2 = NaN;
  private h3 = NaN;
  private oscillationCount = 0;

  constructor() {
    super();
    this.UseCache = "GPU";
    this.HeightRequest = BALL_SIZE;
    this.LockRatio = 1;
    this.HorizontalOptions = "Start";
    this.VerticalOptions = "Start";
    this.Type = "Circle";
    this.StrokeColor = "#FFFFFF";
    this.StrokeWidth = 2;
    this.BackgroundColor = "#FFFF00";
    this.BevelType = "Bevel";
    this.Bevel = new SkiaBevel({ Depth: 3, LightColor: "#FFFFFF", ShadowColor: "#333333", Opacity: 0.33 });
  }

  get Angle(): number { return this.angle; }
  set Angle(value: number) {
    this.angle = BallSprite.ClampAngleFromHorizontal(value);
    this.TrackAngleForOscillation(this.angle);
  }

  UpdateState(time: number, force = false): void {
    if (force || this.stateUpdated !== time) {
      this.HitBox = GetHitBox(this);
      this.stateUpdated = time;
    }
  }

  UpdatePosition(deltaSeconds: number): void {
    if (deltaSeconds <= 0 || !this.IsMoving) return;
    this.Left += this.Speed * Math.cos(this.Angle) * deltaSeconds;
    this.Top += this.Speed * Math.sin(this.Angle) * deltaSeconds;
  }

  /** Keeps the ball from flying (almost) horizontally: at least `min` radians away from 0 and PI. */
  static ClampAngleFromHorizontal(angle: number, min = Math.PI / 10): number {
    const twoPi = 2 * Math.PI;
    let normalized = angle % twoPi;
    if (normalized <= -Math.PI) normalized += twoPi;
    else if (normalized > Math.PI) normalized -= twoPi;

    const nearZero = Math.abs(normalized) < min;
    const nearPi = Math.abs(normalized) > Math.PI - min;
    if (!nearZero && !nearPi) return normalized;

    const sign = Math.sign(normalized) || 1;
    return nearZero ? sign * min : sign * (Math.PI - min);
  }

  private TrackAngleForOscillation(newAngle: number): void {
    if (!this.IsMoving) { this.ResetOscillation(); return; }
    this.h3 = this.h2;
    this.h2 = this.h1;
    this.h1 = newAngle;
    if (!Number.isNaN(this.h3)) this.CheckOscillation();
  }

  private CheckOscillation(): void {
    const tolerance = 0.01;
    if (Math.abs(this.h1 - this.h3) < tolerance && Math.abs(this.h2 - this.h1) > tolerance) {
      if (++this.oscillationCount >= 6) this.UnstickBall();
    } else {
      this.oscillationCount = 0;
    }
  }

  private UnstickBall(): void {
    const nudge = (Math.random() - 0.5) * 0.4;
    this.angle = BallSprite.ClampAngleFromHorizontal(this.angle + nudge);
    this.ResetOscillation();
  }

  private ResetOscillation(): void {
    this.h1 = this.h2 = this.h3 = NaN;
    this.oscillationCount = 0;
  }
}

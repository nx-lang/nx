import type { PongGame } from "./PongGame";
import { MidX, MidY, PADDLE_HEIGHT, PADDLE_WIDTH } from "./Sprites";

// Ported from DrawnUi src/Shared/Samples/Pong.Shared/Game/Ai/PongAI.cs.

export type AIDifficulty = "Easy" | "Medium" | "Hard" | "Perfect";

const rnd = () => Math.random();

export class PongAI {
  private targetX = 0;
  private reactionTimer = 0;
  private mistakeTimer = 0;
  private decisionChangeTimer = 0;
  private movementSmoothingTimer = 0;
  private makingMistake = false;
  private mistakeDirection = 0;
  private isMoving = false;
  private lastMovementDir = 0;

  private readonly reactionTimeMin: number;
  private readonly reactionTimeMax: number;
  private readonly accuracy: number;
  private readonly mistakeProbability: number;
  private readonly mistakeDurationMin: number;
  private readonly mistakeDurationMax: number;
  private decisionChangeInterval: number;
  private readonly movementSmoothingTime: number;

  constructor(private readonly game: PongGame, difficulty: AIDifficulty = "Medium") {
    const p = {
      Easy: [0.5, 1.2, 0.55, 0.45, 0.8, 1.6, 1.0, 0.3],
      Medium: [0.06, 0.22, 0.84, 0.10, 0.25, 0.5, 1.8, 0.10],
      Hard: [0.05, 0.15, 0.95, 0.03, 0.1, 0.25, 3.0, 0.05],
      Perfect: [0.01, 0.02, 1.0, 0.0, 0, 0, 5.0, 0.01],
    }[difficulty];
    [this.reactionTimeMin, this.reactionTimeMax, this.accuracy, this.mistakeProbability,
      this.mistakeDurationMin, this.mistakeDurationMax, this.decisionChangeInterval, this.movementSmoothingTime] = p;
    this.ResetTimers();
  }

  ResetTimers(): void {
    this.reactionTimer = this.GetReactionTime();
    this.mistakeTimer = 0;
    this.decisionChangeTimer = this.decisionChangeInterval;
    this.movementSmoothingTimer = 0;
    this.makingMistake = false;
    this.mistakeDirection = 0;
    this.isMoving = false;
    this.lastMovementDir = 0;
    this.game.SetAiMovement(0);
  }

  Update(delta: number): void {
    const ball = this.game.Ball;
    if (!ball.IsMoving) { this.SetMovement(0); return; }

    this.reactionTimer -= delta;
    this.movementSmoothingTimer -= delta;

    if (this.makingMistake) {
      this.mistakeTimer -= delta;
      if (this.mistakeTimer <= 0) {
        this.makingMistake = false;
        this.reactionTimer = this.GetReactionTime();
        this.SetMovement(0);
      } else {
        this.ApplyMistakeMovement();
      }
      return;
    }

    this.decisionChangeTimer -= delta;
    if (this.decisionChangeTimer <= 0) {
      if (rnd() < this.mistakeProbability) {
        this.makingMistake = true;
        this.mistakeTimer = rnd() * (this.mistakeDurationMax - this.mistakeDurationMin) + this.mistakeDurationMin;
        this.mistakeDirection = rnd() < 0.7 ? (rnd() < 0.5 ? -1 : 1) : 0;
      }
      this.decisionChangeInterval = 0.5 + rnd();
      this.decisionChangeTimer = this.decisionChangeInterval;
    }

    if (this.makingMistake) { this.ApplyMistakeMovement(); return; }

    const ballComingUp = Math.sin(ball.Angle) < 0;

    if (ballComingUp && this.reactionTimer <= 0) {
      const ballVelX = Math.cos(ball.Angle) * ball.Speed;
      const ballVelY = Math.sin(ball.Angle) * ball.Speed;

      // the hit box is in field space already (GetHitBox), same origin as Left / Top
      const aiPaddleCenterY = this.game.AiPaddle.Top + PADDLE_HEIGHT / 2;
      const ballCenterY = MidY(ball.HitBox);
      const timeToIntersect = (aiPaddleCenterY - ballCenterY) / ballVelY;

      if (timeToIntersect > 0) {
        const width = this.game.FieldWidth;
        let predicted = MidX(ball.HitBox) + ballVelX * timeToIntersect;
        while (predicted < 0 || predicted > width) {
          if (predicted < 0) predicted = -predicted;
          else if (predicted > width) predicted = 2 * width - predicted;
        }

        const maxError = (1 - this.accuracy) * PADDLE_WIDTH;
        const error = (rnd() * 2 - 1) * maxError;
        this.targetX = predicted + error - PADDLE_WIDTH / 2;
        this.reactionTimer = this.GetReactionTime();
        this.MoveTowardTarget();
      }
    } else if (!ballComingUp) {
      if (this.movementSmoothingTimer <= 0) {
        const centerX = (this.game.FieldWidth - PADDLE_WIDTH) / 2;
        if (Math.abs(this.game.AiPaddle.Left - centerX) > PADDLE_WIDTH) {
          this.targetX = centerX;
          this.MoveTowardTarget();
        } else if (this.isMoving && rnd() < 0.3) {
          this.SetMovement(0);
        }
        this.movementSmoothingTimer = this.movementSmoothingTime;
      }
    }
  }

  private ApplyMistakeMovement(): void {
    this.SetMovement(this.mistakeDirection < 0 ? -1 : this.mistakeDirection > 0 ? 1 : 0);
  }

  private MoveTowardTarget(): void {
    if (this.movementSmoothingTimer > 0) return;
    this.movementSmoothingTimer = this.movementSmoothingTime / 2;

    const dist = this.targetX - this.game.AiPaddle.Left;
    if (Math.abs(dist) < PADDLE_WIDTH * 0.15) { this.SetMovement(0); return; }
    this.SetMovement(dist < 0 ? -1 : 1);
  }

  private SetMovement(dir: number): void {
    if (dir === this.lastMovementDir) return;
    this.lastMovementDir = dir;
    this.isMoving = dir !== 0;
    this.game.SetAiMovement(dir);
  }

  private GetReactionTime(): number {
    return rnd() * (this.reactionTimeMax - this.reactionTimeMin) + this.reactionTimeMin;
  }
}

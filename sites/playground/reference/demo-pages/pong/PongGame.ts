import { DrawnGame, SkiaLabel, SkiaShape, Thickness, type GestureEventProcessingInfo, type SkiaControl, type SkiaGesturesParameters } from "drawnui-react/core";
import { PongAI } from "./PongAI";
import { BALL_SIZE, BALL_SPEED, BallSprite, IntersectsWith, MidX, PADDLE_HEIGHT, PADDLE_WIDTH, PaddleSprite } from "./Sprites";

// Ported from DrawnUi src/Shared/Samples/Pong.Shared/Game/PongGame.cs + PongGame.Loop.cs, the same game the
// MAUI, WPF, OpenTK, Blazor and pure-WASM heads run.

type GamePhase = "WaitingToStart" | "Playing" | "Scored" | "GameOver";
type Scorer = "Player" | "Ai";

export class PongGame extends DrawnGame {
  static readonly WIDTH = 360;
  static readonly HEIGHT = 640;
  static readonly PADDLE_SPEED = 420;
  static readonly PADDLE_MARGIN = 40;
  static readonly WIN_SCORE = 7;

  readonly Ball: BallSprite;
  readonly PlayerPaddle: PaddleSprite;
  readonly AiPaddle: PaddleSprite;
  /** Field width in points (C# `_game.Width` in the AI). */
  readonly FieldWidth = PongGame.WIDTH;

  private playerScore = 0;
  private aiScore = 0;
  private readonly ai: PongAI;
  private playerMovement = 0;
  private aiMovement = 0;
  private readonly scoreLabel: SkiaLabel;
  private readonly messageLabel: SkiaLabel;

  private phase: GamePhase = "WaitingToStart";
  private phaseTimer = 0;
  private aiServes = false;
  private autoServeTimer = 0;
  private aiWanderTimer = 0;
  private aiWanderDir = 0;
  private playerHasMoved = false;
  private lastScorer: Scorer = "Player";

  constructor() {
    super();
    const { WIDTH, HEIGHT, PADDLE_MARGIN } = PongGame;
    this.WidthRequest = WIDTH;
    this.HeightRequest = HEIGHT;
    this.HorizontalOptions = "Start";
    this.VerticalOptions = "Start";
    this.BackgroundColor = "#006400"; // Colors.DarkGreen
    this.Type = "Absolute";

    this.AddSubView(Object.assign(new SkiaShape(), {
      Type: "Rectangle", BackgroundColor: "#00000000", StrokeColor: "#FEFEFE", StrokeWidth: 2,
      HorizontalOptions: "Fill", VerticalOptions: "Fill",
    } as Partial<SkiaShape>));

    this.AiPaddle = Object.assign(new PaddleSprite("#FF2222"), { Left: (WIDTH - PADDLE_WIDTH) / 2, Top: PADDLE_MARGIN });
    this.PlayerPaddle = Object.assign(new PaddleSprite("#4CC9F0"), { Left: (WIDTH - PADDLE_WIDTH) / 2, Top: HEIGHT - PADDLE_MARGIN - PADDLE_HEIGHT });
    this.Ball = Object.assign(new BallSprite(), { Left: (WIDTH - BALL_SIZE) / 2, Top: HEIGHT - PADDLE_MARGIN - PADDLE_HEIGHT - BALL_SIZE });
    this.scoreLabel = Object.assign(new SkiaLabel(), {
      Text: "0 : 0", FontFamily: "FontGame", FontSize: 28, TextColor: "#FFFFFF",
      HorizontalOptions: "Center", VerticalOptions: "Start", Margin: new Thickness(0, HEIGHT / 2 - 24, 0, 0), HorizontalTextAlignment: "Center",
    } as Partial<SkiaLabel>);
    this.messageLabel = Object.assign(new SkiaLabel(), {
      Text: "TAP TO SERVE", FontFamily: "FontGame", FontSize: 14, TextColor: "#CCFFFFFF",
      HorizontalOptions: "Center", VerticalOptions: "Start", Margin: new Thickness(0, HEIGHT / 2 + 12, 0, 0), HorizontalTextAlignment: "Center",
    } as Partial<SkiaLabel>);
    for (const c of [this.AiPaddle, this.PlayerPaddle, this.Ball, this.scoreLabel, this.messageLabel]) this.AddSubView(c);

    this.ResetBall(true);
    this.ai = new PongAI(this, "Medium");

    this.StartLoop();
  }

  private ResetBall(playerServes: boolean): void {
    const { WIDTH, HEIGHT, PADDLE_MARGIN } = PongGame;
    this.Ball.Left = (WIDTH - BALL_SIZE) / 2;
    this.Ball.Top = playerServes ? HEIGHT - PADDLE_MARGIN - PADDLE_HEIGHT - BALL_SIZE : PADDLE_MARGIN + PADDLE_HEIGHT - 1;
    this.Ball.IsMoving = false;
    this.Ball.UpdateState(0, true);
    const angle = playerServes ? Math.PI / 2 + (Math.random() - 0.5) * 0.6 : -Math.PI / 2 + (Math.random() - 0.5) * 0.6;
    this.Ball.Angle = BallSprite.ClampAngleFromHorizontal(angle);
  }

  private ResetPaddles(): void {
    this.PlayerPaddle.Left = (PongGame.WIDTH - PADDLE_WIDTH) / 2;
    this.AiPaddle.Left = (PongGame.WIDTH - PADDLE_WIDTH) / 2;
    this.PlayerPaddle.UpdateState(0, true);
    this.AiPaddle.UpdateState(0, true);
  }

  private UpdateScoreLabel(): void { this.scoreLabel.Text = `${this.aiScore} : ${this.playerScore}`; }

  SetAiMovement(dir: number): void { this.aiMovement = dir; }
  SetPlayerMovement(dir: number): void { this.playerMovement = dir; }

  Serve(triggeredByAi = false): void {
    if (this.phase === "WaitingToStart" && (!this.aiServes || triggeredByAi)) {
      this.Ball.IsMoving = true;
      this.Ball.Speed = BALL_SPEED;
      this.phase = "Playing";
      this.messageLabel.Text = "";
      this.ai.ResetTimers();
    } else if (this.phase === "GameOver") {
      this.playerScore = 0;
      this.aiScore = 0;
      this.aiServes = false;
      this.playerHasMoved = false;
      this.aiWanderDir = 0;
      this.UpdateScoreLabel();
      this.ResetBall(true);
      this.ResetPaddles();
      this.phase = "WaitingToStart";
      this.messageLabel.Text = "TAP TO SERVE";
    }
  }

  override OnKeyDown(key: string): void {
    switch (key) {
      case "ArrowLeft": this.SetPlayerMovement(-1); break;
      case "ArrowRight": this.SetPlayerMovement(1); break;
      case "Space": case "ArrowUp": case "ArrowDown": case "Enter": this.Serve(); break;
    }
  }

  override OnKeyUp(key: string): void {
    if (key === "ArrowLeft" && this.playerMovement < 0) this.SetPlayerMovement(0);
    else if (key === "ArrowRight" && this.playerMovement > 0) this.SetPlayerMovement(0);
  }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Panning") {
      const velocityX = args.Event.Distance.Velocity.X / this.RenderingScale;
      this.SetPlayerMovement(Math.abs(velocityX) > 5 ? (velocityX > 0 ? 1 : -1) : 0);
    } else if (args.Type === "Tapped") {
      this.Serve();
    } else if (args.Type === "Up") {
      this.SetPlayerMovement(0);
    }
    return super.ProcessGestures(args, apply);
  }

  // ---- PongGame.Loop.cs ----

  override GameLoop(deltaSeconds: number): void {
    const { WIDTH, HEIGHT } = PongGame;

    if (this.phase === "WaitingToStart") {
      if (this.aiServes) {
        this.autoServeTimer -= deltaSeconds;
        this.Wander(deltaSeconds, 0.25, 0.25, 0.35);
        PongGame.MovePaddle(this.AiPaddle, this.aiWanderDir, deltaSeconds);
        PongGame.MovePaddle(this.PlayerPaddle, this.playerMovement, deltaSeconds);
        this.Ball.Left = this.AiPaddle.Left + (PADDLE_WIDTH - BALL_SIZE) / 2;
        if (this.autoServeTimer <= 0) this.Serve(true);
      } else {
        if (this.playerMovement !== 0) this.playerHasMoved = true;
        PongGame.MovePaddle(this.PlayerPaddle, this.playerMovement, deltaSeconds);
        this.Ball.Left = this.PlayerPaddle.Left + (PADDLE_WIDTH - BALL_SIZE) / 2;
        if (this.playerHasMoved) {
          this.Wander(deltaSeconds, 0.3, 0.5, 0.7);
          PongGame.MovePaddle(this.AiPaddle, this.aiWanderDir * 0.3, deltaSeconds);
        }
      }
      return;
    }

    if (this.phase === "Scored") {
      this.phaseTimer -= deltaSeconds;
      if (this.phaseTimer <= 0) {
        if (this.playerScore >= PongGame.WIN_SCORE || this.aiScore >= PongGame.WIN_SCORE) {
          this.phase = "GameOver";
          this.messageLabel.Text = this.playerScore >= PongGame.WIN_SCORE ? "YOU WIN!\nTAP TO RESTART" : "AI WINS!\nTAP TO RESTART";
        } else {
          const playerServes = this.lastScorer === "Player";
          this.aiServes = !playerServes;
          this.autoServeTimer = 1.5;
          this.playerHasMoved = false;
          this.aiWanderDir = 0;
          this.ResetPaddles();
          this.ResetBall(playerServes);
          this.phase = "WaitingToStart";
          this.messageLabel.Text = this.aiServes ? "" : "TAP TO SERVE";
        }
      }
      return;
    }

    if (this.phase === "GameOver") return;

    const time = Math.trunc(deltaSeconds * 1000);
    this.Ball.UpdateState(time, true);
    this.PlayerPaddle.UpdateState(time, true);
    this.AiPaddle.UpdateState(time, true);

    this.Ball.UpdatePosition(deltaSeconds);

    if (this.Ball.Left < 0) {
      this.Ball.Left = 0;
      this.Ball.Angle = Math.PI - this.Ball.Angle;
    } else if (this.Ball.Left + BALL_SIZE > WIDTH) {
      this.Ball.Left = WIDTH - BALL_SIZE;
      this.Ball.Angle = Math.PI - this.Ball.Angle;
    }

    this.Ball.UpdateState(time, true);
    const ballHit = this.Ball.HitBox;
    const MAX_DEV = Math.PI * 0.27;
    const MAX_SPEED = BALL_SPEED * 2;

    const playerHit = this.PlayerPaddle.HitBox;
    if (IntersectsWith(ballHit, playerHit) && this.Ball.Angle > 0) {
      const hitPos = (MidX(ballHit) - playerHit.Left) / playerHit.Width;
      this.Ball.Angle = BallSprite.ClampAngleFromHorizontal(-Math.PI / 2 + (hitPos - 0.5) * MAX_DEV * 2);
      if (Math.sin(this.Ball.Angle) > 0) this.Ball.Angle = -this.Ball.Angle;
      this.Ball.Top = playerHit.Top - BALL_SIZE;
      this.Ball.Speed = Math.min(this.Ball.Speed + 20, MAX_SPEED);
    }

    const aiHit = this.AiPaddle.HitBox;
    if (IntersectsWith(ballHit, aiHit) && this.Ball.Angle < 0) {
      const hitPos = (MidX(ballHit) - aiHit.Left) / aiHit.Width;
      this.Ball.Angle = BallSprite.ClampAngleFromHorizontal(Math.PI / 2 + (hitPos - 0.5) * MAX_DEV * 2);
      if (Math.sin(this.Ball.Angle) < 0) this.Ball.Angle = -this.Ball.Angle;
      this.Ball.Top = aiHit.Bottom;
      this.Ball.Speed = Math.min(this.Ball.Speed + 20, MAX_SPEED);
    }

    if (this.Ball.Top + BALL_SIZE < 0) {
      this.playerScore++;
      this.UpdateScoreLabel();
      this.Score("Player");
      return;
    }
    if (this.Ball.Top > HEIGHT) {
      this.aiScore++;
      this.UpdateScoreLabel();
      this.Score("Ai");
      return;
    }

    PongGame.MovePaddle(this.PlayerPaddle, this.playerMovement, deltaSeconds);
    PongGame.MovePaddle(this.AiPaddle, this.aiMovement, deltaSeconds);

    this.ai.Update(deltaSeconds);
  }

  /** The AI paddle drifting while waiting for a serve (both wander blocks of PongGame.Loop.cs). */
  private Wander(deltaSeconds: number, stayProbability: number, minSecs: number, rangeSecs: number): void {
    this.aiWanderTimer -= deltaSeconds;
    if (this.aiWanderTimer <= 0) {
      this.aiWanderDir = Math.random() < stayProbability ? 0 : Math.random() < 0.5 ? -1 : 1;
      this.aiWanderTimer = minSecs + Math.random() * rangeSecs;
    }
    if (this.AiPaddle.Left <= 0 && this.aiWanderDir < 0) this.aiWanderDir = 1;
    if (this.AiPaddle.Left + PADDLE_WIDTH >= PongGame.WIDTH && this.aiWanderDir > 0) this.aiWanderDir = -1;
  }

  private Score(scorer: Scorer): void {
    this.lastScorer = scorer;
    this.Ball.IsMoving = false;
    this.phase = "Scored";
    this.phaseTimer = 1.5;
    this.messageLabel.Text = scorer === "Player" ? "POINT!" : "AI SCORES!";
  }

  private static MovePaddle(paddle: PaddleSprite, dir: number, delta: number): void {
    if (dir === 0) return;
    const left = paddle.Left + dir * PongGame.PADDLE_SPEED * delta;
    paddle.Left = Math.max(0, Math.min(left, PongGame.WIDTH - PADDLE_WIDTH));
  }
}

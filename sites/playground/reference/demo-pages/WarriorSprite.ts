import { SkiaSpriteSet } from "drawnui-react/core";

export type WarriorAnimState = "IdleRight" | "IdleLeft" | "WalkRight" | "WalkLeft" | "WarRight" | "WarLeft";

/**
 * Port of the FastRepro WarriorSprite: maps warrior states to spritesheets and mirroring.
 * Subclassing SkiaSpriteSet keeps Source swaps atomic per state; geometry (Columns/Rows) and mirroring live here.
 */
export class WarriorSprite extends SkiaSpriteSet {
  private wstate: WarriorAnimState = "IdleRight";

  constructor(color: "Blue" | "Red" = "Blue") {
    super();
    this.Define(0, `anims/${color}Warrior/Warrior_Idle.png`, 8, 1, 15);
    this.Define(1, `anims/${color}Warrior/Warrior_Run.png`, 6, 1, 15);
    this.Define(2, `anims/${color}Warrior/Warrior_Attack1.png`, 4, 1, 8);
    this.WState = "IdleRight";
  }

  get WState(): WarriorAnimState { return this.wstate; }
  set WState(value: WarriorAnimState) {
    if (this.wstate === value) return;
    this.wstate = value;
    // base model: 0 = idle, 1 = walk, 2 = war
    this.State = value === "IdleLeft" || value === "IdleRight" ? 0 : value === "WalkLeft" || value === "WalkRight" ? 1 : 2;
    this.ApplyMirror();
  }

  private ApplyMirror(): void {
    const s = this.CurrentSprite;
    if (!s) return;
    const mirror = this.wstate === "IdleLeft" || this.wstate === "WalkLeft" || this.wstate === "WarLeft";
    s.ScaleX = mirror ? -1 : 1;
  }

  protected override OnChangeState(oldState: number, newState: number): void {
    super.OnChangeState(oldState, newState);
    this.ApplyMirror();
  }
}

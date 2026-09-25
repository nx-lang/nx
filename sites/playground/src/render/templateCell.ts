/**
 * The cell a templated control's factory makes: a host control whose content is what the
 * template function renders for the item DrawnUI binds to it.
 *
 * <para>DrawnUI decides which cells exist, realizes them through the factory, recycles them and
 * measures them by the strategy the author set; none of that is here. What is here is the one
 * step DrawnUI leaves to the template: whenever it binds the cell to an item, the function is
 * called with `Item` and `Index` and its result is materialized inside the cell. A recycled cell
 * is rebound and so redrawn; a failing call is reported with the index and leaves the cell empty,
 * so the rest of the list still draws.</para>
 */
import { callFunction, type NxCanonicalValue } from "@nx-lang/ir-runtime";
import { SkiaLayout } from "drawnui-react/core";
import type { Program } from "./evaluate";
import { materialize, type MaterializeContext } from "./materialize";
import type { FunctionRecord } from "./values";

export interface TemplateCellContext extends MaterializeContext {
  /** Reported once per failing call, with the index of the item it was bound to. */
  readonly reportTemplateFailure: (where: string) => void;
}

export class NxTemplateCell extends SkiaLayout {
  readonly #program: Program;
  readonly #template: FunctionRecord;
  readonly #params: readonly string[];
  readonly #context: TemplateCellContext;

  constructor(program: Program, template: FunctionRecord, params: readonly string[], context: TemplateCellContext) {
    super();
    this.Type = "Absolute";
    this.HorizontalOptions = "Fill";
    this.#program = program;
    this.#template = template;
    this.#params = params;
    this.#context = context;
  }

  /** The item this cell shows, once bound: what `Item` was. */
  get Item(): unknown {
    return this.BindingContext;
  }

  protected override OnBindingContextChanged(): void {
    for (const view of [...this.Views]) {
      this.RemoveSubView(view);
    }
    if (this.BindingContext === undefined) {
      this.InvalidateMeasure();
      return;
    }
    const args: Record<string, NxCanonicalValue> = {};
    const [itemParam, indexParam] = this.#params;
    if (itemParam !== undefined) {
      args[itemParam] = this.BindingContext as NxCanonicalValue;
    }
    if (indexParam !== undefined) {
      args[indexParam] = this.ContextIndex;
    }
    try {
      const rendered = callFunction(this.#program, this.#template as unknown as NxCanonicalValue, args);
      for (const control of materialize(rendered as never, this.#context)) {
        this.AddSubView(control);
      }
    } catch (error) {
      this.#context.reportTemplateFailure(
        `${this.#template.name} at index ${this.ContextIndex}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
    this.InvalidateMeasure();
  }
}

/** The factory a templated control's `ItemTemplate` takes: one new cell per call. */
export function templateFactory(
  program: Program,
  template: FunctionRecord,
  params: readonly string[],
  context: TemplateCellContext,
): () => NxTemplateCell {
  return () => new NxTemplateCell(program, template, params, context);
}

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { NxHostRecord } from "@nx-lang/ir-runtime";
import "@nx-lang/value-view";
import {
  NX_VALUE_ACTION_EVENT,
  NX_VALUE_NAVIGATE_EVENT,
  type NxValueActionDetail,
  type NxValueActions,
  type NxValueDescribe,
  type NxValueElement,
  type NxValueNavigateDetail,
} from "@nx-lang/value-view";
import type { Diagnostic } from "../compile";
import { EDITOR_THEMES } from "../theme.ts";
import { actionFrom, initialInput, type FieldInput } from "./fields.ts";
import type { PreviewAction, PreviewField, PreviewSessionView, PreviewStep, PreviewValue } from "./protocol.ts";
import type { Preview } from "./usePreview.ts";

export interface PreviewViewProps {
  readonly preview: Preview;
  /** The source does not compile and never has, so there is nothing to run. */
  readonly doesNotCompile: boolean;
  /** The editor holds source the session does not run: an edit broke compilation or the props. */
  readonly stale: boolean;
  readonly describe: NxValueDescribe;
  /** Shows a node's source: where it was built when `built`, otherwise its declaration. */
  readonly onNavigate: (detail: NxValueNavigateDetail, built: boolean) => void;
}

/** An action form the visitor opened from a handler's button. */
interface OpenForm {
  readonly handler: string;
  readonly action: PreviewAction;
}

/**
 * One component of the visitor's source, running: its output with a button on every handler, the
 * timeline of steps, and its state and what it emitted.
 */
export function PreviewView({ preview, doesNotCompile, stale, describe, onNavigate }: PreviewViewProps) {
  const { state, notice, failure } = preview;
  const session = state?.session ?? null;
  const [form, setForm] = useState<OpenForm | null>(null);

  // A form belongs to the output it was opened from.
  const current = session?.current;
  useEffect(() => setForm(null), [current]);

  const press = (handler: string, type: string) => {
    const action = session?.actions[type];
    if (action === undefined) {
      return;
    }
    if (action.fields.length === 0) {
      setForm(null);
      preview.dispatch(handler, { $type: type });
    } else {
      setForm({ handler, action });
    }
  };

  const problem = state?.problem ?? null;
  const dispatchFailure = problem?.kind === "dispatchFailed" ? problem.diagnostics : null;

  return (
    <div className="preview">
      {state !== null && state.components.length > 0 && (
        <div className="preview-bar">
          <label className="preview-picker">
            <span>Component</span>
            <select
              value={state.component ?? ""}
              onChange={(event) => {
                setForm(null);
                preview.start(event.target.value);
              }}
            >
              {state.components.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          {session !== null && <Timeline session={session} onGoTo={preview.goTo} />}
        </div>
      )}
      {failure !== null && (
        <div className="output-message fault" role="alert">
          <p>{failure}</p>
          <button type="button" className="preview-button" onClick={preview.retry}>
            Start again
          </button>
        </div>
      )}
      {state === null && failure === null && !doesNotCompile && <p className="output-message quiet">Starting…</p>}
      {(doesNotCompile || problem?.kind === "doesNotCompile") && session === null && (
        <p className="output-message">
          The program does not compile yet, so there is nothing to preview. The problems are listed under the
          source.
        </p>
      )}
      {problem?.kind === "noComponent" && (
        <p className="output-message">
          There is no component to preview. Declare one, such as{" "}
          <code>component &lt;Counter /&gt; = &#123; … &#125;</code>, and Preview runs it here, with buttons for
          its handlers.
        </p>
      )}
      {problem?.kind === "programFailed" && (
        <Failure heading="The program could not be prepared to run" diagnostics={problem.diagnostics} />
      )}
      {problem?.kind === "startFailed" && (
        <Failure
          heading={`${state?.component ?? "The component"} would not start`}
          diagnostics={problem.diagnostics}
          note={
            problem.missingProp ? (
              <>
                End the program with the component's element, with its props, such as{" "}
                <code>&lt;{state?.component ?? "Counter"} … /&gt;</code>, and Preview runs it with them.
              </>
            ) : undefined
          }
        />
      )}
      {notice !== null && <ReloadNotice notice={notice} steps={session?.path.length ?? 0} />}
      {stale && session !== null && (
        <p className="output-message quiet">
          The preview runs the last program that compiled and rendered; it updates when the source does.
        </p>
      )}
      {form !== null && (
        <ActionForm
          key={`${form.handler} ${form.action.type}`}
          form={form}
          onSend={(action) => preview.dispatch(form.handler, action)}
          onCancel={() => setForm(null)}
          diagnostics={dispatchFailure}
        />
      )}
      {form === null && dispatchFailure !== null && <Failure heading="The step failed" diagnostics={dispatchFailure} />}
      {session !== null && (
        <SessionOutput
          session={session}
          stale={stale}
          describe={describe}
          onPress={press}
          onNavigate={onNavigate}
        />
      )}
      {session !== null && <SessionState session={session} stale={stale} describe={describe} />}
    </div>
  );
}

/** The steps from the start to the current one, with Back and Forward and a way into each branch. */
function Timeline({ session, onGoTo }: { session: PreviewSessionView; onGoTo: (tick: number) => void }) {
  const { path, current, forward } = session;
  const parent = path.length > 1 ? path[path.length - 2]!.id : null;
  const strip = useRef<HTMLOListElement>(null);

  // The newest step is the one to see.
  useEffect(() => {
    const list = strip.current;
    if (list !== null) {
      list.scrollLeft = list.scrollWidth;
    }
  }, [current]);

  return (
    <nav className="timeline" aria-label="Steps">
      <button
        type="button"
        className="preview-button"
        onClick={() => parent !== null && onGoTo(parent)}
        disabled={parent === null}
        aria-label="Back a step"
        title="Back a step"
      >
        ‹
      </button>
      <button
        type="button"
        className="preview-button"
        onClick={() => forward !== null && onGoTo(forward)}
        disabled={forward === null}
        aria-label="Forward a step"
        title="Forward a step"
      >
        ›
      </button>
      <ol ref={strip} className="timeline-steps">
        {path.map((step, index) => (
          <TimelineStep
            key={step.id}
            step={step}
            number={index + 1}
            next={path[index + 1]?.id}
            current={step.id === current}
            onGoTo={onGoTo}
          />
        ))}
      </ol>
    </nav>
  );
}

function TimelineStep({
  step,
  number,
  next,
  current,
  onGoTo,
}: {
  step: PreviewStep;
  number: number;
  next: number | undefined;
  current: boolean;
  onGoTo: (tick: number) => void;
}) {
  return (
    <li className={`timeline-step kind-${step.kind}`}>
      <button
        type="button"
        aria-current={current ? "step" : undefined}
        onClick={() => onGoTo(step.id)}
        title={`Step ${number}: ${step.label}`}
      >
        <span className="timeline-number">{number}</span> {step.label}
      </button>
      {step.children.length > 1 && (
        <select
          className="timeline-branches"
          aria-label={`${step.children.length} branches from step ${number}`}
          title={`${step.children.length} branches leave this step`}
          value={next ?? ""}
          onChange={(event) => onGoTo(Number(event.target.value))}
        >
          {next === undefined && (
            <option value="" disabled>
              ⑂ {step.children.length}
            </option>
          )}
          {step.children.map((child, index) => (
            <option key={child.id} value={child.id}>
              {child.id === next ? "⑂ " : ""}
              {index + 1}. {child.label}
            </option>
          ))}
        </select>
      )}
    </li>
  );
}

/** The current output, with a button on each handler. */
function SessionOutput({
  session,
  stale,
  describe,
  onPress,
  onNavigate,
}: {
  session: PreviewSessionView;
  stale: boolean;
  describe: NxValueDescribe;
  onPress: (handler: string, type: string) => void;
  onNavigate: (detail: NxValueNavigateDetail, built: boolean) => void;
}) {
  const element = useRef<NxValueElement>(null);
  const themes = EDITOR_THEMES;
  const latest = useRef({ onPress, onNavigate, session });
  latest.current = { onPress, onNavigate, session };

  const handlers = useMemo(() => new Map(session.handlers.map((entry) => [entry.pointer, entry.action])), [session]);
  const actions = useMemo<NxValueActions>(
    () => (node) => {
      const type = node.role === "record" && node.pointer !== undefined ? handlers.get(node.pointer) : undefined;
      const action = type === undefined ? undefined : session.actions[type];
      return action === undefined
        ? undefined
        : [{ id: type!, label: action.label, title: `Send ${action.type} to this handler` }];
    },
    [handlers, session],
  );

  useEffect(() => {
    const target = element.current;
    if (target === null) {
      return;
    }
    const onAction = (event: Event) => {
      const { node, id } = (event as CustomEvent<NxValueActionDetail>).detail;
      if (node.pointer !== undefined) {
        latest.current.onPress(node.pointer, id);
      }
    };
    const onNavigateEvent = (event: Event) => {
      const detail = (event as CustomEvent<NxValueNavigateDetail>).detail;
      latest.current.onNavigate(detail, latest.current.session.built.includes(detail.index));
    };
    target.addEventListener(NX_VALUE_ACTION_EVENT, onAction);
    target.addEventListener(NX_VALUE_NAVIGATE_EVENT, onNavigateEvent);
    return () => {
      target.removeEventListener(NX_VALUE_ACTION_EVENT, onAction);
      target.removeEventListener(NX_VALUE_NAVIGATE_EVENT, onNavigateEvent);
    };
  });

  const { output } = session;
  if ("diagnostics" in output) {
    return <Failure heading="The output has no NX spelling" diagnostics={output.diagnostics} />;
  }
  return (
    <nx-value
      ref={element}
      value={output.value}
      truncated={output.truncated}
      stale={stale}
      describe={describe}
      actions={actions}
      themes={themes}
    />
  );
}

/** The current step's state, and what it emitted to its host. */
function SessionState({ session, stale, describe }: { session: PreviewSessionView; stale: boolean; describe: NxValueDescribe }) {
  return (
    <div className="preview-panels">
      <section className="preview-panel" aria-label="State">
        <h3>State</h3>
        {session.state === null ? (
          <p className="preview-empty">The component keeps no state.</p>
        ) : (
          <PanelValue value={session.state} stale={stale} describe={describe} />
        )}
      </section>
      <section className="preview-panel" aria-label="Emitted">
        <h3>Emitted</h3>
        {session.emitted.length === 0 ? (
          <p className="preview-empty">This step emitted nothing to the host.</p>
        ) : (
          session.emitted.map((value, index) => <PanelValue key={index} value={value} stale={stale} describe={describe} />)
        )}
      </section>
    </div>
  );
}

function PanelValue({ value, stale, describe }: { value: PreviewValue; stale: boolean; describe: NxValueDescribe }) {
  const themes = EDITOR_THEMES;
  return (
    <nx-value
      className="panel-value"
      value={value.value}
      truncated={value.truncated}
      stale={stale}
      describe={describe}
      themes={themes}
    />
  );
}

/** A form for an action with fields, one input per field. */
function ActionForm({
  form,
  onSend,
  onCancel,
  diagnostics,
}: {
  form: OpenForm;
  onSend: (action: NxHostRecord) => void;
  onCancel: () => void;
  diagnostics: readonly Diagnostic[] | null;
}) {
  const { action } = form;
  const [inputs, setInputs] = useState<Record<string, FieldInput>>(() =>
    Object.fromEntries(action.fields.map((field) => [field.name, initialInput(field)])),
  );
  const [invalid, setInvalid] = useState<{ field: string; message: string } | null>(null);

  const submit = () => {
    const made = actionFrom(action.type, action.fields, inputs);
    if ("field" in made) {
      setInvalid(made);
      return;
    }
    setInvalid(null);
    onSend(made.action);
  };

  return (
    <form
      className="action-form"
      aria-label={`Send ${action.type}`}
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <p className="action-form-title">
        Send <code>{action.type}</code>
      </p>
      {action.fields.map((field) => (
        <FieldInputRow
          key={field.name}
          field={field}
          value={inputs[field.name] ?? initialInput(field)}
          invalid={invalid?.field === field.name}
          onChange={(value) => setInputs((previous) => ({ ...previous, [field.name]: value }))}
        />
      ))}
      {invalid !== null && (
        <p className="action-form-error" role="alert">
          {invalid.message}
        </p>
      )}
      {diagnostics !== null && <Failure heading="The step failed" diagnostics={diagnostics} />}
      <div className="action-form-buttons">
        <button type="submit" className="preview-button primary">
          Send
        </button>
        <button type="button" className="preview-button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function FieldInputRow({
  field,
  value,
  invalid,
  onChange,
}: {
  field: PreviewField;
  value: FieldInput;
  invalid: boolean;
  onChange: (value: FieldInput) => void;
}) {
  const label = (
    <span className="field-name">
      {field.name}
      {field.required ? "" : " (optional)"}
    </span>
  );
  const common = { "aria-invalid": invalid || undefined };
  switch (field.kind) {
    case "boolean":
      return (
        <label className="field field-check">
          <input type="checkbox" checked={value === true} onChange={(event) => onChange(event.target.checked)} {...common} />
          {label}
        </label>
      );
    case "cases":
      return (
        <label className="field">
          {label}
          <select value={String(value)} onChange={(event) => onChange(event.target.value)} {...common}>
            {!field.required && <option value="">(its default)</option>}
            {field.cases.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
      );
    case "integer":
    case "number":
      return (
        <label className="field">
          {label}
          <input
            type="number"
            step={field.kind === "integer" ? 1 : "any"}
            value={String(value)}
            onChange={(event) => onChange(event.target.value)}
            {...common}
          />
        </label>
      );
    case "json":
      return (
        <label className="field">
          {label}
          <textarea
            rows={2}
            spellCheck={false}
            placeholder="JSON"
            value={String(value)}
            onChange={(event) => onChange(event.target.value)}
            {...common}
          />
        </label>
      );
    default:
      return (
        <label className="field">
          {label}
          <input type="text" value={String(value)} onChange={(event) => onChange(event.target.value)} {...common} />
        </label>
      );
  }
}

/** What the last reload did, when it did more than keep the state. */
function ReloadNotice({ notice, steps }: { notice: NonNullable<Preview["notice"]>; steps: number }) {
  if (notice.kind === "failed") {
    return (
      <Failure
        heading="The edit could not run the current props"
        diagnostics={notice.diagnostics}
        note="The preview keeps the last output until it can."
      />
    );
  }
  if (notice.completed) {
    return (
      <p className="output-message quiet" role="status">
        The state no longer fit the edit, so the run was replayed under it.
      </p>
    );
  }
  return (
    <Failure
      heading={`The state no longer fit the edit, so the run was replayed under it, and stopped after step ${notice.steps || steps}`}
      diagnostics={notice.diagnostics}
    />
  );
}

function Failure({
  heading,
  diagnostics,
  note,
}: {
  heading: string;
  diagnostics: readonly Diagnostic[];
  note?: ReactNode;
}) {
  return (
    <div className="output-message error" role="alert">
      <p className="output-heading">{heading}</p>
      {diagnostics.map((diagnostic, index) => (
        <p key={index}>
          {diagnostic.span !== null && (
            <span className="where">
              {diagnostic.span.startLine}:{diagnostic.span.startColumn}
            </span>
          )}
          {diagnostic.message}
        </p>
      ))}
      {note !== undefined && <p>{note}</p>}
    </div>
  );
}

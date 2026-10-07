/**
 * The steps the performance harness times: each call a host makes on the conformance corpus's
 * large programs, and calls that take a large input.
 *
 * This module is the part of the harness that runs anywhere. It loads no other module, reads no
 * clock and uses nothing of Node, so a host can load it into its own engine, hand it its own build
 * of `@nx-lang/ir-runtime` and the corpus images, and time the steps however that engine allows:
 * an engine whose clock stands still while code runs times them from outside. `run.mjs` is the
 * driver that times them in Node.
 *
 * `buildSteps(runtime, corpus)` returns the steps in report order. A step is:
 *
 * - `workload`, `phase`, `subject`, `variant`: what it is. `variant` is `unlimited`, a call with
 *   no limits, or `limited`, the same call with `maxOperations`, `maxInputSize` and `usage` set
 *   to values it does not reach. A phase that evaluates nothing has only `unlimited`.
 * - `ready()`: does, once and outside what is timed, whatever the call needs: preparing, linking,
 *   initializing, building its input. Calling it is optional; `run` does it if it was not done.
 * - `run()`: makes the call once and returns what the runtime's usage report gave,
 *   `{ operations, inputSize }`, both absent for `unlimited`. A dispatch step runs its whole
 *   script, one call per batch, and returns the totals.
 * - `first()`, on a dispatch step only: makes the first call of the script alone, which is what
 *   the first request of an isolate does. Elsewhere the first call is `run`.
 * - `divisor`: what one run is divided by to report one call: the batches of a dispatch script,
 *   and 1 for everything else.
 * - `recorded`: the operations the corpus records for the step, where it records them.
 * - `programs`: the corpus programs the step runs, and `requires`: the functions of the runtime it
 *   calls. A comparison of two revisions reads both to say when a step cannot be compared.
 */

/** The corpus programs the steps run. `corpus` holds each by name. */
export const PROGRAMS = ["large-catalog", "question-flow", "host-values", "agent-tool-context"];

/** The programs whose every phase is timed; the others only lend a function to a call that takes input. */
const TIMED_PROGRAMS = ["large-catalog", "question-flow"];

/** How many batches of a lifecycle have been dispatched when its state is the one a host stored. */
export const STORED_AFTER = 15;

/** How many records a call of the `input` phase is given. */
export const INPUT_RECORDS = 10_000;

/** How many objects the nested input is inside: more than the runtime's check for plain data follows. */
const INPUT_NESTING = 33;

/** A budget and an input limit no step reaches: what a host sets to read what a call used and limit nothing. */
const AMPLE = 2 ** 40;

function lazy(make) {
  let made = false;
  let value;
  return () => {
    if (!made) {
      value = make();
      made = true;
    }
    return value;
  };
}

/**
 * The two variants of a step that evaluates. `call(options)` makes the runtime call with the
 * options of its variant, which are `undefined` for the call with no limits.
 */
function evaluating(step, call) {
  return [
    { ...step, variant: "unlimited", run: () => (step.ready(), call(undefined), {}) },
    {
      ...step,
      variant: "limited",
      run: () => {
        step.ready();
        const usage = {};
        call({ maxOperations: AMPLE, maxInputSize: AMPLE, usage });
        return { operations: usage.operations, inputSize: usage.inputSize };
      },
    },
  ];
}

/** The state `lifecycle`'s component holds once its first `count` batches have been dispatched. */
function stateAfter(runtime, linked, lifecycle, count) {
  let { instance, state } = runtime.initializeComponent(linked, lifecycle.component, lifecycle.props ?? {});
  for (const batch of lifecycle.batches.slice(0, count)) {
    ({ instance, state } = runtime.dispatchComponentActions(linked, instance, batch));
  }
  return state;
}

function prepareAll(runtime, program) {
  const modules = new Map();
  for (const identity of program.manifest.emit) {
    modules.set(identity, runtime.prepareNxIrModule(program.images[identity]));
  }
  return modules;
}

/** Links a program's entry against its own prepared modules, as the corpus tests do. */
function link(runtime, program, modules) {
  return runtime.linkNxIrProgram(modules.get(program.manifest.entry), { resolve: (identity) => modules.get(identity) });
}

/**
 * The steps of one corpus program: preparing and linking it, evaluating each entrypoint of its
 * entry module that takes no arguments, and each lifecycle's initialization, resumption,
 * evaluation with a state and dispatch script.
 */
function programSteps(runtime, name, program, stored) {
  const { manifest } = program;
  const counts = program.operations?.counts ?? {};
  const modules = lazy(() => prepareAll(runtime, program));
  const linked = lazy(() => link(runtime, program, modules()));
  const about = { workload: name, programs: [name], divisor: 1 };
  const steps = [
    {
      ...about,
      phase: "prepare",
      subject: `${manifest.emit.length} images`,
      variant: "unlimited",
      requires: ["prepareNxIrModule"],
      ready: () => {},
      run: () => (prepareAll(runtime, program), {}),
    },
    {
      ...about,
      phase: "link",
      subject: manifest.entry,
      variant: "unlimited",
      requires: ["prepareNxIrModule", "linkNxIrProgram"],
      ready: modules,
      run: () => (link(runtime, program, modules()), {}),
    },
  ];
  const evaluates = ["prepareNxIrModule", "linkNxIrProgram"];
  for (const entrypoint of manifest.entrypoints) {
    if (entrypoint.module !== manifest.entry || entrypoint.arguments !== undefined) {
      continue;
    }
    steps.push(
      ...evaluating(
        {
          ...about,
          phase: "evaluate",
          subject: entrypoint.function,
          recorded: counts[`${entrypoint.module}::${entrypoint.function}`],
          requires: [...evaluates, "evaluateFunction"],
          ready: linked,
        },
        (options) => runtime.evaluateFunction(linked(), entrypoint.function, [], options),
      ),
    );
  }
  for (const lifecycle of manifest.lifecycles ?? []) {
    if (lifecycle.module !== manifest.entry) {
      continue;
    }
    const { component, batches } = lifecycle;
    const props = lifecycle.props ?? {};
    const recorded = counts[`${lifecycle.module}::${component}`];
    const initial = lazy(() => runtime.initializeComponent(linked(), component, props).instance);
    const state = lazy(() => stored?.[name]?.[component] ?? stateAfter(runtime, linked(), lifecycle, STORED_AFTER));
    const lifecycleStep = { ...about, subject: component };
    steps.push(
      ...evaluating(
        {
          ...lifecycleStep,
          phase: "initialize",
          recorded: recorded?.initial,
          requires: [...evaluates, "initializeComponent"],
          ready: linked,
        },
        (options) => runtime.initializeComponent(linked(), component, props, options),
      ),
      ...evaluating(
        {
          ...lifecycleStep,
          phase: "resume",
          requires: [...evaluates, "initializeComponent", "dispatchComponentActions"],
          ready: () => (linked(), state()),
        },
        (options) => runtime.initializeComponent(linked(), component, props, { ...options, state: state() }),
      ),
      ...evaluating(
        {
          ...lifecycleStep,
          phase: "evaluate with state",
          requires: [...evaluates, "initializeComponent", "dispatchComponentActions", "evaluateComponent"],
          ready: () => (linked(), state()),
        },
        (options) => runtime.evaluateComponent(linked(), component, props, state(), options),
      ),
    );
    // The script runs from the initial instance every time: an instance is never modified, and
    // the tokens the script names are those of the generations that follow it.
    const dispatch = {
      ...lifecycleStep,
      phase: "dispatch",
      divisor: batches.length,
      recorded: recorded?.batches.reduce((sum, count) => sum + count, 0),
      requires: [...evaluates, "initializeComponent", "dispatchComponentActions"],
      ready: initial,
      first: () => (runtime.dispatchComponentActions(linked(), initial(), batches[0]), {}),
    };
    steps.push(
      {
        ...dispatch,
        variant: "unlimited",
        run: () => {
          let instance = initial();
          for (const batch of batches) {
            ({ instance } = runtime.dispatchComponentActions(linked(), instance, batch));
          }
          return {};
        },
      },
      {
        ...dispatch,
        variant: "limited",
        run: () => {
          let instance = initial();
          let operations = 0;
          let inputSize = 0;
          for (const batch of batches) {
            const usage = {};
            ({ instance } = runtime.dispatchComponentActions(linked(), instance, batch, {
              maxOperations: AMPLE,
              maxInputSize: AMPLE,
              usage,
            }));
            operations += usage.operations;
            inputSize += usage.inputSize;
          }
          return { operations, inputSize };
        },
      },
    );
  }
  return steps;
}

/** Records at `object`: three fields each, of the three scalar kinds. */
function rows(count) {
  const rows = [];
  for (let index = 0; index < count; index += 1) {
    rows.push({ id: index, label: `row ${index}`, done: index % 2 === 0 });
  }
  return rows;
}

/** Records of a declared type, `ChoiceAnswer` of `question-flow`: checked field by field. */
function answers(count) {
  const answers = [];
  for (let index = 0; index < count; index += 1) {
    answers.push({ $type: "ChoiceAnswer", questionId: `q${index}`, selected: `option ${index % 7}` });
  }
  return answers;
}

/** `records` with a member set to `undefined` in the last of them, which a reading finds only at the end. */
function withUndefinedMember(records, name) {
  return [...records.slice(0, -1), { ...records[records.length - 1], [name]: undefined }];
}

function nested(value, depth) {
  let nested = value;
  for (let level = 0; level < depth; level += 1) {
    nested = { inner: nested };
  }
  return nested;
}

/**
 * The steps of calls that take input, run against functions the corpus already has: a large
 * value at `object` that is returned, the same number of typed records that are only checked,
 * and a function called by name with arguments and a context record, as `@nx-lang/agent` calls a
 * tool's function.
 */
function callSteps(runtime, corpus) {
  const linkedProgram = (name) => lazy(() => link(runtime, corpus[name], prepareAll(runtime, corpus[name])));
  const evaluates = ["prepareNxIrModule", "linkNxIrProgram"];
  const steps = [];

  const input = (programName, name, shape, build) => {
    if (corpus[programName] === undefined) {
      return;
    }
    const linked = linkedProgram(programName);
    const value = lazy(build);
    steps.push(
      ...evaluating(
        {
          workload: "calls",
          phase: "input",
          subject: `${name}, ${shape}`,
          divisor: 1,
          programs: [programName],
          requires: [...evaluates, "evaluateFunction"],
          ready: () => (linked(), value()),
        },
        (options) => runtime.evaluateFunction(linked(), name, [value()], options),
      ),
    );
  };
  input("host-values", "held", "plain", () => rows(INPUT_RECORDS));
  input("host-values", "held", "undefined member", () => withUndefinedMember(rows(INPUT_RECORDS), "note"));
  input("host-values", "held", "nested", () => nested(rows(INPUT_RECORDS), INPUT_NESTING));
  input("question-flow", "received", "plain", () => answers(INPUT_RECORDS));
  input("question-flow", "received", "undefined member", () => withUndefinedMember(answers(INPUT_RECORDS), "other"));

  const toolCall = (name, args) => {
    const program = corpus["agent-tool-context"];
    if (program === undefined) {
      return;
    }
    const linked = linkedProgram("agent-tool-context");
    const record = { $type: "Function", module: program.manifest.entry, name };
    steps.push(
      ...evaluating(
        {
          workload: "calls",
          phase: "tool call",
          subject: name,
          divisor: 1,
          programs: ["agent-tool-context"],
          requires: [...evaluates, "callFunction"],
          ready: linked,
        },
        (options) => runtime.callFunction(linked(), record, args, options),
      ),
    );
  };
  toolCall("seatCount", { teamSize: 12, cap: 40 });
  toolCall("whoAmI", {
    context: { $type: "ChatToolContext", callId: "call-1", conversationId: "conversation-1", contactEmail: "ada@example.com" },
  });
  return steps;
}

/**
 * The steps, in report order. `runtime` is the runtime module, every export of it. `corpus` holds
 * each program of {@link PROGRAMS} by name as `{ manifest, images, operations }`: its
 * `program.json`, its images by module identity as `Uint8Array`, and, optionally, its
 * `operations.json`, which gives the steps their `recorded` counts. A program `corpus` lacks has
 * no steps. `stored`, optional, is what {@link storedStates} returned, here or elsewhere: with it
 * a step that starts from a stored state does not dispatch anything to make one, which is how a
 * host that kept only the state runs.
 */
export function buildSteps(runtime, corpus, stored) {
  return [
    ...TIMED_PROGRAMS.flatMap((name) => (corpus[name] === undefined ? [] : programSteps(runtime, name, corpus[name], stored))),
    ...callSteps(runtime, corpus),
  ];
}

/**
 * The state each lifecycle's component holds after {@link STORED_AFTER} batches, by program and
 * component: plain data, as a host stores it and passes it back.
 */
export function storedStates(runtime, corpus) {
  const states = {};
  for (const name of TIMED_PROGRAMS) {
    const program = corpus[name];
    const lifecycles = (program?.manifest.lifecycles ?? []).filter((lifecycle) => lifecycle.module === program.manifest.entry);
    if (lifecycles.length === 0) {
      continue;
    }
    const linked = link(runtime, program, prepareAll(runtime, program));
    states[name] = {};
    for (const lifecycle of lifecycles) {
      states[name][lifecycle.component] = stateAfter(runtime, linked, lifecycle, STORED_AFTER);
    }
  }
  return states;
}

/** A step's name in a report: `workload / phase / subject`. With its variant it names one step. */
export function stepName(step) {
  return `${step.workload} / ${step.phase} / ${step.subject}`;
}

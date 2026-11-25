//! A differential stress test of the instance tree: seeded random sequences of root changes,
//! dispatches and passes, after each of which every node's rendered output must be the one a
//! fresh evaluation gives.
//!
//! <para>The tree keeps renders and derived results across operations and decides from its
//! records what to evaluate again. A record that misses a read keeps output a fresh evaluation
//! would not give. The reference here keeps nothing: after each step and the pass that follows
//! it, it initializes every node through the bare lifecycle, with the root values, the place's
//! overrides and the state the tree holds, and compares the outputs with their handler tokens left
//! out. It also checks that the change set lists every node whose output changed.</para>
//!
//! <para>`NX_TREE_STRESS_SEEDS` sets how many seeds run (default 20) and
//! `NX_TREE_STRESS_ITERATIONS` how many steps each runs (default 40); `NX_TREE_STRESS_SEED` runs
//! that one seed alone. A failure names the seed, the steps that led to it and every node whose
//! output differs. With `NX_TREE_STRESS_ALL` set, a seed goes on past a node that differs, and the
//! end sums up, by node, how many seeds found it differing and the first difference found. Outputs
//! compare as a host sees them written, so `0.0` and `-0.0` differ.</para>

use crate::ir_runtime_tests::program;
use nx_ir_runtime::{ComponentInit, ComponentInstance, Environment, InstanceTree, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;
use std::sync::Arc;

const CONTROLS: &str = "
abstract external component <Node />
external component <Label extends Node Text?:string />
external component <Num extends Node Value?:float64 />
external component <Any extends Node Value?:object />
external component <Button extends Node Text?:string emits { Tapped { } } />
external component <Stack extends Node content Children?:Node+ />
";

/// Every risky feature at once: element overrides and `provide` blocks, an entry-module bundle
/// with a formula entry, formulas the body provides (one reading its own value), field
/// overrides, a prop default that reads `env`, state the body reads and state only a handler
/// reads, light and dark sections, kept top-level values that read `env`, conditional reads,
/// and values a host can write differently that compare equal.
const PROGRAM: &str = "
type Mode = light | dark
type Colors = { accent:string tone:string }
environment mode: Mode = light
environment locale: string = \"en\"
environment region: string = \"eu\"
environment space: int = 1
environment gap: int = 0
environment z: float64 = 1.0
environment amount: object = 1
type Box = { v:float64 }
environment boxed: Box = { <Box v={env.z} /> }
environment colors: Colors = {
  if env.mode is { light => <Colors accent=\"blue\" tone=\"t\" /> dark => <Colors accent=\"navy\" tone=\"t\" /> }
}
environment loud: string = { modeName(env.mode) + \"!\" + env.locale + \"!\" + env.region }
environment label: string = { if env.space > 2 { \"wide\" } else { env.locale } }
provide env with <env.Update gap={env.space * 2} />
let direction: string = { if env.locale == \"ar\" { \"rtl\" } else { \"ltr\" } }
let accentOf: string = { env.colors.accent + \"/\" + direction }
let held(value:object): object = { value }
let amountOf: object = { held(env.amount) }
let modeName(m:Mode): string = { if m is { dark => \"dark\" light => \"light\" } }
let regionOf(m:Mode, l:string): string = { if m is { dark => l light => \"fixed\" } }

component <Show extends Node text:string = { env.label } /> = {
  state { taps:int = 0 hidden:int = 0 }
  <Stack>
    <Label Text={text + \" \" + taps} />
    <Button Text=\"Tap\" onTapped=<State.Update taps={taps + 1} /> />
    <Button Text=\"Hide\" onTapped=<State.Update hidden={hidden + 1} /> />
  </Stack>
}
component <Zero extends Node /> = { <Num Value={env.z} /> }
component <Boxed extends Node /> = { <Num Value={env.boxed.v} /> }
component <Card extends Node flag:boolean = false /> = {
  state { n:int = 0 }
  <Stack>
    <Label Text={accentOf + \" \" + env.gap + \" \" + n} />
    { if flag { <Label Text={env.loud} /> } else { <Label Text={direction} /> } }
    <Button Text=\"Inc\" onTapped=<State.Update n={n + 1} /> />
  </Stack>
}
component <Amount extends Node /> = { <Any Value={amountOf} /> }
component <Loud extends Node /> = {
  <Stack>{ provide env.mode=dark { <Label Text={env.loud + \" \" + env.colors.accent} /> } }</Stack>
}
component <SelfRef extends Node /> = {
  <Stack>{ provide env.space={env.space + 1} { <Label Text={\"\" + env.gap + \" \" + env.label} /> } }</Stack>
}
component <Field extends Node /> = {
  <Stack>{
    provide env.colors.accent={env.locale} { <Label Text={env.colors.accent + env.colors.tone} /> }
  }</Stack>
}
component <Page /> = {
  state { dark:boolean = false flag:boolean = false labelled:boolean = false }
  <Stack>
    <Card flag={flag} />
    { if dark { <Stack env.mode=dark><Card /></Stack> } else { <Stack env.mode=light><Card /></Stack> } }
    <Stack env.gap={if env.space > 0 { 1 } else { 0 }}><Card flag={true} /></Stack>
    <Loud />
    <Stack env.region={regionOf(env.mode, env.locale)}><Loud /></Stack>
    <SelfRef />
    <Stack env.colors.tone=\"p\"><Field /></Stack>
    <Amount />
    <Zero />
    <Stack env.z={0.0}><Boxed /></Stack>
    <Stack env.z={-0.0}><Boxed /></Stack>
    { if labelled { <Show text=\"x\" /> } else { <Show /> } }
    <Button Text=\"Dark\" onTapped=<State.Update dark={!dark} /> />
    <Button Text=\"Flag\" onTapped=<State.Update flag={!flag} /> />
    <Button Text=\"Label\" onTapped=<State.Update labelled={!labelled} /> />
  </Stack>
}
let root() = { <Page /> }
";

/// SplitMix64: a small seeded generator, so a seed always gives the same steps.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// The values a host may give each root, including ones written differently that compare equal.
fn root_choices() -> Vec<(&'static str, Vec<NxValue>)> {
    let text = |value: &str| NxValue::String(value.to_string());
    vec![
        ("mode", vec![text("light"), text("dark")]),
        ("locale", vec![text("en"), text("ar"), text("fr")]),
        ("region", vec![text("eu"), text("us")]),
        (
            "space",
            vec![
                NxValue::Int(0),
                NxValue::Int(1),
                NxValue::Int(3),
                NxValue::Int(5),
            ],
        ),
        (
            "z",
            vec![
                NxValue::Float(1.0),
                NxValue::Float(0.0),
                NxValue::Float(-0.0),
            ],
        ),
        (
            "amount",
            vec![NxValue::Int(1), NxValue::Float(1.0), NxValue::Int(2)],
        ),
    ]
}

fn env_var(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn options() -> RuntimeOptions {
    RuntimeOptions::default()
}

/// The output with every handler's token left out: a render through the bare lifecycle numbers
/// its handlers afresh.
fn untokened(value: &NxValue) -> NxValue {
    match value {
        NxValue::Array(items) => NxValue::Array(items.iter().map(untokened).collect()),
        NxValue::Record {
            type_name,
            properties,
        } => NxValue::Record {
            type_name: type_name.clone(),
            properties: properties
                .iter()
                .filter(|(name, _)| *name != "token")
                .map(|(name, value)| (name.clone(), untokened(value)))
                .collect(),
        },
        other => other.clone(),
    }
}

/// Whether two outputs are written alike: `NxValue`'s `==` finds `0.0` and `-0.0` equal.
fn same_written(left: &NxValue, right: &NxValue) -> bool {
    format!("{left:?}") == format!("{right:?}")
}

/// The descriptors of authored components in `value`, depth first.
fn authored<'a>(tree: &InstanceTree, value: &'a NxValue, found: &mut Vec<&'a NxValue>) {
    match value {
        NxValue::Array(items) => items.iter().for_each(|item| authored(tree, item, found)),
        NxValue::Record {
            type_name,
            properties,
        } => {
            if type_name
                .as_deref()
                .is_some_and(|name| tree.can_instantiate(name))
            {
                found.push(value);
                return;
            }
            properties
                .values()
                .for_each(|item| authored(tree, item, found));
        }
        _ => {}
    }
}

fn props(descriptor: &NxValue) -> (&str, &BTreeMap<String, NxValue>) {
    match descriptor {
        NxValue::Record {
            type_name: Some(name),
            properties,
        } => (name, properties),
        other => panic!("not a descriptor: {other:?}"),
    }
}

/// A descriptor of `Show` as a host writes it, with or without `text`.
fn host_show(text: bool) -> NxValue {
    let mut properties = BTreeMap::new();
    if text {
        properties.insert("text".to_string(), NxValue::String("host".to_string()));
    }
    NxValue::Record {
        type_name: Some("Show".to_string()),
        properties,
    }
}

/// One run of a seed: the tree, the roots it was last given, and the steps so far.
struct Run {
    tree: InstanceTree,
    /// In `NX_TREE_STRESS_ALL` mode, the nodes found differing so far, which a run goes on past.
    differing: Option<BTreeMap<String, String>>,
    roots: Vec<(String, NxValue)>,
    host_text: bool,
    steps: Vec<String>,
    seed: u64,
}

impl Run {
    fn fail(&self, message: String) -> ! {
        std::panic::panic_any(Failure(format!(
            "instance tree stress, seed {} (rerun with NX_TREE_STRESS_SEED={}): {message}\nsteps:\n  {}",
            self.seed,
            self.seed,
            self.steps.join("\n  ")
        )));
    }

    /// A pass over the tree as a host makes one: the Page at `root`, a host-written `Show` at
    /// `host`, and every authored descriptor under them.
    fn pass(&mut self) {
        let page = self
            .tree
            .program()
            .evaluate_function("root", &[], &options())
            .unwrap_or_else(|error| self.fail(format!("root(): {error}")));
        self.tree.begin();
        let mut queue: Vec<(String, NxValue, Option<String>)> = vec![
            ("root".to_string(), page, None),
            ("host".to_string(), host_show(self.host_text), None),
        ];
        while let Some((key, descriptor, parent)) = queue.pop() {
            let output = match self
                .tree
                .visit(&key, &descriptor, parent.as_deref(), &options())
            {
                Ok(output) => output,
                Err(error) => self.fail(format!("visit {key}: {error}")),
            };
            let mut children = Vec::new();
            authored(&self.tree, &output, &mut children);
            for (index, child) in children.into_iter().enumerate() {
                queue.push((format!("{key}/{index}"), child.clone(), Some(key.clone())));
            }
        }
        self.tree.finish();
    }

    /// Every node's output, by key.
    fn outputs(&self) -> BTreeMap<String, Arc<NxValue>> {
        let mut outputs = BTreeMap::new();
        let mut keys = vec!["root".to_string(), "host".to_string()];
        while let Some(key) = keys.pop() {
            let Some(output) = self.tree.rendered(&key) else {
                continue;
            };
            let mut children = Vec::new();
            authored(&self.tree, output, &mut children);
            for index in 0..children.len() {
                keys.push(format!("{key}/{index}"));
            }
            outputs.insert(key, Arc::clone(output));
        }
        outputs
    }

    /// Checks every node against a fresh initialization through the bare lifecycle, with the
    /// state the tree holds, at the place the overrides above it give.
    fn check_fresh(&mut self) {
        let roots = Environment::root(self.roots.iter().cloned());
        let page = self
            .tree
            .program()
            .evaluate_function("root", &[], &options())
            .unwrap_or_else(|error| self.fail(format!("root(): {error}")));
        let mut queue: Vec<(String, NxValue, Environment, Option<ComponentInstance>)> = vec![
            ("root".to_string(), page, roots.clone(), None),
            ("host".to_string(), host_show(self.host_text), roots, None),
        ];
        let mut differ = Vec::new();
        while let Some((key, descriptor, above, parent)) = queue.pop() {
            let environment = above
                .enter(&descriptor)
                .unwrap_or_else(|error| self.fail(format!("$env of {key}: {error}")));
            let Some(state) = self.tree.state(&key) else {
                self.fail(format!("the tree holds no node at {key}"));
            };
            let (name, properties) = props(&descriptor);
            let fresh = self
                .tree
                .program()
                .initialize_component(
                    name,
                    properties,
                    &ComponentInit {
                        parent: parent.as_ref(),
                        state: Some(state),
                    },
                    &RuntimeOptions {
                        environment: environment.clone(),
                        ..options()
                    },
                )
                .unwrap_or_else(|error| self.fail(format!("fresh {key}: {error}")));
            let held = self.tree.rendered(&key).expect("a node");
            if !same_written(&untokened(held), &untokened(&fresh.rendered)) {
                differ.push((
                    key.clone(),
                    format!(
                        "{key} holds\n    {:?}\n  but a fresh render gives\n    {:?}",
                        untokened(held),
                        untokened(&fresh.rendered)
                    ),
                ));
            }
            let mut children = Vec::new();
            authored(&self.tree, &fresh.rendered, &mut children);
            for (index, child) in children.into_iter().enumerate() {
                queue.push((
                    format!("{key}/{index}"),
                    child.clone(),
                    environment.clone(),
                    Some(fresh.instance.clone()),
                ));
            }
        }
        if differ.is_empty() {
            return;
        }
        if let Some(differing) = &mut self.differing {
            for (key, text) in differ {
                differing
                    .entry(key)
                    .or_insert_with(|| format!("{text}\nafter:\n  {}", self.steps.join("\n  ")));
            }
            return;
        }
        self.fail(
            differ
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }

    /// Checks that the change set lists every node whose output changed, was added or left.
    fn check_changes(&mut self, before: &BTreeMap<String, Arc<NxValue>>) {
        let after = self.outputs();
        let changes = self.tree.take_changes();
        for (key, output) in &after {
            match before.get(key) {
                None => {
                    if !changes.added.contains(key) {
                        self.fail(format!("{key} is new but not listed as added: {changes:?}"));
                    }
                }
                Some(old) if !same_written(old, output) => {
                    if !changes.rendered.contains(key) && !changes.added.contains(key) {
                        self.fail(format!(
                            "{key} changed its output but is not listed: {changes:?}"
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        for key in before.keys() {
            if !after.contains_key(key) && !changes.removed.contains(key) {
                self.fail(format!(
                    "{key} left but is not listed as removed: {changes:?}"
                ));
            }
        }
    }

    /// One random step: new roots, a tap on a button somewhere, or a pass with another host
    /// descriptor. Every step ends with a pass, as a host runs one after each change.
    fn step(&mut self, random: &mut Random) {
        let before = self.outputs();
        match random.below(3) {
            0 => {
                let mut roots = Vec::new();
                for (name, values) in root_choices() {
                    if random.below(3) != 0 {
                        roots.push((name.to_string(), random.pick(&values).clone()));
                    }
                }
                self.steps.push(format!("set_environment {roots:?}"));
                self.roots = roots.clone();
                if let Err(error) = self
                    .tree
                    .set_environment(Environment::root(roots), &options())
                {
                    self.fail(format!("set_environment: {error}"));
                }
            }
            1 => {
                let keys: Vec<String> = before.keys().cloned().collect();
                let key = random.pick(&keys).clone();
                let mut buttons = Vec::new();
                buttons_of(&before[&key], &mut buttons);
                if buttons.is_empty() {
                    self.steps.push(format!("tap nothing at {key}"));
                } else {
                    let (text, token) = random.pick(&buttons).clone();
                    self.steps.push(format!("tap {text} at {key}"));
                    let action = NxValue::from_json_str(r#"{ "$type": "Button.Tapped" }"#)
                        .expect("an action");
                    if let Err(error) = self.tree.dispatch(&key, &token, action, &options()) {
                        self.fail(format!("tap {text} at {key}: {error}"));
                    }
                }
            }
            _ => {
                self.host_text = random.below(2) == 0;
                self.steps
                    .push(format!("pass, host text {}", self.host_text));
            }
        }
        self.pass();
        self.check_fresh();
        self.check_changes(&before);
    }
}

/// The buttons in a node's own output, as text and token: those of the authored components in it
/// belong to their own nodes.
fn buttons_of(value: &NxValue, found: &mut Vec<(String, String)>) {
    match value {
        NxValue::Array(items) => items.iter().for_each(|item| buttons_of(item, found)),
        NxValue::Record {
            type_name,
            properties,
        } => {
            if type_name.as_deref() == Some("Button") {
                let text = match properties.get("Text") {
                    Some(NxValue::String(text)) => text.clone(),
                    _ => String::new(),
                };
                if let Some(NxValue::Record { properties, .. }) = properties.get("onTapped") {
                    if let Some(NxValue::String(token)) = properties.get("token") {
                        found.push((text, token.clone()));
                    }
                }
                return;
            }
            properties.values().for_each(|item| buttons_of(item, found));
        }
        _ => {}
    }
}

/// Runs `iterations` steps of `seed`. In `all` mode a node that differs is noted and the run goes
/// on; the nodes found differing are returned, each with the first difference and its steps.
fn run_seed(seed: u64, iterations: u64, all: bool) -> BTreeMap<String, String> {
    let mut random = Random(seed);
    let mut run = Run {
        tree: InstanceTree::new(program(&format!("{CONTROLS}{PROGRAM}"))),
        differing: all.then(BTreeMap::new),
        roots: Vec::new(),
        host_text: false,
        steps: vec!["pass".to_string()],
        seed,
    };
    let empty = BTreeMap::new();
    run.pass();
    run.check_fresh();
    run.check_changes(&empty);
    for _ in 0..iterations {
        run.step(&mut random);
    }
    run.differing.unwrap_or_default()
}

/// A failure of one seed, with what to print.
struct Failure(String);

#[test]
fn the_tree_keeps_only_output_a_fresh_evaluation_gives() {
    let iterations = env_var("NX_TREE_STRESS_ITERATIONS", 40);
    let seeds: Vec<u64> = match std::env::var("NX_TREE_STRESS_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
    {
        Some(seed) => vec![seed],
        None => (0..env_var("NX_TREE_STRESS_SEEDS", 20)).collect(),
    };
    let all = std::env::var_os("NX_TREE_STRESS_ALL").is_some();
    // By node: how many seeds found it differing, and the first difference found.
    let mut by_node: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut other: Vec<String> = Vec::new();
    for seed in seeds {
        let result = std::panic::catch_unwind(|| run_seed(seed, iterations, all));
        let message = match result {
            Ok(differing) => {
                for (key, text) in differing {
                    let entry = by_node
                        .entry(key)
                        .or_insert_with(|| (0, format!("seed {seed}: {text}")));
                    entry.0 += 1;
                }
                continue;
            }
            Err(payload) => match payload.downcast::<Failure>() {
                Ok(failure) => failure.0,
                Err(payload) => std::panic::resume_unwind(payload),
            },
        };
        if !all {
            panic!("{message}");
        }
        other.push(message);
    }
    if !by_node.is_empty() || !other.is_empty() {
        let counts: BTreeMap<&String, usize> = by_node
            .iter()
            .map(|(key, (count, _))| (key, *count))
            .collect();
        let firsts: Vec<&str> = by_node.values().map(|(_, text)| text.as_str()).collect();
        panic!(
            "seeds differing by node: {counts:?}\n\n{}\n\nother failures:\n\n{}",
            firsts.join("\n\n"),
            other.join("\n\n")
        );
    }
}

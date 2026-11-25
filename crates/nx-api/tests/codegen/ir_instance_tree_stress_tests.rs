//! A differential stress test of the instance tree: seeded random sequences of root changes,
//! taps, host descriptors and removals, after each of which the tree must hold what a model built
//! on the bare lifecycle holds.
//!
//! <para>The tree keeps renders and derived results across operations and decides from its
//! records what to evaluate again. A record that misses a read keeps output a fresh evaluation
//! would not give. The model keeps nothing of that: it holds the state of each node by key, and
//! after each step it initializes every node through the bare lifecycle, with the root values, the
//! place's overrides and the state it holds, and it runs each tap through
//! `dispatch_component_actions` on the instances it initialized, against the node that created the
//! handler, routing what a dispatch emits to the handler the parent bound, as the tree does. A node
//! whose component changes at its key, whose parent is replaced, or that the host removed, starts
//! from its initial state. After each step the tree must hold the model's nodes and no others,
//! with the model's state and output, compared with handler tokens left out, and every node
//! settled. What a token names is checked through the taps, whose state and effects must be the
//! model's. An operation fails in the tree exactly
//! when it fails in the model, and then changes nothing. The host effects of a tap are the
//! model's.</para>
//!
//! <para>The change set is checked both ways: a key is added when the model has a node there now
//! and had none, or had one it replaced; removed the other way round; and rendered exactly when
//! the node stayed and its output changed in any way, in value or in what a token names. A node
//! that rendered output equal to what it held, its tokens aside, must have had a reason to: its
//! record holds a read it cannot check (`renders_at_every_check`), the node above it rendered, its
//! state or the host's descriptor changed, an environment value its component may depend on is
//! written otherwise at its place, or a root its component reads directly was set, cleared or
//! changed. The values a component may depend on are listed by hand, more than it reads rather
//! than fewer, so a render is caught only where none of them changed. That catches a tree that
//! renders where a record holds, or renders a node that reads nothing that changed; it does not
//! catch one that renders a node whose value read changed in a part the node does not use, or a
//! reader of a changed root whose value read comes back equal. After a tap or a root change,
//! before the pass, a node is unsettled exactly when it or a node under it rendered.</para>
//!
//! <para>The program mixes the risky features: element overrides and `provide` blocks, an
//! entry-module bundle with a formula entry, formulas the body provides (one reading its own
//! value), field overrides, override formulas that capture a prop or a state field, a prop default
//! that reads `env`, kept top-level values that read `env`, light and dark sections, values a host
//! can write differently that compare equal, host roots for formula-declared values, refused root
//! values, renders and emit handlers that fail on some values, patches that write the value a
//! field holds, emits bound and unbound, two emits in one handler, a handler in a content child,
//! and nodes that appear, leave and are replaced by another component at their key. It does not
//! cover a handler passed as a prop other than an emit's, `remove` under a node that is left
//! unvisited, a pass that visits only part of the tree, or operation limits.</para>
//!
//! <para>`NX_TREE_STRESS_SEEDS` sets how many seeds run (default 20), `NX_TREE_STRESS_BASE_SEED`
//! the first of them (default 0) and `NX_TREE_STRESS_ITERATIONS` how many steps each runs (default
//! 30); `NX_TREE_STRESS_SEED` runs that one seed alone. A value that is set must be a whole number,
//! the counts at least 1. A failure, a panic in the runtime included, names the seed, the step and
//! the steps that led to it, and gives the command that runs it again. With `NX_TREE_STRESS_ALL`
//! set, a seed goes on past a node whose output or state differs, and the end sums up, by node, how
//! many seeds found it differing and the first difference found. Outputs compare as a host sees
//! them written, so `0.0` and `-0.0` differ.</para>

use crate::ir_runtime_tests::program;
use nx_ir_runtime::{
    ComponentInit, ComponentInstance, Environment, InstanceTree, Program, RuntimeOptions,
    TreeChanges,
};
use nx_value::NxValue;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use std::sync::Arc;

const CONTROLS: &str = "
abstract external component <Node />
external component <Label extends Node Text?:string />
external component <Num extends Node Value?:float64 />
external component <Any extends Node Value?:object />
external component <Button extends Node Text?:string emits { Tapped { } } />
external component <Stack extends Node content Children?:Node+ />
";

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

component <Dir extends Node /> = {
  <Label Text={direction + \" \" + env.label + \" \" + (12 / (env.space - 5))} />
}
component <Show extends Node text:string = { env.label } /> = {
  state { taps:int = 0 hidden:int = 0 level:int = 0 }
  <Stack>
    <Label Text={text + \" \" + taps} />
    <Label Text={if level > 5 { \"big\" } else { \"small\" }} />
    <Button Text=\"Level\" onTapped=<State.Update level={level + 1} /> />
    <Button Text=\"Tap\" onTapped=<State.Update taps={taps + 1} /> />
    <Button Text=\"Same\" onTapped=<State.Update taps={taps} /> />
    <Button Text=\"Hide\" onTapped=<State.Update hidden={hidden + 1} /> />
    <Stack env.space={if text == \"x\" { 4 } else { env.space }}><Dir /></Stack>
  </Stack>
}
component <Zero extends Node /> = { <Num Value={env.z} /> }
component <Boxed extends Node /> = { <Num Value={env.boxed.v} /> }
component <Card extends Node flag:boolean = false /> = {
  state { n:int = 0 }
  <Stack>
    <Label Text={accentOf + \" \" + env.gap + \" \" + n + \" \" + (6 / (2 - n))} />
    { if flag { <Label Text={env.loud} /> } else { <Label Text={direction} /> } }
    <Button Text=\"Inc\" onTapped=<State.Update n={n + 1} /> />
    <Button Text=\"Keep\" onTapped=<State.Update n={n} /> />
    <Stack env.locale={if n > 0 { \"ar\" } else { env.locale }}><Dir /></Stack>
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
component <Picker extends Node emits { Picked { value:int } } /> = {
  state { picks:int = 0 }
  <Stack>
    <Label Text={\"picks \" + picks} />
    <Button Text=\"Pick\" onTapped={<State.Update picks={picks + 1} /> <Picker.Picked value={picks} />} />
    <Button Text=\"Twice\" onTapped={<Picker.Picked value={2} /> <Picker.Picked value={3} />} />
  </Stack>
}
component <Frame extends Node content Children:Node+ /> = {
  state { opened:int = 0 }
  <Stack>
    <Label Text={\"opened \" + opened} />
    <Button Text=\"Open\" onTapped=<State.Update opened={opened + 1} /> />
    {Children}
  </Stack>
}
component <Page /> = {
  state {
    dark:boolean = false flag:boolean = false labelled:boolean = false rows:int = 1 picked:int = 0
    deep:int = 0
  }
  <Stack>
    <Card flag={flag} />
    { if dark { <Stack env.mode=dark><Card /></Stack> } else { <Stack env.mode=light><Card /></Stack> } }
    <Stack env.gap={if env.space > 0 { 1 } else { 0 }}><Card flag={true} /></Stack>
    { if rows > 0 { <Loud /> } else { <Label Text=\"none\" /> } }
    <Stack env.region={regionOf(env.mode, env.locale)}><Loud /></Stack>
    <SelfRef />
    <Stack env.colors.tone=\"p\"><Field /></Stack>
    { if flag { <Amount /> } else { <Zero /> } }
    { if rows > 1 { <Card flag={dark} /> } else { <Label Text=\"few\" /> } }
    <Stack env.z={0.0}><Boxed /></Stack>
    <Stack env.z={-0.0}><Boxed /></Stack>
    { if labelled { <Show text=\"x\" /> } else { <Show /> } }
    <Label Text={\"picked \" + picked} />
    <Picker onPicked=<State.Update picked={picked + 12 / (1 - action.value)} /> />
    <Picker />
    <Frame>
      <Label Text={\"deep \" + deep} />
      <Button Text=\"Deep\" onTapped=<State.Update deep={deep + 1} /> />
    </Frame>
    <Button Text=\"Dark\" onTapped=<State.Update dark={!dark} /> />
    <Button Text=\"Flag\" onTapped=<State.Update flag={!flag} /> />
    <Button Text=\"Label\" onTapped=<State.Update labelled={!labelled} /> />
    <Button Text=\"Rows\" onTapped=<State.Update rows={if rows > 1 { 0 } else { rows + 1 }} /> />
  </Stack>
}
let root() = { <Page /> }
type PlaceValues = {
  mode:Mode locale:string region:string space:int gap:int z:float64 amount:object boxed:Box colors:Colors
  loud:string label:string
}
let placeValues(): PlaceValues = {
  <PlaceValues mode={env.mode} locale={env.locale} region={env.region} space={env.space} gap={env.gap}
    z={env.z} amount={env.amount} boxed={env.boxed} colors={env.colors} loud={env.loud} label={env.label} />
}
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

fn text(value: &str) -> NxValue {
    NxValue::String(value.to_string())
}

fn record(name: &str, fields: &[(&str, NxValue)]) -> NxValue {
    NxValue::Record {
        type_name: Some(name.to_string()),
        properties: fields
            .iter()
            .map(|(field, value)| (field.to_string(), value.clone()))
            .collect(),
    }
}

/// The values a host may give each root: ones written differently that compare equal, values for
/// the roots whose defaults are formulas, and ones the tree refuses (`space` 5 makes `Dir` divide
/// by zero, and a string is not an `int` or a `Mode`).
fn root_choices() -> Vec<(&'static str, Vec<NxValue>)> {
    vec![
        ("mode", vec![text("light"), text("dark"), text("purple")]),
        ("locale", vec![text("en"), text("ar"), text("fr")]),
        ("region", vec![text("eu"), text("us")]),
        (
            "space",
            vec![
                NxValue::Int(0),
                NxValue::Int(1),
                NxValue::Int(3),
                NxValue::Int(5),
                text("wide"),
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
        ("gap", vec![NxValue::Int(7)]),
        ("label", vec![text("host label")]),
        ("loud", vec![text("host loud")]),
        ("boxed", vec![record("Box", &[("v", NxValue::Float(2.0))])]),
        (
            "colors",
            vec![record(
                "Colors",
                &[("accent", text("red")), ("tone", text("q"))],
            )],
        ),
    ]
}

/// The environment values a component's render may read directly, as a root read whose revision
/// a change of the root makes stale: in its body, its prop defaults and the sources of its own
/// overrides, and in the formula defaults evaluated under its own overrides, which are not kept.
/// What it reads only through a kept top-level value (`direction`, `accentOf`, `amountOf`) or a
/// formula default it reads where nothing of its own overrides is not here: those reads stop at
/// the cutoff. A read of `gap` reads `space` too, through the entry bundle's formula. The lists
/// may say more than a render reads, never less.
fn direct_reads(component: &str) -> &'static [&'static str] {
    match component {
        "Dir" => &["label", "space"],
        "Show" => &["label", "space"],
        "Zero" => &["z"],
        "Boxed" => &["boxed"],
        "Card" => &["gap", "space", "loud"],
        "Loud" => &["mode", "locale", "region", "loud", "colors"],
        "SelfRef" => &["space", "gap", "label", "locale"],
        "Field" => &["locale", "colors", "mode"],
        _ => &[],
    }
}

/// The names a component's render may read, environment values and the kept top-level values
/// alike, wherever it reads them: in its body, its prop defaults and its own overrides. A list
/// may say more than a render reads, never less.
fn read_names(component: &str) -> &'static [&'static str] {
    match component {
        "Dir" => &["direction", "label", "space"],
        "Show" => &["label", "space"],
        "Zero" => &["z"],
        "Boxed" => &["boxed"],
        "Card" => &["accentOf", "direction", "gap", "loud", "locale"],
        "Amount" => &["amountOf"],
        "Loud" => &["mode", "loud", "colors"],
        "SelfRef" => &["space", "gap", "label"],
        "Field" => &["locale", "colors"],
        "Page" => &["mode", "locale", "space"],
        _ => &[],
    }
}

/// The environment values a component's render may depend on: those it reads, and those the
/// formula defaults, the entry bundle's formula and the kept top-level values it reads read in
/// turn.
fn place_reads(component: &str) -> BTreeSet<&'static str> {
    let through = |name: &str| -> &'static [&'static str] {
        match name {
            "label" => &["space", "locale"],
            "loud" => &["mode", "locale", "region"],
            "colors" => &["mode"],
            "boxed" => &["z"],
            "gap" => &["space"],
            "direction" => &["locale"],
            "accentOf" => &["colors", "direction"],
            "amountOf" => &["amount"],
            _ => &[],
        }
    };
    let mut found = BTreeSet::new();
    let mut pending: Vec<&'static str> = read_names(component).to_vec();
    while let Some(name) = pending.pop() {
        if found.insert(name) {
            pending.extend(through(name));
        }
    }
    found
}

/// The descriptor a host passes at `host`, if any.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Host {
    None,
    Show,
    ShowText,
    Zero,
}

impl Host {
    const ALL: [Host; 4] = [Host::None, Host::Show, Host::ShowText, Host::Zero];

    fn descriptor(self) -> Option<NxValue> {
        match self {
            Host::None => None,
            Host::Show => Some(record("Show", &[])),
            Host::ShowText => Some(record("Show", &[("text", text("host"))])),
            Host::Zero => Some(record("Zero", &[])),
        }
    }
}

/// The value of the variable `name`, or `default` when it is not set. A value that is set and is
/// not a whole number, or is below `least`, stops the test, so a mistyped rerun cannot pass.
fn env_var(name: &str, default: u64, least: u64) -> u64 {
    let Some(value) = std::env::var_os(name) else {
        return default;
    };
    let value = value.to_string_lossy();
    match value.parse::<u64>() {
        Ok(number) if number >= least => number,
        _ => panic!(
            "{name} must be a whole number from {least} to {}, not {value:?}",
            u64::MAX
        ),
    }
}

fn options() -> RuntimeOptions {
    RuntimeOptions::default()
}

/// The output with every handler's token left out: the model's instances number their handlers
/// apart from the tree's.
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

/// Every handler token in `value`, at any depth, in a fixed order.
fn tokens(value: &NxValue, found: &mut Vec<String>) {
    match value {
        NxValue::Array(items) => items.iter().for_each(|item| tokens(item, found)),
        NxValue::Record { properties, .. } => {
            for (name, item) in properties {
                match item {
                    NxValue::String(token) if name == "token" => found.push(token.clone()),
                    _ => tokens(item, found),
                }
            }
        }
        _ => {}
    }
}

fn tokens_of(value: &NxValue) -> Vec<String> {
    let mut found = Vec::new();
    tokens(value, &mut found);
    found
}

/// Whether two values are written alike: `NxValue`'s `==` finds `0.0` and `-0.0` equal.
fn same_written<T: std::fmt::Debug>(left: &T, right: &T) -> bool {
    format!("{left:?}") == format!("{right:?}")
}

/// The components `PROGRAM` declares with a body, which a tree can instantiate.
const AUTHORED: [&str; 12] = [
    "Dir", "Show", "Zero", "Boxed", "Card", "Amount", "Loud", "SelfRef", "Field", "Picker",
    "Frame", "Page",
];

/// The descriptors of authored components in `value`, depth first.
fn authored<'a>(value: &'a NxValue, found: &mut Vec<&'a NxValue>) {
    match value {
        NxValue::Array(items) => items.iter().for_each(|item| authored(item, found)),
        NxValue::Record {
            type_name,
            properties,
        } => {
            if type_name
                .as_deref()
                .is_some_and(|name| AUTHORED.contains(&name))
            {
                found.push(value);
                return;
            }
            properties.values().for_each(|item| authored(item, found));
        }
        _ => {}
    }
}

fn children_of(value: &NxValue) -> Vec<&NxValue> {
    let mut found = Vec::new();
    authored(value, &mut found);
    found
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

/// The buttons in a node's output, as text and token, in a fixed order. Those in the content of a
/// descriptor it renders are its own; those an authored child renders are the child's.
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

fn buttons(value: &NxValue) -> Vec<(String, String)> {
    let mut found = Vec::new();
    buttons_of(value, &mut found);
    found
}

/// Whether `key` is `other` or a key under it.
fn at_or_under(key: &str, other: &str) -> bool {
    key == other
        || key
            .strip_prefix(other)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn parent_key(key: &str) -> Option<&str> {
    key.rsplit_once('/').map(|(parent, _)| parent)
}

fn field_of(value: &NxValue, name: &str) -> Option<NxValue> {
    match value {
        NxValue::Record { properties, .. } => properties.get(name).cloned(),
        _ => None,
    }
}

fn tapped() -> NxValue {
    record("Button.Tapped", &[])
}

fn invocation(token: &str, action: NxValue) -> NxValue {
    record(
        "ActionHandlerInvocation",
        &[("token", text(token)), ("action", action)],
    )
}

/// A node of the model: what the bare lifecycle initialized at a key.
#[derive(Clone)]
struct ModelNode {
    component: String,
    state: BTreeMap<String, NxValue>,
    instance: ComponentInstance,
    rendered: NxValue,
    /// The node's place: the root values and the overrides above it and on its descriptor.
    environment: Environment,
}

/// What a walk of the model found: its nodes, and the keys at which it replaced a node it held.
struct Walk {
    nodes: BTreeMap<String, ModelNode>,
    replaced: BTreeSet<String>,
}

/// The model: the nodes the bare lifecycle gives, by key, and the keys the host removed since.
struct Model {
    nodes: BTreeMap<String, ModelNode>,
    removed: BTreeSet<String>,
}

impl Model {
    /// Every node, initialized afresh through the bare lifecycle from the root and host
    /// descriptors, with the state the model holds for a node of that component at that key.
    fn walk(&self, program: &Program, roots: &Environment, host: Host) -> Result<Walk, String> {
        let page = program
            .evaluate_function("root", &[], &options())
            .map_err(|error| format!("root(): {error}"))?;
        let mut queue: Vec<(String, NxValue, Environment, bool)> =
            vec![("root".to_string(), page, roots.clone(), false)];
        if let Some(descriptor) = host.descriptor() {
            queue.push(("host".to_string(), descriptor, roots.clone(), false));
        }
        let mut nodes: BTreeMap<String, ModelNode> = BTreeMap::new();
        let mut replaced = BTreeSet::new();
        while let Some((key, descriptor, above, parent_replaced)) = queue.pop() {
            let environment = above
                .enter(&descriptor)
                .map_err(|error| format!("$env of {key}: {error}"))?;
            let (name, properties) = props(&descriptor);
            let held = self.nodes.get(&key);
            let fresh = held.is_some()
                && (parent_replaced
                    || self.removed.contains(&key)
                    || held.is_some_and(|node| node.component != name));
            if fresh {
                replaced.insert(key.clone());
            }
            let state = held.filter(|_| !fresh).map(|node| &node.state);
            let parent = parent_key(&key)
                .and_then(|parent| nodes.get(parent))
                .map(|node| node.instance.clone());
            let init = program
                .initialize_component(
                    name,
                    properties,
                    &ComponentInit {
                        parent: parent.as_ref(),
                        state,
                    },
                    &RuntimeOptions {
                        environment: environment.clone(),
                        ..options()
                    },
                )
                .map_err(|error| format!("initialize {key}: {error}"))?;
            for (index, child) in children_of(&init.rendered).into_iter().enumerate() {
                queue.push((
                    format!("{key}/{index}"),
                    child.clone(),
                    environment.clone(),
                    parent_replaced || fresh,
                ));
            }
            nodes.insert(
                key,
                ModelNode {
                    component: name.to_string(),
                    state: init.state,
                    instance: init.instance,
                    rendered: init.rendered,
                    environment,
                },
            );
        }
        Ok(Walk { nodes, replaced })
    }

    /// The node, `from` or above it, farthest from it whose output holds a handler that
    /// `matches`, and the token it holds it under.
    fn creator(
        &self,
        from: Option<&str>,
        matches: impl Fn(&ComponentInstance, &str) -> bool,
    ) -> Option<(String, String)> {
        let mut found = None;
        let mut next = from;
        while let Some(key) = next {
            let node = self.nodes.get(key)?;
            if let Some(token) = tokens_of(&node.rendered)
                .into_iter()
                .find(|token| matches(&node.instance, token))
            {
                found = Some((key.to_string(), token));
            }
            next = parent_key(key);
        }
        found
    }

    /// A tap on the button at `index` in the output of the node at `key`, run through the bare
    /// lifecycle as the tree runs it: against the node that created the handler, with each emit
    /// routed to the handler the parent bound, the node farthest from the root first. Returns the
    /// host effects, or the error, which leaves the model as it was.
    fn tap(
        &mut self,
        program: &Program,
        key: &str,
        index: usize,
    ) -> Result<Vec<(String, String, NxValue)>, String> {
        let node = &self.nodes[key];
        let (_, token) = buttons(&node.rendered)[index].clone();
        let instance = node.instance.clone();
        let Some(first) = self.creator(Some(key), |other, other_token| {
            instance.same_handler(&token, other, other_token)
        }) else {
            return Err(format!("no node created the handler under {token}"));
        };
        let mut pending = vec![(first.0, first.1, tapped())];
        let mut effects = Vec::new();
        let mut next = self.nodes.clone();
        while let Some(deepest) = pending
            .iter()
            .map(|(key, _, _)| key.clone())
            .max_by_key(|key| key.matches('/').count())
        {
            let (batch, rest): (Vec<_>, Vec<_>) =
                pending.into_iter().partition(|(key, _, _)| *key == deepest);
            pending = rest;
            let batch: Vec<NxValue> = batch
                .into_iter()
                .map(|(_, token, action)| invocation(&token, action))
                .collect();
            let node = &next[&deepest];
            let result = program
                .dispatch_component_actions(&node.instance, &batch, &options())
                .map_err(|error| format!("dispatch at {deepest}: {error}"))?;
            for action in result.effects {
                let emit = match &action {
                    NxValue::Record {
                        type_name: Some(name),
                        ..
                    } => name
                        .strip_prefix(&format!("{}.", node.component))
                        .map(|emit| format!("on{emit}")),
                    _ => None,
                };
                let bound = emit.and_then(|property| {
                    self.creator(parent_key(&deepest), |other, other_token| {
                        node.instance
                            .bound_handler_is(&property, other, other_token)
                    })
                });
                match bound {
                    Some((key, token)) => pending.push((key, token, action)),
                    None => effects.push((deepest.clone(), node.component.clone(), action)),
                }
            }
            let node = next.get_mut(&deepest).expect("a node");
            node.state = result.state;
            node.instance = result.instance;
        }
        self.nodes = next;
        Ok(effects)
    }
}

/// What a step saw of the tree before it: each node's output and instance, and whether its record
/// makes it render at every check, by key.
type Snapshot = BTreeMap<String, (Arc<NxValue>, ComponentInstance, bool)>;

/// One run of a seed: the tree, the model, the roots and host descriptor they were last given,
/// and the steps so far.
struct Run {
    tree: InstanceTree,
    model: Model,
    /// In `NX_TREE_STRESS_ALL` mode, the nodes found differing so far, which a run goes on past.
    differing: Option<BTreeMap<String, String>>,
    roots: Vec<(String, NxValue)>,
    host: Host,
    /// The host descriptor, the roots and each node's state before the step.
    host_before: Host,
    states_before: BTreeMap<String, BTreeMap<String, NxValue>>,
    roots_before: Vec<(String, NxValue)>,
    steps: Rc<RefCell<Vec<String>>>,
}

impl Run {
    fn fail(&self, message: String) -> ! {
        std::panic::panic_any(Failure(message));
    }

    fn note(&self, step: String) {
        self.steps.borrow_mut().push(step);
    }

    fn roots(&self) -> Environment {
        Environment::root(self.roots.iter().cloned())
    }

    /// A pass over the tree as a host makes one: the Page at `root`, the host's descriptor at
    /// `host`, and every authored descriptor under them, all or nothing.
    fn pass(&mut self) {
        let host = self.host;
        let result = self.tree.atomically(|tree| {
            let page = tree.program().evaluate_function("root", &[], &options())?;
            tree.begin();
            let mut queue: Vec<(String, NxValue, Option<String>)> =
                vec![("root".to_string(), page, None)];
            if let Some(descriptor) = host.descriptor() {
                queue.push(("host".to_string(), descriptor, None));
            }
            while let Some((key, descriptor, parent)) = queue.pop() {
                let output = tree.visit(&key, &descriptor, parent.as_deref(), &options())?;
                for (index, child) in children_of(&output).into_iter().enumerate() {
                    queue.push((format!("{key}/{index}"), child.clone(), Some(key.clone())));
                }
            }
            tree.finish();
            Ok::<(), nx_ir_runtime::NxIrRuntimeError>(())
        });
        if let Err(error) = result {
            self.fail(format!("the pass failed: {error}"));
        }
    }

    /// The tree's output and instance at every key the model holds, and whether the node renders at
    /// every check.
    fn snapshot(&self) -> Snapshot {
        self.model
            .nodes
            .keys()
            .filter_map(|key| {
                let output = self.tree.rendered(key)?;
                let instance = self.tree.instance(key)?;
                let every_check = self.tree.renders_at_every_check(key);
                Some((
                    key.clone(),
                    (Arc::clone(output), instance.clone(), every_check),
                ))
            })
            .collect()
    }

    /// Checks that nothing in the tree changed: the same allocation of output at every node, the
    /// same state, and nothing in the change set.
    fn check_unchanged(&mut self, before: &Snapshot, what: &str) {
        for (key, (output, _, _)) in before {
            let same = self
                .tree
                .rendered(key)
                .is_some_and(|now| Arc::ptr_eq(now, output));
            let state = self.tree.state(key);
            if !same || state.is_none_or(|state| !same_written(state, &self.model.nodes[key].state))
            {
                self.fail(format!("{key} changed after {what}"));
            }
        }
        let changes = self.tree.take_changes();
        if !changes.is_empty() {
            self.fail(format!("{what} left a change set: {changes:?}"));
        }
    }

    /// Checks, after a tap or a root change and before the pass, that a node is unsettled exactly
    /// when it or a node under it rendered: holds another allocation of output.
    fn check_settled(&self, before: &Snapshot) {
        let rendered: Vec<&String> = before
            .iter()
            .filter(|(key, (output, _, _))| {
                !self
                    .tree
                    .rendered(key)
                    .is_some_and(|now| Arc::ptr_eq(now, output))
            })
            .map(|(key, _)| key)
            .collect();
        for key in before.keys() {
            let expected = !rendered.iter().any(|other| at_or_under(other, key));
            if self.tree.is_settled(key) != expected {
                self.fail(format!(
                    "{key} is {}settled, but the nodes that rendered are {rendered:?}",
                    if expected { "not " } else { "" }
                ));
            }
        }
    }

    /// Walks the model after a step and checks the tree against it: the same nodes, each with
    /// the model's state and output and settled, and the change set the difference makes.
    fn check(&mut self, before: &Snapshot) {
        let walk = self
            .model
            .walk(self.tree.program(), &self.roots(), self.host)
            .unwrap_or_else(|error| self.fail(format!("the model's walk failed: {error}")));
        let mut differ = Vec::new();
        for (key, node) in &walk.nodes {
            let (Some(held), Some(state), Some(instance)) = (
                self.tree.rendered(key),
                self.tree.state(key),
                self.tree.instance(key),
            ) else {
                self.fail(format!("the tree holds no node at {key}"));
            };
            if instance.component() != node.component {
                self.fail(format!(
                    "{key} is a {} in the tree but a {} in the model",
                    instance.component(),
                    node.component
                ));
            }
            if !same_written(&untokened(held), &untokened(&node.rendered)) {
                differ.push((
                    key.clone(),
                    format!(
                        "{key} holds\n    {:?}\n  but the model renders\n    {:?}",
                        untokened(held),
                        untokened(&node.rendered)
                    ),
                ));
            } else if !same_written(state, &node.state) {
                differ.push((
                    key.clone(),
                    format!(
                        "{key} holds the state {state:?} but the model {:?}",
                        node.state
                    ),
                ));
            }
            if !self.tree.is_settled(key) {
                self.fail(format!("{key} is not settled after the pass"));
            }
        }
        for key in self.model.nodes.keys() {
            if !walk.nodes.contains_key(key) && self.tree.rendered(key).is_some() {
                self.fail(format!(
                    "the tree still holds {key}, which the model dropped"
                ));
            }
        }
        self.check_changes(before, &walk);
        self.model.nodes = walk.nodes;
        self.model.removed.clear();
        if differ.is_empty() {
            return;
        }
        if let Some(differing) = &mut self.differing {
            let steps = self.steps.borrow().join("\n  ");
            for (key, text) in differ {
                differing
                    .entry(key)
                    .or_insert_with(|| format!("{text}\nafter:\n  {steps}"));
            }
            // Go on from what the tree holds, so one difference is not found again at every
            // step.
            for (key, node) in &mut self.model.nodes {
                if let Some(state) = self.tree.state(key) {
                    node.state = state.clone();
                }
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

    /// Whether a token in the output of the node at `key` names a handler that a node above it
    /// created and that node rendered since `before`: the token now names a handler of that
    /// node's new instance.
    fn rebound(&self, key: &str, before: &Snapshot) -> bool {
        let (Some(output), Some(instance)) = (self.tree.rendered(key), self.tree.instance(key))
        else {
            return false;
        };
        tokens_of(output).iter().any(|token| {
            let mut above = parent_key(key);
            while let Some(other) = above {
                let (Some(theirs), Some(their_instance)) =
                    (self.tree.rendered(other), self.tree.instance(other))
                else {
                    return false;
                };
                let rendered = before
                    .get(other)
                    .is_none_or(|(held, _, _)| !Arc::ptr_eq(held, theirs));
                if rendered
                    && tokens_of(theirs)
                        .iter()
                        .any(|their| instance.same_handler(token, their_instance, their))
                {
                    return true;
                }
                above = parent_key(other);
            }
            false
        })
    }

    /// Whether the node at `key` had a reason to render in the step: the node above it rendered
    /// (so its descriptor may name other handlers), the host gave another descriptor, its state
    /// changed since before the step, an environment value it may depend on (`place_reads`) is
    /// now written otherwise at its place, or the host set,
    /// cleared or changed the root of a value its component reads directly, which is a change to
    /// that read even where the value read is the same. A render with none of these is one no
    /// record needs: a read through a kept result that comes back equal stops at the cutoff.
    fn had_reason(&self, key: &str, before: &Snapshot, now: &ModelNode) -> bool {
        let parent_rendered = match parent_key(key) {
            Some(parent) => before.get(parent).is_none_or(|(held, _, _)| {
                !self
                    .tree
                    .rendered(parent)
                    .is_some_and(|current| Arc::ptr_eq(held, current))
            }),
            None => key == "host" && self.host != self.host_before,
        };
        let then = &self.model.nodes[key];
        let roots = |roots: &[(String, NxValue)]| -> BTreeMap<String, String> {
            roots
                .iter()
                .map(|(name, value)| (name.clone(), format!("{value:?}")))
                .collect()
        };
        let (after, earlier) = (roots(&self.roots), roots(&self.roots_before));
        let root_changed = direct_reads(&now.component)
            .iter()
            .any(|name| after.get(*name) != earlier.get(*name));
        let state_before = self.states_before.get(key).unwrap_or(&then.state);
        let place_changed = || {
            let (earlier, later) = (
                self.place_values(&then.environment),
                self.place_values(&now.environment),
            );
            place_reads(&now.component)
                .into_iter()
                .any(|name| !same_written(&field_of(&earlier, name), &field_of(&later, name)))
        };
        parent_rendered
            || root_changed
            || !same_written(state_before, &now.state)
            || place_changed()
    }

    /// Every environment value at a place, as the bare lifecycle evaluates them there.
    fn place_values(&self, environment: &Environment) -> NxValue {
        self.tree
            .program()
            .evaluate_function(
                "placeValues",
                &[],
                &RuntimeOptions {
                    environment: environment.clone(),
                    ..options()
                },
            )
            .unwrap_or_else(|error| self.fail(format!("placeValues(): {error}")))
    }

    /// Checks the change set against the model's: added where the model has a node it had not,
    /// or replaced, and removed the other way round. A node that stayed is rendered when its
    /// output changed in any way: in value, in a token's handler, or by a rebinding to a node
    /// above that rendered, and is not otherwise. A node that rendered output equal to what it held
    /// may be listed or not; it must have had a reason to render, by `had_reason`, unless its
    /// record holds a read it cannot check, which renders at every check.
    fn check_changes(&mut self, before: &Snapshot, walk: &Walk) {
        let mut expected = TreeChanges::default();
        let mut may = Vec::new();
        for key in walk.nodes.keys() {
            if !self.model.nodes.contains_key(key) || walk.replaced.contains(key) {
                expected.added.push(key.clone());
                continue;
            }
            let (output, instance, every_check) = &before[key];
            let now = self.tree.rendered(key).expect("a node");
            let current = self.tree.instance(key).expect("a node");
            let rendered = !Arc::ptr_eq(output, now);
            if rendered
                && same_written(&untokened(output), &untokened(now))
                && !every_check
                && !self.had_reason(key, before, &walk.nodes[key])
            {
                self.fail(format!(
                    "{key} rendered again with nothing it could read changed: no value at its \
                     place, no root, no state and no descriptor"
                ));
            }
            let changed = !same_written(&**output, &**now)
                || tokens_of(now)
                    .iter()
                    .any(|token| !instance.same_handler(token, current, token))
                || self.rebound(key, before);
            if changed {
                expected.rendered.push(key.clone());
            } else if rendered {
                may.push(key.clone());
            }
        }
        for key in self.model.nodes.keys() {
            if !walk.nodes.contains_key(key) || walk.replaced.contains(key) {
                expected.removed.push(key.clone());
            }
        }
        let mut changes = self.tree.take_changes();
        let listed = changes.rendered.clone();
        changes.rendered.retain(|key| !may.contains(key));
        if changes != expected {
            self.fail(format!(
                "the change set is\n    {:?}\n  but the model expects\n    {expected:?}\n  \
                 and allows {may:?} as rendered",
                TreeChanges {
                    rendered: listed,
                    ..changes
                }
            ));
        }
    }

    /// One random step: new roots, a tap on a button somewhere, another host descriptor, or a
    /// host's removal of a node. Every step ends with a pass, as a host runs one after each
    /// change.
    fn step(&mut self, random: &mut Random) {
        let before = self.snapshot();
        self.host_before = self.host;
        self.roots_before = self.roots.clone();
        self.states_before = self
            .model
            .nodes
            .iter()
            .map(|(key, node)| (key.clone(), node.state.clone()))
            .collect();
        match random.below(7) {
            0 | 1 => {
                let mut roots = Vec::new();
                for (name, values) in root_choices() {
                    if random.below(3) == 0 {
                        roots.push((name.to_string(), random.pick(&values).clone()));
                    }
                }
                self.note(format!("set_environment {roots:?}"));
                let environment = Environment::root(roots.clone());
                let expected = self
                    .model
                    .walk(self.tree.program(), &environment, self.host)
                    .map(|_| ());
                let actual = self.tree.set_environment(environment, &options());
                match (expected, actual) {
                    (Ok(()), Ok(())) => {
                        self.roots = roots;
                        self.check_settled(&before);
                    }
                    (Err(_), Err(_)) => self.check_unchanged(&before, "a refused root change"),
                    (Ok(()), Err(error)) => {
                        self.fail(format!("set_environment failed: {error}"));
                    }
                    (Err(error), Ok(())) => self.fail(format!(
                        "set_environment succeeded, but the model fails: {error}"
                    )),
                }
            }
            2..=4 => {
                let keys: Vec<String> = before.keys().cloned().collect();
                let key = random.pick(&keys).clone();
                let found = buttons(&before[&key].0);
                if found.is_empty() {
                    self.note(format!("tap nothing at {key}"));
                } else {
                    let index = random.below(found.len());
                    let (label, token) = found[index].clone();
                    self.note(format!("tap {label} at {key}"));
                    let program = self.tree.program().clone();
                    let expected = self.model.tap(&program, &key, index);
                    let actual = self.tree.dispatch(&key, &token, tapped(), &options());
                    match (expected, actual) {
                        (Ok(expected), Ok(actual)) => {
                            let actual: Vec<(String, String, NxValue)> = actual
                                .into_iter()
                                .map(|effect| (effect.key, effect.component, effect.action))
                                .collect();
                            if !same_written(&actual, &expected) {
                                self.fail(format!(
                                    "the host effects are {actual:?}, but the model's {expected:?}"
                                ));
                            }
                            self.check_settled(&before);
                        }
                        (Err(_), Err(_)) => self.check_unchanged(&before, "a failed tap"),
                        (Ok(_), Err(error)) => self.fail(format!("the tap failed: {error}")),
                        (Err(error), Ok(_)) => {
                            self.fail(format!("the tap succeeded, but the model fails: {error}"))
                        }
                    }
                }
            }
            5 => {
                self.host = *random.pick(&Host::ALL);
                self.note(format!("pass, host {:?}", self.host));
            }
            _ => {
                let keys: Vec<String> = before.keys().cloned().collect();
                let key = random.pick(&keys).clone();
                self.note(format!("remove {key}"));
                if !self.tree.remove(&key) {
                    self.fail(format!("remove {key} found no node"));
                }
                self.model.removed.insert(key);
            }
        }
        self.pass();
        self.check(&before);
    }
}

/// Runs `iterations` steps of `seed`, noting each in `steps`. In `all` mode a node that differs
/// is noted and the run goes on; the nodes found differing are returned, each with the first
/// difference and its steps.
fn run_seed(
    seed: u64,
    iterations: u64,
    all: bool,
    steps: &Rc<RefCell<Vec<String>>>,
) -> BTreeMap<String, String> {
    let mut random = Random(seed);
    let mut run = Run {
        tree: InstanceTree::new(program(&format!("{CONTROLS}{PROGRAM}"))),
        model: Model {
            nodes: BTreeMap::new(),
            removed: BTreeSet::new(),
        },
        differing: all.then(BTreeMap::new),
        roots: Vec::new(),
        host: Host::Show,
        host_before: Host::Show,
        states_before: BTreeMap::new(),
        roots_before: Vec::new(),
        steps: Rc::clone(steps),
    };
    run.note("pass, host Show".to_string());
    run.pass();
    run.check(&Snapshot::new());
    for _ in 0..iterations {
        run.step(&mut random);
    }
    run.differing.unwrap_or_default()
}

/// A failure the test found, which the seed's report adds the seed and the steps to.
struct Failure(String);

/// The text of a panic: the test's own failure, or a panic's message.
fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<Failure>() {
        Ok(failure) => failure.0,
        Err(payload) => match payload.downcast::<String>() {
            Ok(message) => format!("panicked: {message}"),
            Err(payload) => match payload.downcast::<&str>() {
                Ok(message) => format!("panicked: {message}"),
                Err(_) => "panicked".to_string(),
            },
        },
    }
}

#[test]
fn the_tree_holds_what_the_bare_lifecycle_gives() {
    let iterations = env_var("NX_TREE_STRESS_ITERATIONS", 30, 1);
    let seeds: Vec<u64> = match std::env::var_os("NX_TREE_STRESS_SEED") {
        Some(_) => vec![env_var("NX_TREE_STRESS_SEED", 0, 0)],
        None => {
            let base = env_var("NX_TREE_STRESS_BASE_SEED", 0, 0);
            let count = env_var("NX_TREE_STRESS_SEEDS", 20, 1);
            let Some(last) = base.checked_add(count - 1) else {
                panic!(
                    "NX_TREE_STRESS_BASE_SEED {base} plus {count} seeds is past {}",
                    u64::MAX
                );
            };
            (base..=last).collect()
        }
    };
    println!(
        "instance tree stress: seeds {} to {}, {iterations} steps each",
        seeds[0],
        seeds[seeds.len() - 1]
    );
    let all = std::env::var_os("NX_TREE_STRESS_ALL").is_some();
    // By node: how many seeds found it differing, and the first difference found.
    let mut by_node: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut other: Vec<String> = Vec::new();
    for seed in seeds {
        let steps = Rc::new(RefCell::new(Vec::new()));
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| run_seed(seed, iterations, all, &steps)));
        let text = match result {
            Ok(differing) => {
                for (key, text) in differing {
                    let entry = by_node
                        .entry(key)
                        .or_insert_with(|| (0, format!("seed {seed}: {text}")));
                    entry.0 += 1;
                }
                continue;
            }
            Err(payload) => panic_text(payload),
        };
        let steps = steps.borrow();
        // The first entry is the pass before the first step, so a failure at step `n` needs
        // `n` iterations to recur.
        let step = steps.len().saturating_sub(1).max(1);
        let message = format!(
            "instance tree stress, seed {seed}, step {step} (rerun with NX_TREE_STRESS_SEED={seed} \
             NX_TREE_STRESS_ITERATIONS={step}): {text}\nsteps:\n  {}",
            steps.join("\n  ")
        );
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

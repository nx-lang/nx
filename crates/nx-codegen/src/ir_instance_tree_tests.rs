//! The instance tree against NX source: each test compiles a small program, links it, and drives
//! an `InstanceTree` the way a host does: visit the root's descriptor, visit each authored
//! descriptor in what a node rendered under that node, and dispatch the tokens the rendered
//! output carries.
//!
//! <para>There is one test for each scenario of the `component-instance-tree` capability, named
//! after it, and the cases of the DrawnUI fiddle's TypeScript tree (`nx/test/instances.test.ts`
//! in the DrawnUi.FiddleEngine repository) the scenarios do not already state. The records a
//! host would interpret itself are external components declared in the test source, in place of a
//! catalog.</para>

use crate::ir_runtime_tests::{json, program};
use nx_ir_runtime::{ComponentInit, HostEffect, InstanceTree, NxIrRuntimeError, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The controls every program here renders: what a host's catalog would declare.
const CONTROLS: &str = "
abstract external component <Node />
external component <Label extends Node Text?:string />
external component <Button extends Node Text?:string emits { Tapped { } } />
external component <Stack extends Node content Children?:Node+ />
";

fn options() -> RuntimeOptions {
    RuntimeOptions::default()
}

fn linked(source: &str) -> InstanceTree {
    InstanceTree::new(program(&format!("{CONTROLS}{source}")))
}

/// What a function of the program evaluates to: a descriptor nothing has instantiated yet.
fn evaluated(tree: &InstanceTree, function: &str) -> NxValue {
    tree.program()
        .evaluate_function(function, &[], &options())
        .unwrap_or_else(|error| panic!("{function}: {error}"))
}

/// Compiles a program and returns the tree over it, in a pass that has visited the node of what
/// `root()` renders at the key `root`.
fn open(source: &str) -> InstanceTree {
    let mut tree = linked(source);
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options())
        .unwrap_or_else(|error| panic!("root: {error}"));
    tree
}

fn rendered(tree: &InstanceTree, key: &str) -> Arc<NxValue> {
    Arc::clone(
        tree.rendered(key)
            .unwrap_or_else(|| panic!("no node at {key}")),
    )
}

/// Visits, at `key` under `parent`, the first descriptor of `component` in what `parent`
/// rendered.
fn visit_child(tree: &mut InstanceTree, key: &str, component: &str, parent: &str) -> Arc<NxValue> {
    let output = rendered(tree, parent);
    let descriptor = find(&output, component, None)
        .unwrap_or_else(|| panic!("{parent} rendered no {component}: {output:?}"));
    tree.visit(key, descriptor, Some(parent), &options())
        .unwrap_or_else(|error| panic!("{key}: {error}"))
}

fn type_of(value: &NxValue) -> Option<&str> {
    match value {
        NxValue::Record { type_name, .. } => type_name.as_deref(),
        _ => None,
    }
}

fn field<'a>(value: &'a NxValue, name: &str) -> Option<&'a NxValue> {
    match value {
        NxValue::Record { properties, .. } => properties.get(name),
        _ => None,
    }
}

/// The first record of type `type_name` in a rendered value, depth first, whose `Text` is `text`
/// when one is asked for.
fn find<'a>(value: &'a NxValue, type_name: &str, text: Option<&str>) -> Option<&'a NxValue> {
    match value {
        NxValue::Array(items) => items.iter().find_map(|item| find(item, type_name, text)),
        NxValue::Record { properties, .. } => {
            let is_it = type_of(value) == Some(type_name)
                && text.is_none_or(|text| {
                    field(value, "Text") == Some(&NxValue::String(text.to_string()))
                });
            if is_it {
                return Some(value);
            }
            properties
                .values()
                .find_map(|entry| find(entry, type_name, text))
        }
        _ => None,
    }
}

fn renders(tree: &InstanceTree, key: &str, type_name: &str, text: &str) -> bool {
    find(&rendered(tree, key), type_name, Some(text)).is_some()
}

/// The token of the `onTapped` handler of a button in the output of the node at `key`.
fn token(tree: &InstanceTree, key: &str, text: &str) -> String {
    let output = rendered(tree, key);
    let button = find(&output, "Button", Some(text))
        .unwrap_or_else(|| panic!("{key} renders no button '{text}': {output:?}"));
    match field(button, "onTapped").and_then(|handler| field(handler, "token")) {
        Some(NxValue::String(token)) => token.clone(),
        other => panic!("the button '{text}' of {key} carries no token: {other:?}"),
    }
}

fn tapped() -> NxValue {
    json(r#"{ "$type": "Button.Tapped" }"#)
}

/// Taps the button `text` in the output of the node at `key`.
fn tap(
    tree: &mut InstanceTree,
    key: &str,
    text: &str,
) -> Result<Vec<HostEffect>, NxIrRuntimeError> {
    let token = token(tree, key, text);
    tree.dispatch(key, &token, tapped(), &options())
}

fn state(tree: &InstanceTree, key: &str) -> NxValue {
    NxValue::Record {
        type_name: None,
        properties: tree
            .state(key)
            .unwrap_or_else(|| panic!("no node at {key}"))
            .clone(),
    }
}

fn generation(tree: &InstanceTree, key: &str) -> u64 {
    tree.instance(key)
        .unwrap_or_else(|| panic!("no node at {key}"))
        .generation()
}

const COUNTER: &str = "
component <Counter /> = {
  state { count:int = 0 }
  <Stack>
    <Label Text={if count > 0 { \"tapped\" } else { \"untapped\" }} />
    <Button Text=\"Tap\" onTapped=<Update count={count + 1} /> />
  </Stack>
}
component <Timer /> = {
  state { ticks:int = 7 }
  <Label Text={\"\" + ticks} />
}
let root() = { <Counter /> }
let timer() = { <Timer /> }
";

/// `Card` emits `Logged`, and `Page` binds a handler for it that patches `Page`.
const CARD: &str = "
component <Card extends Node emits { Logged { text:string } } /> = {
  <Button Text=\"Log\" onTapped=<Card.Logged text=\"hi\" /> />
}
component <Page /> = {
  state { last:string = \"\" }
  <Stack>
    <Label Text={last} />
    <Card onLogged=<Update last={action.text} /> />
  </Stack>
}
let root() = { <Page /> }
";

/// `Box` renders its content and keeps state of its own; `Page` puts a button of its own inside.
const CONTENT: &str = "
component <Box extends Node content Children:Node+ /> = {
  state { opened:int = 0 }
  <Stack>
    <Button Text=\"Open\" onTapped=<Update opened={opened + 1} /> />
    {Children}
  </Stack>
}
component <Page /> = {
  state { count:int = 0 }
  <Box>
    <Label Text={if count > 0 { \"counted\" } else { \"zero\" }} />
    <Button Text=\"More\" onTapped=<Update count={count + 1} /> />
  </Box>
}
let root() = { <Page /> }
";

/// `Child` patches itself and emits `A`, and the handler `Page` bound for `A` divides by zero.
const FAILING_PARENT: &str = "
component <Child extends Node emits { A { } } /> = {
  state { c:int = 0 }
  <Button Text=\"Go\" onTapped={<Update c={c + 1} /> <Child.A />} />
}
component <Page /> = {
  state { n:int = 1 }
  <Child onA=<Update n={n / 0} /> />
}
let root() = { <Page /> }
";

// ------------------------------------------------------------------------------------------------
// The Rust runtime exports an instance tree over a linked program
// ------------------------------------------------------------------------------------------------

#[test]
fn a_node_holds_what_the_lifecycle_renders() {
    let tree = open(COUNTER);
    let initialized = tree
        .program()
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();
    assert_eq!(*rendered(&tree, "root"), initialized.rendered);
    assert_eq!(tree.state("root"), Some(&initialized.state));
    assert_eq!(token(&tree, "root", "Tap"), "h1-1");
    assert_eq!(generation(&tree, "root"), initialized.instance.generation());
}

#[test]
fn an_authored_component_is_told_from_an_external_one() {
    let tree = linked("component <Card extends Node /> = { <Label /> }\nlet root() = { <Card /> }");
    assert!(tree.can_instantiate("Card"));
    assert!(!tree.can_instantiate("Label"));
    assert!(!tree.can_instantiate("Node"));
    assert!(!tree.can_instantiate("Missing"));
    assert!(!tree.can_instantiate("root"));
}

#[test]
fn a_limit_reached_inside_a_tree_operation_is_the_runtimes_diagnostic() {
    let mut tree = open(
        "
let total(n:int): int = { if n <= 0 { 0 } else { n + total(n - 1) } }
component <Counter /> = {
  state { count:int = 0 }
  <Button Text=\"Tap\" onTapped=<Update count={count + total(50)} /> />
}
let root() = { <Counter /> }",
    );
    let before = rendered(&tree, "root");
    let limited = RuntimeOptions {
        max_operations: Some(40),
        ..options()
    };
    let error = tree
        .dispatch("root", "h1-1", tapped(), &limited)
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-resource-limit");
    let limit = error.diagnostics[0].limit.expect("a limit");
    assert_eq!((limit.name, limit.value), ("maxOperations", Some(40)));

    // The tree is as it was: the same output, the same state, and a token that still dispatches.
    assert!(Arc::ptr_eq(&before, &rendered(&tree, "root")));
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));
    assert!(tree
        .dispatch("root", "h1-1", tapped(), &options())
        .unwrap()
        .is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1275 }"#));

    // A visit is under the options it is given as well.
    let mut tree = linked(COUNTER);
    let root = evaluated(&tree, "root");
    let starved = RuntimeOptions {
        max_operations: Some(1),
        ..options()
    };
    let error = tree.visit("root", &root, None, &starved).unwrap_err();
    assert_eq!(error.code(), "nx-ir-resource-limit");
    assert!(tree.rendered("root").is_none());
}

/// Visits the `component` each node renders under that node, from what `root()` renders down,
/// at the keys `1`, `2` and so on, until a node renders none or a visit fails. Returns how many
/// nodes it visited and the failure that stopped it.
fn descend(
    tree: &mut InstanceTree,
    component: &str,
    options: &RuntimeOptions,
) -> (usize, Option<NxIrRuntimeError>) {
    let mut descriptor = evaluated(tree, "root");
    let mut parent: Option<String> = None;
    let mut visited = 0;
    loop {
        let key = (visited + 1).to_string();
        let output = match tree.visit(&key, &descriptor, parent.as_deref(), options) {
            Ok(output) => output,
            Err(error) => return (visited, Some(error)),
        };
        visited += 1;
        match find(&output, component, None) {
            Some(next) => descriptor = next.clone(),
            None => return (visited, None),
        }
        parent = Some(key);
    }
}

#[test]
fn a_component_that_renders_itself_ends_at_the_component_depth() {
    let mut tree = linked(
        "
component <Loop extends Node /> = { <Stack><Loop /></Stack> }
let root() = { <Loop /> }",
    );
    tree.begin();
    let (visited, error) = descend(&mut tree, "Loop", &options());
    let error = error.expect("a tree without end is refused");
    assert_eq!(visited, 100);
    assert_eq!(error.code(), "nx-ir-resource-limit");
    let limit = error.diagnostics[0].limit.expect("a limit");
    assert_eq!((limit.name, limit.value), ("maxComponentDepth", Some(100)));
    assert!(
        error.diagnostics[0].message.contains("'Loop' component"),
        "{error}"
    );

    // The visit that was refused changed nothing: the hundred nodes above it stand, and a host
    // that runs its pass as one change has the tree it began with.
    assert!(tree.rendered("100").is_some());
    assert!(tree.rendered("101").is_none());
    let mut tree = InstanceTree::new(tree.program().clone());
    let failed = tree.atomically(|tree| {
        tree.begin();
        match descend(tree, "Loop", &options()) {
            (_, Some(error)) => Err(error),
            (visited, None) => Ok(visited),
        }
    });
    assert_eq!(failed.unwrap_err().code(), "nx-ir-resource-limit");
    assert!(tree.rendered("1").is_none());
}

#[test]
fn a_host_sets_the_component_depth() {
    let mut tree = linked(
        "
component <Level extends Node left:int /> = {
  <Stack>
    <Label Text={\"\" + left} />
    {if left > 0 { <Level left={left - 1} /> }}
  </Stack>
}
let root() = { <Level left={149} /> }",
    );
    let depth = |max_component_depth: u32| RuntimeOptions {
        max_component_depth,
        ..options()
    };

    // A hundred and fifty levels of data are past the default and within a limit the host raised.
    tree.begin();
    let (visited, error) = descend(&mut tree, "Level", &options());
    assert_eq!(visited, 100);
    assert_eq!(error.unwrap().code(), "nx-ir-resource-limit");
    let (visited, error) = descend(&mut tree, "Level", &depth(150));
    assert_eq!((visited, error), (150, None));
    assert!(renders(&tree, "150", "Label", "0"));

    // A lowered limit is the one in force, for a node the tree holds already as for a new one.
    let before = rendered(&tree, "4");
    let (visited, error) = descend(&mut tree, "Level", &depth(3));
    assert_eq!(visited, 3);
    let limit = error.unwrap().diagnostics[0].limit.expect("a limit");
    assert_eq!((limit.name, limit.value), ("maxComponentDepth", Some(3)));
    assert!(Arc::ptr_eq(&before, &rendered(&tree, "4")));
}

// ------------------------------------------------------------------------------------------------
// A node is one use of an authored component at one place in the output
// ------------------------------------------------------------------------------------------------

#[test]
fn the_first_visit_initializes_from_the_initial_state() {
    let tree = open(COUNTER);
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));
    assert!(renders(&tree, "root", "Label", "untapped"));
    assert_eq!(token(&tree, "root", "Tap"), "h1-1");
    assert_eq!(tree.instance("root").unwrap().component(), "Counter");
}

#[test]
fn a_child_receives_the_handler_its_parent_bound() {
    let mut tree = open(CARD);
    let page = rendered(&tree, "root");
    let descriptor = find(&page, "Card", None).expect("a Card descriptor");
    let Some(NxValue::String(bound)) =
        field(descriptor, "onLogged").and_then(|handler| field(handler, "token"))
    else {
        panic!("the parent's descriptor carries no handler token: {descriptor:?}");
    };
    visit_child(&mut tree, "root/card", "Card", "root");
    let (card, page) = (
        tree.instance("root/card").unwrap(),
        tree.instance("root").unwrap(),
    );
    assert!(card.bound_handler_is("onLogged", page, bound));
}

#[test]
fn another_component_at_the_same_place_starts_again() {
    let mut tree = open(COUNTER);
    for _ in 0..3 {
        tap(&mut tree, "root", "Tap").unwrap();
    }
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 3 }"#));

    let timer = evaluated(&tree, "timer");
    tree.visit("root", &timer, None, &options()).unwrap();
    assert_eq!(tree.instance("root").unwrap().component(), "Timer");
    assert_eq!(state(&tree, "root"), json(r#"{ "ticks": 7 }"#));
    assert_eq!(generation(&tree, "root"), 1);

    // And a Counter back at that place is a new one.
    let counter = evaluated(&tree, "root");
    tree.visit("root", &counter, None, &options()).unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));
}

#[test]
fn a_node_under_another_parent_starts_again_and_takes_what_was_under_it() {
    let source = format!(
        "{CONTENT}component <Timer /> = {{ <Label Text=\"tick\" /> }}\nlet timer() = {{ <Timer /> }}"
    );
    // Two Pages, and a Box visited at one key under the first.
    let mut tree = open(&source);
    let root = evaluated(&tree, "root");
    tree.visit("other", &root, None, &options()).unwrap();
    visit_child(&mut tree, "box", "Box", "root");
    tap(&mut tree, "box", "Open").unwrap();
    assert_eq!(state(&tree, "box"), json(r#"{ "opened": 1 }"#));

    // A Box at the same key under the other Page is at another place in the output.
    visit_child(&mut tree, "box", "Box", "other");
    assert_eq!(state(&tree, "box"), json(r#"{ "opened": 0 }"#));
    // Its handlers are the ones its new parent bound: the Page's button patches that Page.
    tap(&mut tree, "box", "More").unwrap();
    assert_eq!(state(&tree, "other"), json(r#"{ "count": 1 }"#));
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));

    // A node that is replaced takes the nodes under it with it: they belonged to output
    // its place no longer has.
    let timer = evaluated(&tree, "timer");
    tree.visit("other", &timer, None, &options()).unwrap();
    assert_eq!(tree.instance("other").unwrap().component(), "Timer");
    assert!(tree.rendered("box").is_none());
    assert!(tree.rendered("root").is_some());
}

#[test]
fn a_value_that_is_not_a_descriptor_and_a_key_that_names_no_node_are_refused() {
    let mut tree = open(COUNTER);
    let root = evaluated(&tree, "root");
    for value in [NxValue::Int(1), json(r#"{ "count": 1 }"#), json("[]")] {
        let error = tree.visit("other", &value, None, &options()).unwrap_err();
        assert_eq!(error.code(), "nx-ir-boundary-type", "{value:?}");
    }
    let error = tree
        .visit("child", &root, Some("missing"), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-instance-key");
    let error = tree
        .dispatch("missing", "h1-1", tapped(), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-instance-key");
    // A node is not visited under itself.
    let error = tree
        .visit("root", &root, Some("root"), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-instance-key");
    // An external component is the host's to interpret, and the lifecycle says so.
    let label = json(r#"{ "$type": "Label" }"#);
    let error = tree.visit("label", &label, None, &options()).unwrap_err();
    assert_eq!(error.code(), "nx-ir-component");
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));

    // A refused visit is not a visit of the pass: a pass that reached the root only by one
    // leaves no node there, and none at the keys the refused visits named.
    tree.begin();
    let error = tree
        .visit("root", &NxValue::Int(1), None, &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-boundary-type");
    assert!(tree.rendered("root").is_some());
    tree.finish();
    assert!(tree.rendered("root").is_none());
    for key in ["other", "child", "label"] {
        assert!(tree.rendered(key).is_none(), "{key}");
    }
}

// ------------------------------------------------------------------------------------------------
// Props flow down and state stays with the node
// ------------------------------------------------------------------------------------------------

#[test]
fn a_parent_that_renders_again_re_initializes_the_child_with_the_state_it_held() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tap(&mut tree, "root/box", "Open").unwrap();
    tap(&mut tree, "root/box", "Open").unwrap();
    tap(&mut tree, "root/box", "More").unwrap();
    assert!(renders(&tree, "root/box", "Label", "zero"));

    // The Page rendered again: its output holds another descriptor at the same place.
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();

    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 2 }"#));
    assert!(renders(&tree, "root/box", "Label", "counted"));
    // Initialized again, not dispatched: the tokens are those of a first render.
    assert_eq!(generation(&tree, "root/box"), 1);
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
}

#[test]
fn a_render_reaches_what_is_under_the_node_and_nothing_beside_it() {
    // Two Pages side by side under no node, each with a Card that emits to it and a Box.
    let mut tree = linked(
        "
component <Card extends Node emits { Logged { text:string } } /> = {
  state { logs:int = 0 }
  <Button Text=\"Log\" onTapped={<Update logs={logs + 1} /> <Card.Logged text=\"hi\" />} />
}
component <Box extends Node /> = {
  state { opened:int = 0 }
  <Button Text=\"Open\" onTapped=<Update opened={opened + 1} /> />
}
component <Page /> = {
  state { last:string = \"\" }
  <Stack><Card onLogged=<Update last={action.text} /> /><Box /></Stack>
}
let root() = { <Page /> }",
    );
    let root = evaluated(&tree, "root");
    let pass = |tree: &mut InstanceTree| {
        tree.begin();
        for page in ["one", "two"] {
            tree.visit(page, &root, None, &options()).unwrap();
            visit_child(tree, &format!("{page}/card"), "Card", page);
            visit_child(tree, &format!("{page}/box"), "Box", page);
        }
        tree.finish();
    };
    pass(&mut tree);
    tap(&mut tree, "one/box", "Open").unwrap();
    pass(&mut tree);
    let before: Vec<_> = ["one/box", "two", "two/card", "two/box"]
        .iter()
        .map(|key| rendered(&tree, key))
        .collect();

    // The Card's emit patches the first Page, which renders again.
    tap(&mut tree, "one/card", "Log").unwrap();
    pass(&mut tree);
    assert_eq!(state(&tree, "one"), json(r#"{ "last": "hi" }"#));
    // Both nodes under it were initialized again, with the state each held: the Card, which
    // the Page handed another handler, and the Box, which it handed nothing new.
    assert_eq!(generation(&tree, "one/card"), 1);
    assert_eq!(state(&tree, "one/card"), json(r#"{ "logs": 1 }"#));
    assert_eq!(generation(&tree, "one/box"), 1);
    assert_eq!(state(&tree, "one/box"), json(r#"{ "opened": 1 }"#));
    assert!(!Arc::ptr_eq(&before[0], &rendered(&tree, "one/box")));
    // The Page beside it, and what is under that one, did not render.
    for (held, key) in before[1..].iter().zip(["two", "two/card", "two/box"]) {
        assert!(Arc::ptr_eq(held, &rendered(&tree, key)), "{key}");
    }
}

#[test]
fn the_same_descriptor_leaves_the_node_alone() {
    let mut tree = open(COUNTER);
    tap(&mut tree, "root", "Tap").unwrap();
    let before = rendered(&tree, "root");
    let root = evaluated(&tree, "root");
    let again = tree.visit("root", &root, None, &options()).unwrap();
    assert!(Arc::ptr_eq(&before, &again));
    assert_eq!(token(&tree, "root", "Tap"), "h2-1");
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
}

#[test]
fn a_descriptor_that_reads_the_same_from_a_parent_initialized_again_is_a_new_one() {
    let mut tree = linked(
        "
component <Card extends Node emits { Logged { text:string } } /> = {
  state { logs:int = 0 }
  <Button Text=\"Log\" onTapped={<Update logs={logs + 1} /> <Card.Logged text=\"hi\" />} />
}
component <Page tag:string /> = {
  state { last:string = \"\" }
  <Card onLogged=<Update last={tag + action.text} /> />
}
let first() = { <Page tag=\"a\" /> }
let second() = { <Page tag=\"b\" /> }",
    );
    let (first, second) = (evaluated(&tree, "first"), evaluated(&tree, "second"));
    tree.visit("root", &first, None, &options()).unwrap();
    let card_of = |tree: &InstanceTree| {
        find(&rendered(tree, "root"), "Card", None)
            .cloned()
            .expect("a Card descriptor")
    };
    let before = card_of(&tree);
    visit_child(&mut tree, "root/card", "Card", "root");
    tap(&mut tree, "root/card", "Log").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "ahi" }"#));

    // The Page is given other props and initialized again. What it renders for the Card reads
    // as it did the first time, since a render numbers its tokens from one.
    tree.visit("root", &second, None, &options()).unwrap();
    assert_eq!(generation(&tree, "root"), 1);
    assert_eq!(card_of(&tree), before);

    visit_child(&mut tree, "root/card", "Card", "root");
    // Initialized again, with the state it held and the handler the Page now holds.
    assert_eq!(generation(&tree, "root/card"), 1);
    assert_eq!(state(&tree, "root/card"), json(r#"{ "logs": 1 }"#));
    let (card, page) = (
        tree.instance("root/card").unwrap(),
        tree.instance("root").unwrap(),
    );
    assert!(card.bound_handler_is("onLogged", page, "h1-1"));
    tap(&mut tree, "root/card", "Log").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "bhi" }"#));
}

// ------------------------------------------------------------------------------------------------
// A pass drops the nodes whose places left the output
// ------------------------------------------------------------------------------------------------

/// A `Page` that renders a `Counter` only while `shown` is true.
const SHOWN: &str = "
component <Counter extends Node /> = {
  state { count:int = 0 }
  <Button Text=\"Add\" onTapped=<Update count={count + 1} /> />
}
component <Page /> = {
  state { shown:boolean = true }
  <Stack>
    <Button Text=\"Toggle\" onTapped=<Update shown={!shown} /> />
    { if shown { <Counter /> } else { <Label Text=\"hidden\" /> } }
  </Stack>
}
let root() = { <Page /> }
";

#[test]
fn a_use_that_left_the_output_loses_its_state() {
    let mut tree = open(SHOWN);
    visit_child(&mut tree, "root/counter", "Counter", "root");
    tree.finish();
    for _ in 0..5 {
        tap(&mut tree, "root/counter", "Add").unwrap();
    }
    assert_eq!(state(&tree, "root/counter"), json(r#"{ "count": 5 }"#));

    // The Page renders again without the Counter. The pass visits the Page, finds no Counter in
    // what it rendered, and so visits none: the Counter's place is gone and the node with it.
    tap(&mut tree, "root", "Toggle").unwrap();
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    assert!(find(&rendered(&tree, "root"), "Counter", None).is_none());
    tree.finish();
    assert!(tree.rendered("root/counter").is_none());

    // The place comes back, and what is at it is a new Counter.
    tap(&mut tree, "root", "Toggle").unwrap();
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/counter", "Counter", "root");
    tree.finish();
    assert_eq!(state(&tree, "root/counter"), json(r#"{ "count": 0 }"#));
}

#[test]
fn an_unchanged_node_keeps_the_nodes_under_it_without_a_visit() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    for _ in 0..5 {
        tap(&mut tree, "root/box", "Open").unwrap();
    }
    // The Box rendered and nothing is under it, so a pass has only the Box to walk.
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    let held = rendered(&tree, "root/box");

    // Nothing has rendered since. A pass that visits the Page alone keeps the Box: the Page's
    // output is what it was, so the Box's place is still in it.
    for _ in 0..2 {
        tree.begin();
        let again = tree.visit("root", &root, None, &options()).unwrap();
        assert!(Arc::ptr_eq(&again, &rendered(&tree, "root")));
        assert!(tree.is_settled("root"));
        tree.finish();
        assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 5 }"#));
        assert!(Arc::ptr_eq(&held, &rendered(&tree, "root/box")));
    }
    // And the Box, not visited, still dispatches.
    tap(&mut tree, "root/box", "Open").unwrap();
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 6 }"#));

    // Once the Page has rendered again, a pass that visits it has walked new output: the Box it
    // did not visit there is one that output has no place for.
    tap(&mut tree, "root/box", "More").unwrap();
    assert!(!tree.is_settled("root"));
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    tree.finish();
    assert!(tree.rendered("root/box").is_none());
    assert!(tree.is_settled("root"));
}

#[test]
fn a_node_dispatched_against_after_its_visit_is_not_taken_for_walked() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    tap(&mut tree, "root/box", "Open").unwrap();
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    assert!(tree.is_settled("root"));

    // A pass visits the Page, which is settled, and so leaves the Box alone; then the Page is
    // dispatched against before the pass ends. The pass walked none of what the Page now
    // holds, so it drops nothing under the Page, and the Page still has output to walk.
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    tap(&mut tree, "root/box", "More").unwrap();
    tree.finish();
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 1 }"#));
    assert!(!tree.is_settled("root"));

    // The pass that follows walks it.
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    assert!(tree.is_settled("root"));
    assert!(renders(&tree, "root/box", "Label", "counted"));
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 1 }"#));
}

#[test]
fn a_node_visited_before_its_parent_rendered_again_does_not_count_as_visited() {
    // A pass walks the Page and the Counter under it; an event then hides the Counter; the host
    // walks again from the root, in the same pass, and finds no Counter in the Page's new output.
    let mut tree = open(SHOWN);
    visit_child(&mut tree, "root/counter", "Counter", "root");
    tree.finish();
    for _ in 0..6 {
        tap(&mut tree, "root/counter", "Add").unwrap();
    }
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/counter", "Counter", "root");
    assert_eq!(state(&tree, "root/counter"), json(r#"{ "count": 6 }"#));
    tap(&mut tree, "root", "Toggle").unwrap();
    assert!(!tree.is_settled("root"));
    tree.visit("root", &root, None, &options()).unwrap();
    assert!(find(&rendered(&tree, "root"), "Counter", None).is_none());
    tree.finish();
    // The Counter was visited, from output the Page no longer holds: its place is gone.
    assert!(tree.rendered("root/counter").is_none());
    assert!(tree.is_settled("root"));
    tap(&mut tree, "root", "Toggle").unwrap();
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/counter", "Counter", "root");
    tree.finish();
    assert_eq!(state(&tree, "root/counter"), json(r#"{ "count": 0 }"#));

    // The same for a node whose place the new output still has, when the host visits the
    // parent again and does not walk what it rendered: the node goes. Kept, it would sit under
    // a node called settled with a handler of the instance its parent no longer holds, and no
    // pass that stops at a settled node would ever reach it.
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tap(&mut tree, "root/box", "More").unwrap();
    tree.visit("root", &root, None, &options()).unwrap();
    tree.finish();
    assert!(tree.rendered("root/box").is_none());
    assert!(tree.is_settled("root"));
    // A pass that walks what the Page rendered finds the Box's place and fills it.
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    assert!(renders(&tree, "root/box", "Label", "counted"));
    tap(&mut tree, "root/box", "More").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 2 }"#));
}

/// `Top` holds a `Middle`, which holds a `Leaf` that keeps a count of its own, and a `Side`
/// beside the `Middle`.
const SETTLED: &str = "
component <Leaf extends Node /> = {
  state { taps:int = 0 }
  <Button Text=\"Leaf\" onTapped=<Update taps={taps + 1} /> />
}
component <Middle extends Node /> = { <Stack><Label Text=\"middle\" /><Leaf /></Stack> }
component <Side extends Node /> = {
  state { taps:int = 0 }
  <Button Text=\"Side\" onTapped=<Update taps={taps + 1} /> />
}
component <Top /> = { <Stack><Middle /><Side /></Stack> }
let root() = { <Top /> }
";

#[test]
fn a_settled_node_says_so_until_something_under_it_renders() {
    let mut tree = open(SETTLED);
    visit_child(&mut tree, "top/middle", "Middle", "root");
    visit_child(&mut tree, "top/middle/leaf", "Leaf", "top/middle");
    visit_child(&mut tree, "top/side", "Side", "root");
    let all = ["root", "top/middle", "top/middle/leaf", "top/side"];
    // Created and not yet through the end of a pass: each holds output no pass has walked.
    assert!(all.iter().all(|key| !tree.is_settled(key)));
    tree.finish();
    assert!(all.iter().all(|key| tree.is_settled(key)));
    assert!(!tree.is_settled("missing"));
    tap(&mut tree, "top/side", "Side").unwrap();
    tree.begin();
    let root = evaluated(&tree, "root");
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "top/side", "Side", "root");
    tree.finish();
    let side = rendered(&tree, "top/side");

    // A dispatch against the Leaf alone: it and what is above it have something to walk, and
    // the Side beside them does not.
    tap(&mut tree, "top/middle/leaf", "Leaf").unwrap();
    for key in ["root", "top/middle", "top/middle/leaf"] {
        assert!(!tree.is_settled(key), "{key}");
    }
    assert!(tree.is_settled("top/side"));

    // A pass that follows the changes: from the root to the Leaf, and not into the Side.
    tree.begin();
    let top = tree.visit("root", &root, None, &options()).unwrap();
    let middle = visit_child(&mut tree, "top/middle", "Middle", "root");
    visit_child(&mut tree, "top/middle/leaf", "Leaf", "top/middle");
    tree.finish();
    assert!(all.iter().all(|key| tree.is_settled(key)));
    // The Top and the Middle did not render: what they hand out is what they handed out.
    assert!(Arc::ptr_eq(&top, &rendered(&tree, "root")));
    assert!(Arc::ptr_eq(&middle, &rendered(&tree, "top/middle")));
    assert_eq!(generation(&tree, "top/middle"), 1);
    assert_eq!(state(&tree, "top/middle/leaf"), json(r#"{ "taps": 1 }"#));
    assert_eq!(state(&tree, "top/side"), json(r#"{ "taps": 1 }"#));
    assert!(Arc::ptr_eq(&side, &rendered(&tree, "top/side")));
}

#[test]
fn a_node_is_removed_by_key_with_the_nodes_under_it() {
    let mut tree = open(SETTLED);
    visit_child(&mut tree, "top/middle", "Middle", "root");
    visit_child(&mut tree, "top/middle/leaf", "Leaf", "top/middle");
    visit_child(&mut tree, "top/side", "Side", "root");
    tree.finish();
    tap(&mut tree, "top/middle/leaf", "Leaf").unwrap();
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "top/middle", "Middle", "root");
    visit_child(&mut tree, "top/middle/leaf", "Leaf", "top/middle");
    tree.finish();
    assert_eq!(state(&tree, "top/middle/leaf"), json(r#"{ "taps": 1 }"#));

    assert!(tree.remove("top/middle"));
    assert!(!tree.remove("top/middle"));
    assert!(tree.rendered("top/middle").is_none());
    assert!(tree.rendered("top/middle/leaf").is_none());
    // The Top has not rendered, so its output still has the Middle's place; a pass that
    // visits the Top alone leaves the place empty and the Side as it was.
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    tree.finish();
    assert!(tree.rendered("top/middle").is_none());
    assert!(tree.rendered("top/side").is_some());

    // A visit at the key is a first visit.
    visit_child(&mut tree, "top/middle", "Middle", "root");
    visit_child(&mut tree, "top/middle/leaf", "Leaf", "top/middle");
    assert_eq!(state(&tree, "top/middle/leaf"), json(r#"{ "taps": 0 }"#));
}

#[test]
fn a_visited_node_survives_the_pass() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    for _ in 0..5 {
        tap(&mut tree, "root/box", "Open").unwrap();
    }
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 5 }"#));
}

#[test]
fn a_node_whose_parent_left_the_output_leaves_with_it() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    // A pass that visits the child and not the node it was found under.
    tree.begin();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    assert!(tree.rendered("root").is_none());
    assert!(tree.rendered("root/box").is_none());
}

// ------------------------------------------------------------------------------------------------
// A handler runs against the instance whose body bound it
// ------------------------------------------------------------------------------------------------

#[test]
fn a_tap_patches_the_instance_that_bound_the_handler() {
    let mut tree = open(COUNTER);
    assert!(renders(&tree, "root", "Label", "untapped"));
    let effects = tap(&mut tree, "root", "Tap").unwrap();
    assert!(effects.is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
    assert!(renders(&tree, "root", "Label", "tapped"));
    // The new render handed out new tokens; the second tap reads the current one, and the state the
    // first left.
    assert_eq!(token(&tree, "root", "Tap"), "h2-1");
    tap(&mut tree, "root", "Tap").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 2 }"#));
}

#[test]
fn a_handler_in_a_content_child_patches_its_owner() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    // The Box's own button first, so it holds state of its own to keep.
    tap(&mut tree, "root/box", "Open").unwrap();
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 1 }"#));
    // The Page's button is in the Box's output, under the Box's token.
    let effects = tap(&mut tree, "root/box", "More").unwrap();
    assert!(effects.is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 1 }"#));
    assert_eq!(generation(&tree, "root/box"), 2);
}

#[test]
fn a_retired_token_is_refused() {
    let mut tree = open(COUNTER);
    tree.dispatch("root", "h1-1", tapped(), &options()).unwrap();
    let error = tree
        .dispatch("root", "h1-1", tapped(), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-handler-token");
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
}

#[test]
fn a_component_nested_under_itself_is_told_from_its_ancestor() {
    let mut tree = open(
        "
component <Rec extends Node /> = {
  state { open:boolean = false }
  <Stack>
    <Button Text=\"Toggle\" onTapped=<Update open={!open} /> />
    { if open { <Rec /> } else { <Label Text=\"closed\" /> } }
  </Stack>
}
let root() = { <Rec /> }",
    );
    // Open the outer one, visit the inner one it now renders, and open that too.
    tap(&mut tree, "root", "Toggle").unwrap();
    visit_child(&mut tree, "root/rec", "Rec", "root");
    tap(&mut tree, "root/rec", "Toggle").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "open": true }"#));
    assert_eq!(state(&tree, "root/rec"), json(r#"{ "open": true }"#));

    // Both now hold the same props and state, so the handlers their bodies bound are equal:
    // the same node with the same capture. They are still two handlers. A tap on the inner
    // one's button closes the inner one.
    tap(&mut tree, "root/rec", "Toggle").unwrap();
    assert_eq!(state(&tree, "root/rec"), json(r#"{ "open": false }"#));
    assert_eq!(state(&tree, "root"), json(r#"{ "open": true }"#));
    assert_eq!(generation(&tree, "root"), 2);
}

#[test]
fn an_event_from_output_a_dispatch_replaced_is_refused() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tap(&mut tree, "root/box", "Open").unwrap();
    // The Page's button is in the Box's output. The first tap patches the Page, which
    // renders again; no pass has visited the Box yet, and it still holds the handler of the Page's
    // earlier render under the same token.
    let token = token(&tree, "root/box", "More");
    tree.dispatch("root/box", &token, tapped(), &options())
        .unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));

    let (page, held) = (rendered(&tree, "root"), rendered(&tree, "root/box"));
    let error = tree
        .dispatch("root/box", &token, tapped(), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-handler-token");
    // Nothing ran: not against the Page, and not against the Box that was only handed the handler.
    assert!(Arc::ptr_eq(&page, &rendered(&tree, "root")));
    assert!(Arc::ptr_eq(&held, &rendered(&tree, "root/box")));
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 1 }"#));
    // The Box's own handler is the Box's still: its output is current for what its body bound.
    tap(&mut tree, "root/box", "Open").unwrap();
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 2 }"#));

    // After a pass the Box holds the Page's current handler, and the tap reaches the Page.
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/box", "Box", "root");
    tree.finish();
    tap(&mut tree, "root/box", "More").unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 2 }"#));
    assert_eq!(state(&tree, "root/box"), json(r#"{ "opened": 2 }"#));
}

#[test]
fn a_readout_joins_the_states_number_boolean_and_float_to_its_words_and_follows_a_tap() {
    let mut tree = open(
        "
component <Page /> = {
  state { taps:int = 0 open:boolean = false speed:float64 = 1 }
  <Stack>
    <Label Text={\"Tapped \" + taps + \"× · IsOpen: \" + open + \" · \" + speed + \"x\"} />
    <Button Text=\"Tap\" onTapped=<Update taps={taps + 1} open={!open} speed={speed / 2} /> />
  </Stack>
}
let root() = { <Page /> }",
    );
    assert!(renders(
        &tree,
        "root",
        "Label",
        "Tapped 0× · IsOpen: false · 1x"
    ));
    tap(&mut tree, "root", "Tap").unwrap();
    assert!(renders(
        &tree,
        "root",
        "Label",
        "Tapped 1× · IsOpen: true · 0.5x"
    ));
}

// ------------------------------------------------------------------------------------------------
// An emitted action is carried to the handler the parent bound
// ------------------------------------------------------------------------------------------------

#[test]
fn a_childs_emit_patches_the_parent() {
    let mut tree = open(CARD);
    visit_child(&mut tree, "root/card", "Card", "root");
    let effects = tap(&mut tree, "root/card", "Log").unwrap();
    assert!(effects.is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "hi" }"#));
    assert!(renders(&tree, "root", "Label", "hi"));
}

#[test]
fn two_emitted_actions_reach_the_parent_in_one_batch() {
    let mut tree = open(
        "
component <Child extends Node emits { A { } B { } } /> = {
  <Button Text=\"Both\" onTapped={<Child.A /> <Child.B />} />
}
component <Page /> = {
  state { a:int = 0 b:int = 0 }
  <Child onA=<Update a={a + 1} /> onB=<Update b={b + a} /> />
}
let root() = { <Page /> }",
    );
    visit_child(&mut tree, "root/child", "Child", "root");
    let effects = tap(&mut tree, "root/child", "Both").unwrap();
    assert!(effects.is_empty());
    // One batch against the parent, so the second handler reads what the first left.
    assert_eq!(state(&tree, "root"), json(r#"{ "a": 1, "b": 1 }"#));
    // And the parent rendered once.
    assert_eq!(generation(&tree, "root"), 2);
}

#[test]
fn an_emit_whose_handler_belongs_to_a_replaced_render_is_refused() {
    let source = "
component <Card extends Node emits { Logged { text:string } } /> = {
  state { logs:int = 0 }
  <Button Text=\"Log\" onTapped={<Update logs={logs + 1} /> <Card.Logged text=\"hi\" />} />
}
component <Page /> = {
  state { last:string = \"\" taps:int = 0 }
  <Stack>
    <Button Text=\"Tap\" onTapped=<Update taps={taps + 1} /> />
    <Card onLogged=<Update last={last + action.text} /> />
  </Stack>
}
let root() = { <Page /> }";
    let pass = |tree: &mut InstanceTree| {
        let root = evaluated(tree, "root");
        tree.begin();
        tree.visit("root", &root, None, &options()).unwrap();
        visit_child(tree, "root/card", "Card", "root");
        tree.finish();
    };

    // The Card's handler twice with no pass between: the first emit patched the Page, whose
    // new render bound another handler for the emit than the one the Card still holds.
    let mut tree = open(source);
    visit_child(&mut tree, "root/card", "Card", "root");
    assert!(tap(&mut tree, "root/card", "Log").unwrap().is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "hi", "taps": 0 }"#));
    let (page, card) = (rendered(&tree, "root"), rendered(&tree, "root/card"));
    let error = tap(&mut tree, "root/card", "Log").unwrap_err();
    assert_eq!(error.code(), "nx-ir-handler-token");
    // The Card's own patch is undone with the emit that could not be carried, and the action
    // was not handed to the host as though nobody had bound a handler for it.
    assert!(Arc::ptr_eq(&page, &rendered(&tree, "root")));
    assert!(Arc::ptr_eq(&card, &rendered(&tree, "root/card")));
    assert_eq!(state(&tree, "root/card"), json(r#"{ "logs": 1 }"#));
    pass(&mut tree);
    assert!(tap(&mut tree, "root/card", "Log").unwrap().is_empty());
    assert_eq!(state(&tree, "root/card"), json(r#"{ "logs": 2 }"#));
    assert_eq!(
        state(&tree, "root"),
        json(r#"{ "last": "hihi", "taps": 0 }"#)
    );

    // The Page's own handler first, then the Card's: the same, from the other side.
    let mut tree = open(source);
    visit_child(&mut tree, "root/card", "Card", "root");
    tap(&mut tree, "root", "Tap").unwrap();
    let error = tap(&mut tree, "root/card", "Log").unwrap_err();
    assert_eq!(error.code(), "nx-ir-handler-token");
    assert_eq!(state(&tree, "root/card"), json(r#"{ "logs": 0 }"#));
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "", "taps": 1 }"#));
    pass(&mut tree);
    assert!(tap(&mut tree, "root/card", "Log").unwrap().is_empty());
    assert_eq!(state(&tree, "root"), json(r#"{ "last": "hi", "taps": 1 }"#));
}

#[test]
fn an_emit_nobody_bound_is_a_host_effect() {
    let mut tree = open(
        "
action Saved = { }
component <Child extends Node emits { Saved } /> = {
  <Button Text=\"Save\" onTapped=<Saved /> />
}
component <Page /> = { <Child /> }
let root() = { <Page /> }",
    );
    visit_child(&mut tree, "root/child", "Child", "root");
    let effects = tap(&mut tree, "root/child", "Save").unwrap();
    assert_eq!(
        effects,
        [HostEffect {
            key: "root/child".to_string(),
            component: "Child".to_string(),
            action: json(r#"{ "$type": "Saved" }"#),
        }]
    );
}

#[test]
fn an_action_emitted_at_the_bottom_of_three_components_is_handled_at_the_top() {
    let mut tree = open(
        "
action Noted = { text:string }
component <Leaf extends Node emits { Picked { name:string } Noted } /> = {
  <Button Text=\"Pick\" onTapped={<Leaf.Picked name=\"deep\" /> <Noted text=\"from the leaf\" />} />
}
component <Middle extends Node emits { Chosen { name:string } } /> = {
  state { passed:int = 0 }
  <Stack>
    <Label Text={\"passed \" + passed} />
    <Leaf onPicked={<Update passed={passed + 1} /> <Middle.Chosen name={action.name + \"er\"} />} />
  </Stack>
}
component <Top /> = {
  state { picked:string = \"\" }
  <Stack>
    <Label Text={\"picked \" + picked} />
    <Middle onChosen=<Update picked={action.name} /> />
  </Stack>
}
let root() = { <Top /> }",
    );
    visit_child(&mut tree, "root/middle", "Middle", "root");
    visit_child(&mut tree, "root/middle/leaf", "Leaf", "root/middle");

    let effects = tap(&mut tree, "root/middle/leaf", "Pick").unwrap();

    // The leaf's emit ran the handler the middle bound, against the middle, whose own emit ran
    // the handler the top bound, against the top.
    assert_eq!(state(&tree, "root/middle"), json(r#"{ "passed": 1 }"#));
    assert_eq!(state(&tree, "root"), json(r#"{ "picked": "deeper" }"#));
    assert!(renders(&tree, "root", "Label", "picked deeper"));
    assert!(renders(&tree, "root/middle", "Label", "passed 1"));
    // Each of the three was dispatched against once.
    for key in ["root", "root/middle", "root/middle/leaf"] {
        assert_eq!(generation(&tree, key), 2, "{key}");
    }
    // What no component handled is the host's, named after the component that produced it.
    assert_eq!(
        effects,
        [HostEffect {
            key: "root/middle/leaf".to_string(),
            component: "Leaf".to_string(),
            action: json(r#"{ "$type": "Noted", "text": "from the leaf" }"#),
        }]
    );

    // The top rendered again, so the next pass hands the middle a new descriptor and initializes it
    // again. The descriptor the middle then holds for the leaf reads exactly as the first one
    // did, token and all, and names a handler of the middle's new instance: the leaf is
    // initialized again too, both keep their state, and the chain still reaches the top.
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    visit_child(&mut tree, "root/middle", "Middle", "root");
    visit_child(&mut tree, "root/middle/leaf", "Leaf", "root/middle");
    tree.finish();
    assert_eq!(state(&tree, "root/middle"), json(r#"{ "passed": 1 }"#));
    assert_eq!(generation(&tree, "root/middle/leaf"), 1);
    let effects = tap(&mut tree, "root/middle/leaf", "Pick").unwrap();
    assert_eq!(effects.len(), 1);
    assert_eq!(state(&tree, "root/middle"), json(r#"{ "passed": 2 }"#));
    assert_eq!(state(&tree, "root"), json(r#"{ "picked": "deeper" }"#));
}

// ------------------------------------------------------------------------------------------------
// A change to the tree is all or nothing
// ------------------------------------------------------------------------------------------------

#[test]
fn a_chain_that_fails_part_way_restores_every_node() {
    let mut tree = open(FAILING_PARENT);
    visit_child(&mut tree, "root/child", "Child", "root");
    let token = token(&tree, "root/child", "Go");
    let before = rendered(&tree, "root/child");

    let error = tree
        .dispatch("root/child", &token, tapped(), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-division-by-zero");
    // The child's dispatch is undone with the parent's failure.
    assert!(Arc::ptr_eq(&before, &rendered(&tree, "root/child")));
    assert_eq!(state(&tree, "root/child"), json(r#"{ "c": 0 }"#));
    assert_eq!(generation(&tree, "root/child"), 1);
    assert_eq!(state(&tree, "root"), json(r#"{ "n": 1 }"#));
    // The token read from the standing output still names a handler.
    let error = tree
        .dispatch("root/child", &token, tapped(), &options())
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-division-by-zero");
}

#[test]
fn a_failed_dispatch_leaves_the_instance_it_named_as_it_was() {
    let mut tree = open(
        "
component <Counter /> = {
  state { count:int = 0 }
  <Button Text=\"Tap\" onTapped=<Update count={count / 0} /> />
}
let root() = { <Counter /> }",
    );
    let before = rendered(&tree, "root");
    for _ in 0..2 {
        // The same token dispatches both times, since nothing about the instance changed.
        let error = tree
            .dispatch("root", "h1-1", tapped(), &options())
            .unwrap_err();
        assert_eq!(error.code(), "nx-ir-division-by-zero");
        assert!(Arc::ptr_eq(&before, &rendered(&tree, "root")));
    }
}

#[test]
fn a_pass_that_fails_leaves_the_tree_as_it_was() {
    let mut tree = open(CONTENT);
    visit_child(&mut tree, "root/box", "Box", "root");
    tap(&mut tree, "root/box", "More").unwrap();
    let root = evaluated(&tree, "root");
    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    // The Page rendered again and this pass has not reached the Box: it holds what it held before.
    let (before, instance) = (rendered(&tree, "root/box"), generation(&tree, "root/box"));
    assert!(renders(&tree, "root/box", "Label", "zero"));

    let failed = tree.atomically(|tree| {
        // The pass re-initializes the Box from the Page's new descriptor, then fails elsewhere.
        visit_child(tree, "root/box", "Box", "root");
        assert!(renders(tree, "root/box", "Label", "counted"));
        tree.visit("root/other", &NxValue::Int(1), Some("root"), &options())
    });
    assert_eq!(failed.unwrap_err().code(), "nx-ir-boundary-type");

    assert!(Arc::ptr_eq(&before, &rendered(&tree, "root/box")));
    assert_eq!(generation(&tree, "root/box"), instance);
    assert!(renders(&tree, "root/box", "Label", "zero"));
    // What the pass had visited before the change is as it was too: ending the pass here drops
    // the Box, which only the failed change reached.
    tree.finish();
    assert!(tree.rendered("root").is_some());
    assert!(tree.rendered("root/box").is_none());
}

#[test]
fn a_host_failure_inside_one_change_undoes_a_dispatch_and_a_pass() {
    let mut tree = open(COUNTER);
    let root = evaluated(&tree, "root");
    let before = rendered(&tree, "root");
    let failed: Result<(), String> = tree.atomically(|tree| {
        tap(tree, "root", "Tap").map_err(|error| error.to_string())?;
        tree.begin();
        tree.visit("root", &root, None, &options())
            .map_err(|error| error.to_string())?;
        Err("the host failed".to_string())
    });
    assert_eq!(failed.unwrap_err(), "the host failed");
    assert!(Arc::ptr_eq(&before, &rendered(&tree, "root")));
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 0 }"#));

    // A change that succeeds stays.
    let done: Result<(), NxIrRuntimeError> =
        tree.atomically(|tree| tap(tree, "root", "Tap").map(|_| ()));
    done.unwrap();
    assert_eq!(state(&tree, "root"), json(r#"{ "count": 1 }"#));
}

// ------------------------------------------------------------------------------------------------
// A handler bound outside any component is reported as inert
// ------------------------------------------------------------------------------------------------

/// `Card` emits `Logged`; `root` binds a handler for it outside any component, and `Page`'s body
/// binds the same one inside a component.
const INERT: &str = "
action Noted = { }
component <Card extends Node emits { Logged { } } /> = {
  <Button Text=\"Log\" onTapped=<Card.Logged /> />
}
component <Page emits { Noted } /> = { <Card onLogged=<Noted /> /> }
let root() = { <Card onLogged=<Noted /> /> }
let page() = { <Page /> }
";

#[test]
fn a_handler_on_the_top_level_element_is_stripped_and_named() {
    let mut tree = linked(INERT);
    let root = evaluated(&tree, "root");
    // Pure evaluation rendered the handler, so it carries no token.
    let handler = field(&root, "onLogged").expect("a handler record");
    assert_eq!(type_of(handler), Some("ActionHandler"));
    assert_eq!(field(handler, "token"), None);

    tree.begin();
    tree.visit("root", &root, None, &options()).unwrap();
    assert_eq!(tree.inert(), ["Card.onLogged"]);
    // The node was initialized without it: its emit is the host's.
    let effects = tap(&mut tree, "root", "Log").unwrap();
    assert_eq!(effects.len(), 1);
    assert_eq!(type_of(&effects[0].action), Some("Card.Logged"));

    // Each pass reports for itself, whether or not the node is initialized again.
    tree.begin();
    assert!(tree.inert().is_empty());
    tree.visit("root", &root, None, &options()).unwrap();
    assert_eq!(tree.inert(), ["Card.onLogged"]);
}

#[test]
fn a_handler_bound_outside_any_component_is_stripped_at_any_depth() {
    let mut tree = open(
        "
action Log = { }
component <Page extends Node content Children:Node+ /> = { <Stack>{Children}</Stack> }
let root() = { <Page><Stack><Button Text=\"Log\" onTapped=<Log /> /></Stack></Page> }",
    );
    assert_eq!(tree.inert(), ["Button.onTapped"]);
    // The instance renders the button with no handler.
    let output = rendered(&tree, "root");
    let button = find(&output, "Button", Some("Log")).expect("the button");
    assert_eq!(field(button, "onTapped"), None);
    tree.begin();
    assert!(tree.inert().is_empty());
}

#[test]
fn a_handler_bound_inside_a_component_is_kept() {
    let mut tree = linked(INERT);
    let page = evaluated(&tree, "page");
    tree.begin();
    tree.visit("root", &page, None, &options()).unwrap();
    visit_child(&mut tree, "root/card", "Card", "root");
    assert!(tree.inert().is_empty());

    // The child holds the handler: its emit runs it, and what the handler returns is the
    // effect of the component that bound it.
    let effects = tap(&mut tree, "root/card", "Log").unwrap();
    assert_eq!(
        effects,
        [HostEffect {
            key: "root".to_string(),
            component: "Page".to_string(),
            action: json(r#"{ "$type": "Noted" }"#),
        }]
    );
}

// ------------------------------------------------------------------------------------------------
// The stack budget
// ------------------------------------------------------------------------------------------------

#[test]
fn a_descriptor_that_nests_too_deeply_is_refused_within_the_stack_budget() {
    let mut tree = open(CONTENT);
    let mut nested = json(r#"{ "$type": "Label" }"#);
    for _ in 0..200 {
        nested = NxValue::Array(vec![nested]);
    }
    let descriptor = NxValue::Record {
        type_name: Some("Box".to_string()),
        properties: BTreeMap::from([("Children".to_string(), nested)]),
    };
    let tiny = RuntimeOptions {
        max_stack_bytes: 1 << 10,
        ..options()
    };
    let error = tree
        .visit("root/box", &descriptor, Some("root"), &tiny)
        .unwrap_err();
    assert_eq!(error.code(), "nx-ir-resource-limit");
    let limit = error.diagnostics[0].limit.expect("a limit");
    assert_eq!((limit.name, limit.value), ("maxStackBytes", Some(1024)));
    assert!(tree.rendered("root/box").is_none());
}

// ---------------------------------------------------------------- nodes held outside a pass

/// A list whose cells a host binds one at a time, as a layout engine realizes them: the template
/// is a function value, and what it returns for an item is a component with state of its own.
const CELLS: &str = "
component <Row extends Node n:int /> = {
  state { picks:int = 0 }
  <Button Text={\"Row \" + n} onTapped=<Update picks={picks + 1} /> />
}
let <Cell Item:int Index:int />: Node = <Row n={Item} />
let template() = { Cell }
component <Top /> = { <Label Text=\"a list\" /> }
let root() = { <Top /> }
";

/// What the template returns for one item: a descriptor no component rendered.
fn cell(tree: &InstanceTree, item: i64) -> NxValue {
    let template = evaluated(tree, "template");
    let args = BTreeMap::from([
        ("Item".to_owned(), NxValue::Int(item)),
        ("Index".to_owned(), NxValue::Int(item - 1)),
    ]);
    tree.program()
        .call_function(&template, &args, &options())
        .unwrap_or_else(|error| panic!("Cell({item}): {error}"))
}

/// A host that draws a virtualized list visits a cell's component when its layout binds the
/// cell, not while it walks the output, so no pass of the drawing's tree would see it. It keeps
/// those nodes in a second tree over the same program and ends no pass there: a node then lives
/// until the host removes it, a dispatch against it changes it alone, and neither tree knows of
/// the other.
#[test]
fn nodes_a_host_holds_outside_a_pass_stay_until_it_removes_them() {
    let mut drawing = open(CELLS);
    drawing.finish();
    let mut cells = InstanceTree::new(drawing.program().clone());

    // No pass is ever ended here. Each cell is visited under no node, at a key of the host's
    // choosing, after a `begin` that drops nothing: it empties what the last bind recorded.
    for item in 1..=3 {
        let descriptor = cell(&cells, item);
        cells.begin();
        cells
            .visit(&format!("cell/{item}"), &descriptor, None, &options())
            .unwrap();
    }
    tap(&mut cells, "cell/2", "Row 2").unwrap();
    tap(&mut cells, "cell/2", "Row 2").unwrap();
    assert_eq!(state(&cells, "cell/1"), json(r#"{ "picks": 0 }"#));
    assert_eq!(state(&cells, "cell/2"), json(r#"{ "picks": 2 }"#));

    // The cell is bound again, to the same item: the same descriptor leaves the node alone.
    let again = cell(&cells, 2);
    let before = generation(&cells, "cell/2");
    cells.visit("cell/2", &again, None, &options()).unwrap();
    assert_eq!(generation(&cells, "cell/2"), before);
    assert_eq!(state(&cells, "cell/2"), json(r#"{ "picks": 2 }"#));

    // Passes over the drawing's tree come and go; the cells are not its nodes.
    let root = evaluated(&drawing, "root");
    drawing.begin();
    drawing.visit("root", &root, None, &options()).unwrap();
    drawing.finish();
    assert!(drawing.rendered("cell/2").is_none());
    assert_eq!(state(&cells, "cell/2"), json(r#"{ "picks": 2 }"#));

    // The list's collection changed: the host drops the nodes it kept for it, by key.
    assert!(cells.remove("cell/2"));
    assert!(cells.rendered("cell/2").is_none());
    assert_eq!(state(&cells, "cell/1"), json(r#"{ "picks": 0 }"#));
    let fresh = cell(&cells, 2);
    cells.visit("cell/2", &fresh, None, &options()).unwrap();
    assert_eq!(state(&cells, "cell/2"), json(r#"{ "picks": 0 }"#));
}

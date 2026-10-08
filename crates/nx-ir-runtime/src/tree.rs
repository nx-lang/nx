//! The instance tree: one instance for each use of an authored component in a program's output,
//! kept as that output changes.
//!
//! <para>The lifecycle renders one instance at a time and composes nothing. A component's
//! rendered output can hold descriptors of other authored components, each of which needs an
//! instance of its own: created under the instance whose output held the descriptor, so a
//! handler the parent bound reaches the child; initialized again with the state it holds when
//! the parent renders again and hands it other props; and dropped when the output no longer has
//! its place. A handler found in one instance's output may have been bound by an enclosing
//! component's body and passed down as content, and an action a component emits belongs to the
//! handler its parent bound for it. [`InstanceTree`] is that composition, made only of
//! [`Program::initialize_component`] and [`Program::dispatch_component_actions`].</para>
//!
//! <para>The tree knows nothing of what a host does with the output. One host draws it; another
//! keeps a document, an index or some other structure derived from it. Either names each node
//! by a key of its own choosing for the node's place in the output, reads what the node
//! rendered, and visits the authored descriptors it finds there. The tree says which nodes have
//! rendered since the host last walked them ([`InstanceTree::is_settled`]), so a host that
//! follows changes walks only where something changed.</para>

use crate::component::{handler_property, ComponentInit, ComponentInstance};
use crate::error::{fail, Result};
use crate::eval::{RuntimeOptions, Stack};
use crate::module::{ComponentDecl, DeclarationKind};
use crate::program::Program;
use crate::value::{
    too_deep, Handler, ACTION_HANDLER_TYPE, HANDLER_INVOCATION_TYPE, MAX_VALUE_DEPTH,
};
use nx_value::NxValue;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// An action that left the tree: no component above the one that produced it bound a handler
/// for it, so it is the host's.
#[derive(Debug, Clone, PartialEq)]
pub struct HostEffect {
    /// The key of the node whose dispatch returned the action.
    pub key: String,
    /// The component that node is an instance of.
    pub component: String,
    pub action: NxValue,
}

/// One use of an authored component, at one place in the output.
///
/// <para>Everything a node holds is shared, so the copy of the tree that makes a change all or
/// nothing copies no value.</para>
#[derive(Debug, Clone)]
struct Node {
    component: Arc<str>,
    /// The key of the node whose rendered output held the descriptor.
    parent: Option<Arc<str>>,
    /// The descriptor the node was last initialized from. Another one means other props.
    descriptor: Arc<NxValue>,
    /// The parent's instance the descriptor's tokens were resolved through. A descriptor that
    /// reads the same in the output of another instance names that instance's handlers: tokens
    /// number a render's handlers from one, so two renders can spell them alike.
    through: Option<ComponentInstance>,
    instance: ComponentInstance,
    state: Arc<BTreeMap<String, NxValue>>,
    rendered: Arc<NxValue>,
    /// Whether the node has rendered since a pass last visited it: on being created, on being
    /// initialized again, and on a dispatch against it. Its output is then one no pass has
    /// walked, so the nodes under it may be ones that output no longer has a place for.
    unwalked: bool,
    /// Whether the node or a node under it is unwalked. A node that is neither is settled.
    /// Dropping a node can leave this set on the nodes above it until a pass ends, which costs a
    /// host a walk it did not need and nothing else.
    unsettled: bool,
}

/// A handler waiting to be dispatched against the node whose body created it.
struct Pending {
    key: Arc<str>,
    token: String,
    action: NxValue,
}

/// The instances behind the output of one linked program.
///
/// <para>A pass is [`begin`](Self::begin), a [`visit`](Self::visit) for each authored descriptor
/// the host finds, top down, and [`finish`](Self::finish), which drops the nodes whose places
/// the output no longer has. An event is one [`dispatch`](Self::dispatch), after which the host
/// runs a pass. Both leave the tree as it was when they fail: a dispatch always, a pass when it
/// runs inside [`atomically`](Self::atomically).</para>
///
/// <para>A pass need not walk what did not change. Under a node that
/// [`is_settled`](Self::is_settled), nothing has rendered since a pass last walked it, and
/// `finish` keeps everything there whether or not the pass visits it.</para>
#[derive(Debug, Clone)]
pub struct InstanceTree {
    program: Program,
    nodes: HashMap<Arc<str>, Node>,
    /// The nodes the pass has visited, each with the instance it held when the visit returned:
    /// a node dispatched against after its visit has rendered output that visit did not see.
    visited: HashMap<Arc<str>, ComponentInstance>,
    inert: Vec<String>,
}

impl InstanceTree {
    /// An empty tree over `program`.
    pub fn new(program: Program) -> Self {
        Self {
            program,
            nodes: HashMap::new(),
            visited: HashMap::new(),
            inert: Vec::new(),
        }
    }

    /// The program the tree's instances are of.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Whether `name` is a component the program can instantiate: a component entrypoint of the
    /// entry module that is declared with a body. A record in rendered output whose type this
    /// accepts is a descriptor to [`visit`](Self::visit); any other is the host's to interpret.
    pub fn can_instantiate(&self, name: &str) -> bool {
        component(&self.program, name)
            .is_some_and(|component| component.body.is_some() && !component.is_abstract)
    }

    /// Starts a pass.
    pub fn begin(&mut self) {
        self.visited.clear();
        self.inert.clear();
    }

    /// Ends a pass: drops the nodes whose places the output no longer has, so an instance lives
    /// as long as its place does.
    ///
    /// <para>A node the pass visited is kept. So is every node under a node the pass did not
    /// reach, and every node under a visited node that has not rendered since a pass last
    /// visited it: that node's output is what it was, so the places in it are still there, and
    /// the host need not have walked it. What goes is what the pass left unvisited under a
    /// visited node that has rendered since, whose output the host has just walked and did not
    /// find it in; a node under no node that the pass did not visit; and every node under a
    /// node that goes.</para>
    ///
    /// <para>An event may reach the tree while a pass is under way. A visit counts for what the
    /// node held when it was made: a node that rendered again after its visit has not been
    /// walked, so nothing under it goes for being unvisited and it is not settled; and a node
    /// under it that the pass visited before it rendered again, and not after, was read from
    /// output it no longer holds, and is as good as unvisited.</para>
    pub fn finish(&mut self) {
        let mut kept: HashMap<Arc<str>, bool> = HashMap::with_capacity(self.nodes.len());
        for key in self.nodes.keys() {
            // Up to a node whose fate is known, or past a node under no node; then back down.
            let mut chain = Vec::new();
            let mut next = Some(key);
            let mut fate = loop {
                let Some(key) = next else { break true };
                if let Some(known) = kept.get(key) {
                    break *known;
                }
                let Some((key, node)) = self.nodes.get_key_value(key) else {
                    break false;
                };
                chain.push((key, node));
                next = node.parent.as_ref();
            };
            while let Some((key, node)) = chain.pop() {
                let visited = self.visited.contains_key(key) && self.read_from_current(node);
                fate = match &node.parent {
                    None => visited,
                    Some(parent) => fate && (visited || !self.walked_anew(parent)),
                };
                kept.insert(Arc::clone(key), fate);
            }
        }
        self.nodes
            .retain(|key, _| kept.get(key).copied().unwrap_or(false));

        // What the pass visited it has walked, unless the node rendered again after its visit.
        for (key, seen) in &self.visited {
            if let Some(node) = self.nodes.get_mut(key) {
                if node.instance.is(seen) {
                    node.unwalked = false;
                }
            }
        }
        let unwalked: Vec<Arc<str>> = self
            .nodes
            .iter_mut()
            .filter_map(|(key, node)| {
                node.unsettled = false;
                node.unwalked.then(|| Arc::clone(key))
            })
            .collect();
        for key in unwalked {
            self.unsettle(Some(key));
        }
    }

    /// Whether `node` was initialized through the instance its parent holds now, so that a visit
    /// of it read its descriptor from the output the parent holds now. A node visited before its
    /// parent rendered again in the same pass was read from output the parent no longer holds:
    /// that visit says nothing about whether the new output has the node's place.
    fn read_from_current(&self, node: &Node) -> bool {
        match (&node.parent, &node.through) {
            (Some(parent), Some(through)) => self
                .nodes
                .get(parent)
                .is_some_and(|parent| parent.instance.is(through)),
            (None, None) => true,
            _ => false,
        }
    }

    /// Whether the pass visited the node at `key` while it held output no pass had walked, and
    /// the node has not rendered again since: the host has then walked the output the node
    /// holds, and what it did not visit under the node is not in it.
    fn walked_anew(&self, key: &str) -> bool {
        match (self.visited.get(key), self.nodes.get(key)) {
            (Some(seen), Some(node)) => node.unwalked && node.instance.is(seen),
            _ => false,
        }
    }

    /// Marks the node at `key` and every node above it as holding something unwalked.
    fn unsettle(&mut self, mut key: Option<Arc<str>>) {
        while let Some(node) = key.and_then(|key| self.nodes.get_mut(&key)) {
            // Every node above one that is marked is marked already.
            if node.unsettled {
                break;
            }
            node.unsettled = true;
            key = node.parent.clone();
        }
    }

    /// Whether nothing at `key` or under it has rendered since a pass last visited it.
    ///
    /// <para>A host that follows changes asks this of a node it has just visited. When the node
    /// is settled, what it and the nodes under it rendered is what the host last read there, so
    /// the pass may leave everything under it unvisited, and [`finish`](Self::finish) keeps it.
    /// When it is not, the host walks what the node rendered and visits what it finds, asking
    /// again at each node. A key that names no node is not settled.</para>
    pub fn is_settled(&self, key: &str) -> bool {
        self.nodes.get(key).is_some_and(|node| !node.unsettled)
    }

    /// Drops the node at `key` and every node under it, and says whether there was one.
    ///
    /// <para>A pass drops what the output no longer has. This is for a place the output still
    /// has and the host no longer wants an instance at, such as an item of a list that the host
    /// instantiates only while it is in view: under a node that has not rendered again, leaving
    /// it unvisited keeps it.</para>
    pub fn remove(&mut self, key: &str) -> bool {
        self.drop_under(key);
        self.visited.remove(key);
        self.nodes.remove(key).is_some()
    }

    /// The handler properties the pass found bound outside any component, each as
    /// `<Type>.<property>` after the record that carried it. Such a handler came from pure
    /// evaluation, which is what a root function is: no instance holds it, so nothing can run
    /// it, and the node was initialized without it.
    pub fn inert(&self) -> &[String] {
        &self.inert
    }

    /// What the node at `key` rendered. It is the same allocation for as long as the node does
    /// not render again, so a host that kept it can tell by `Arc::ptr_eq` whether it has.
    pub fn rendered(&self, key: &str) -> Option<&Arc<NxValue>> {
        self.nodes.get(key).map(|node| &node.rendered)
    }

    /// The state the node at `key` holds.
    pub fn state(&self, key: &str) -> Option<&BTreeMap<String, NxValue>> {
        self.nodes.get(key).map(|node| &*node.state)
    }

    /// The instance the node at `key` holds: the one its rendered output's tokens belong to.
    pub fn instance(&self, key: &str) -> Option<&ComponentInstance> {
        self.nodes.get(key).map(|node| &node.instance)
    }

    /// The node for the authored descriptor found at `key`, and what it rendered.
    ///
    /// <para>`parent` is the key of the node whose rendered output held the descriptor, or `None`
    /// for a descriptor that came from pure evaluation. The first visit at a key initializes
    /// the component from the descriptor's fields, through the parent's instance. A later visit
    /// with the same descriptor, read from the same instance of the parent, does nothing, and
    /// one with another descriptor of the same component under the same parent initializes the
    /// node again with the state it holds: props flow down and state stays. A descriptor of
    /// another component, or another parent, replaces the node, and what was under it, with a
    /// new one in its initial state.</para>
    ///
    /// <para>A node that renders again hands every node under it a descriptor read from another
    /// instance, so each of them is initialized again when the pass visits it, and so on down:
    /// a render reaches everything under the node that rendered. What is beside it, and above
    /// it unless an emit carried a change there, is left alone.</para>
    ///
    /// <para>Under no parent, a handler record without a token names nothing, so it is left out
    /// of the fields the node is initialized from and reported through
    /// [`inert`](Self::inert).</para>
    pub fn visit(
        &mut self,
        key: &str,
        descriptor: &NxValue,
        parent: Option<&str>,
        options: &RuntimeOptions,
    ) -> Result<Arc<NxValue>> {
        let stack = Stack::begin(options.max_stack_bytes);
        let key: Arc<str> = match self.nodes.get_key_value(key) {
            Some((key, _)) => Arc::clone(key),
            None => Arc::from(key),
        };
        let NxValue::Record {
            type_name: Some(component),
            properties,
        } = descriptor
        else {
            return fail(
                "nx-ir-boundary-type",
                format!("Expected the value at '{key}' to be a component descriptor: a record with a '$type'."),
            );
        };
        let above = match parent {
            None => None,
            Some(parent) => {
                let (parent, node) = self.node(parent)?;
                if self.is_under(&parent, &key) {
                    return fail(
                        "nx-ir-instance-key",
                        format!("The node at '{key}' cannot be visited under '{parent}', which is that node or one under it."),
                    );
                }
                Some((parent, node.instance.clone()))
            }
        };

        // A descriptor under no instance came from pure evaluation, so a handler record in it
        // has no token and names nothing: it is reported, and left out rather than refused.
        let mut inert = Vec::new();
        if above.is_none() {
            find_inert(descriptor, &mut inert, 0, &stack)?;
        }

        let existing = self.nodes.get(&key).filter(|node| {
            *node.component == **component
                && node.parent.as_deref() == above.as_ref().map(|(parent, _)| &**parent)
        });
        if let Some(node) = existing {
            let through_same = match (&node.through, &above) {
                (Some(before), Some((_, now))) => before.is(now),
                (None, None) => true,
                _ => false,
            };
            if through_same && same(&node.descriptor, descriptor, 0, &stack)? {
                let rendered = Arc::clone(&node.rendered);
                self.visited.insert(key, node.instance.clone());
                self.inert.append(&mut inert);
                return Ok(rendered);
            }
        }
        let stripped;
        let fields = if inert.is_empty() {
            properties
        } else {
            stripped = fields_without_inert(properties, 0, &stack)?;
            &stripped
        };
        let result = self.program.initialize_component(
            component,
            fields,
            &ComponentInit {
                parent: above.as_ref().map(|(_, instance)| instance),
                state: existing.map(|node| &*node.state),
            },
            options,
        )?;
        let kept = existing.is_some();
        let (parent, through) = above.unzip();
        let node = Node {
            component: Arc::from(component.as_str()),
            parent: parent.clone(),
            descriptor: Arc::new(copy(descriptor, 0, &stack)?),
            through,
            instance: result.instance,
            state: Arc::new(result.state),
            rendered: Arc::new(result.rendered),
            unwalked: true,
            unsettled: true,
        };
        let rendered = Arc::clone(&node.rendered);
        if !kept {
            // Whatever stood here was another component or under another parent: the nodes
            // under it belong to output this place no longer has.
            self.drop_under(&key);
        }
        // The pass reached the node only now that the visit succeeded: a visit that is refused
        // leaves the record of the pass, like the nodes, as it was.
        self.visited.insert(Arc::clone(&key), node.instance.clone());
        self.nodes.insert(key, node);
        self.unsettle(parent);
        self.inert.append(&mut inert);
        Ok(rendered)
    }

    /// Runs `work` as one change to the tree: when it fails, the nodes, what they hold and the
    /// record of what the pass visited are put back as they were before it, so what the host
    /// made of the tree before it still names the instances its events dispatch against.
    pub fn atomically<T, E>(
        &mut self,
        work: impl FnOnce(&mut Self) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        let nodes = self.nodes.clone();
        let visited = self.visited.clone();
        let inert = self.inert.clone();
        let result = work(self);
        if result.is_err() {
            self.nodes = nodes;
            self.visited = visited;
            self.inert = inert;
        }
        result
    }

    /// Runs the handler found under `token` in the output of the node at `key` with `action`,
    /// and returns the effects nothing in the tree handled.
    ///
    /// <para>The handler runs against the node whose body created it: the node named, or the
    /// ancestor farthest from it that holds the same handler, which is where a handler passed
    /// down through content or props was bound. Each effect that dispatch returns is routed to
    /// the handler the node's parent bound for that emit, dispatched in turn against the node
    /// that created it, until none remains to route. A failure anywhere in the chain leaves
    /// every node as it was before the call.</para>
    ///
    /// <para>A dispatch gives the nodes it ran against new instances, and the nodes under them
    /// keep the handlers of the old ones until a pass visits them again. So a host runs a pass
    /// after every dispatch, and [`is_settled`](Self::is_settled) says where it has work. An
    /// event that reaches the tree before the pass has, and whose handler or whose emit's
    /// handler belongs to a render the dispatch replaced, fails with `nx-ir-handler-token` and
    /// changes nothing; the host runs the pass and may send it again.</para>
    ///
    /// <para>Every dispatch routes its effects to strict ancestors of the node it ran against,
    /// so the pending node farthest from the root is dispatched first, with every entry pending
    /// for it in one batch: a node is dispatched at most once in a chain, and no entry holds a
    /// token an earlier dispatch of the chain retired.</para>
    pub fn dispatch(
        &mut self,
        key: &str,
        token: &str,
        action: NxValue,
        options: &RuntimeOptions,
    ) -> Result<Vec<HostEffect>> {
        self.atomically(|tree| tree.dispatch_chain(key, token, action, options))
    }

    fn dispatch_chain(
        &mut self,
        key: &str,
        token: &str,
        action: NxValue,
        options: &RuntimeOptions,
    ) -> Result<Vec<HostEffect>> {
        let (source, node) = self.node(key)?;
        let Some(handler) = node.instance.handler(token).map(Arc::clone) else {
            return fail(
                "nx-ir-handler-token",
                format!(
                    "Unknown handler token '{token}' for the '{}' instance at '{source}'.",
                    node.component
                ),
            );
        };
        let (key, token) = self.creator(Some(&source), &handler)?;
        let mut pending = vec![Pending { key, token, action }];
        let mut effects = Vec::new();
        while let Some(deepest) = pending
            .iter()
            .map(|entry| &entry.key)
            .max_by_key(|key| self.depth(key))
            .map(Arc::clone)
        {
            let (batch, rest): (Vec<Pending>, Vec<Pending>) =
                pending.into_iter().partition(|entry| entry.key == deepest);
            pending = rest;
            let batch: Vec<NxValue> = batch.into_iter().map(invocation).collect();
            let (_, node) = self.node(&deepest)?;
            let result =
                self.program
                    .dispatch_component_actions(&node.instance, &batch, options)?;
            let node = Node {
                component: Arc::clone(&node.component),
                parent: node.parent.clone(),
                descriptor: Arc::clone(&node.descriptor),
                through: node.through.clone(),
                instance: result.instance,
                state: Arc::new(result.state),
                rendered: Arc::new(result.rendered),
                unwalked: true,
                unsettled: true,
            };
            for action in result.effects {
                match self.route(&node, &action)? {
                    Some((key, token)) => pending.push(Pending { key, token, action }),
                    None => effects.push(HostEffect {
                        key: deepest.to_string(),
                        component: node.component.to_string(),
                        action,
                    }),
                }
            }
            let parent = node.parent.clone();
            self.nodes.insert(deepest, node);
            self.unsettle(parent);
        }
        Ok(effects)
    }

    /// Where an effect of `node`'s dispatch goes: to the handler the parent bound for the emit,
    /// against the node that created that handler, or to the host (`None`) when the component
    /// does not emit the action or nothing bound a handler for it.
    fn route(&self, node: &Node, effect: &NxValue) -> Result<Option<(Arc<str>, String)>> {
        let NxValue::Record {
            type_name: Some(action),
            ..
        } = effect
        else {
            return Ok(None);
        };
        let bound = component(&self.program, &node.component)
            .and_then(|component| {
                component.emits.iter().find(|emit| {
                    self.program
                        .data
                        .resolve(0, &emit.action)
                        .is_ok_and(|(_, _, declaration)| *declaration.name == **action)
                })
            })
            .and_then(|emit| node.instance.bound_handler(&handler_property(&emit.name)));
        match bound {
            Some(handler) => self.creator(node.parent.as_deref(), handler).map(Some),
            None => Ok(None),
        }
    }

    /// The node whose body created `handler`, and the token it holds it under: the node, `from`
    /// or an ancestor of it, farthest from `from` whose rendered output holds the handler.
    ///
    /// <para>That node is the creator only while a pass has visited everything that rendered. A
    /// node hands a handler down by rendering it, and the node below takes it from the instance
    /// it was initialized through. Once the node above is dispatched against or initialized
    /// again it holds other handlers, and until a pass initializes the nodes below it again,
    /// they hold handlers nothing above them does. The farthest holder is then a node the
    /// handler was only handed to, which shows in the instance it was initialized through
    /// holding the handler too; and a handler the parent bound may have no holder at all.
    /// Running it anywhere would patch the wrong instance or hand the host an action a parent
    /// bound a handler for, so both fail as a retired token does.</para>
    fn creator(&self, from: Option<&str>, handler: &Arc<Handler>) -> Result<(Arc<str>, String)> {
        let mut found = None;
        let mut next = from;
        while let Some((key, node)) = next.and_then(|key| self.nodes.get_key_value(key)) {
            if let Some(token) = node.instance.token_of(handler) {
                found = Some((Arc::clone(key), token, node));
            }
            next = node.parent.as_deref();
        }
        match found {
            Some((key, token, node))
                if !node
                    .through
                    .as_ref()
                    .is_some_and(|through| through.token_of(handler).is_some()) =>
            {
                Ok((key, token))
            }
            _ => fail(
                "nx-ir-handler-token",
                format!(
                    "The '{}' handler was bound by a render that a dispatch or a pass has since replaced: a pass has to visit the nodes that hold it before an event read from their earlier output can run.",
                    handler.action_name
                ),
            ),
        }
    }

    fn node(&self, key: &str) -> Result<(Arc<str>, &Node)> {
        match self.nodes.get_key_value(key) {
            Some((key, node)) => Ok((Arc::clone(key), node)),
            None => fail(
                "nx-ir-instance-key",
                format!("The instance tree holds no node at '{key}'."),
            ),
        }
    }

    /// How many nodes are above the node at `key`.
    fn depth(&self, key: &str) -> usize {
        let mut depth: usize = 0;
        let mut next = self.nodes.get(key).and_then(|node| node.parent.as_deref());
        while let Some(node) = next.and_then(|key| self.nodes.get(key)) {
            depth = depth.saturating_add(1);
            next = node.parent.as_deref();
        }
        depth
    }

    /// Whether the node at `key` is the node at `ancestor` or one under it.
    fn is_under(&self, key: &str, ancestor: &str) -> bool {
        let mut next = Some(key);
        while let Some(key) = next {
            if key == ancestor {
                return true;
            }
            next = self.nodes.get(key).and_then(|node| node.parent.as_deref());
        }
        false
    }

    /// Drops every node under the node at `key`.
    fn drop_under(&mut self, key: &str) {
        let children = |nodes: &HashMap<Arc<str>, Node>, of: &str| -> Vec<Arc<str>> {
            nodes
                .iter()
                .filter(|(_, node)| node.parent.as_deref() == Some(of))
                .map(|(key, _)| Arc::clone(key))
                .collect()
        };
        let mut dropped = children(&self.nodes, key);
        while let Some(key) = dropped.pop() {
            self.nodes.remove(&key);
            dropped.extend(children(&self.nodes, &key));
        }
    }
}

/// The declaration of the entry module's component `name`.
fn component<'p>(program: &'p Program, name: &str) -> Option<&'p ComponentDecl> {
    let entry = &program.data.entry;
    match &entry.entrypoint(&entry.component_entrypoints, name)?.1.kind {
        DeclarationKind::Component(component) => Some(component),
        _ => None,
    }
}

/// The batch entry that runs the handler under an entry's token with its action.
fn invocation(entry: Pending) -> NxValue {
    NxValue::Record {
        type_name: Some(HANDLER_INVOCATION_TYPE.to_string()),
        properties: BTreeMap::from([
            ("token".to_string(), NxValue::String(entry.token)),
            ("action".to_string(), entry.action),
        ]),
    }
}

/// Whether a value is a handler record no instance rendered: an `ActionHandler` that names its
/// action and carries no token.
fn is_inert(value: &NxValue) -> bool {
    matches!(
        value,
        NxValue::Record { type_name: Some(name), properties }
            if name == ACTION_HANDLER_TYPE
                && matches!(properties.get("action"), Some(NxValue::String(_)))
                && !properties.contains_key("token")
    )
}

// The walks below are over a value a host handed the tree, before any lifecycle call has looked
// at it. Each recurses once for a level of nesting, so each counts the levels and asks the
// stack budget, as the lifecycle's own walks do.

/// Fails when a value at `depth` that holds others is past what a value may nest or past the
/// stack the call may use.
fn enter(value: &NxValue, depth: u32, stack: &Stack) -> Result<()> {
    if matches!(value, NxValue::Array(_) | NxValue::Record { .. }) {
        if depth > MAX_VALUE_DEPTH {
            return too_deep();
        }
        if !stack.within() {
            return stack.value_too_deep();
        }
    }
    Ok(())
}

/// Names every inert handler in `value`, at any depth, as `<Type>.<property>` after the record
/// that carries it.
fn find_inert(value: &NxValue, found: &mut Vec<String>, depth: u32, stack: &Stack) -> Result<()> {
    enter(value, depth, stack)?;
    let deeper = depth.saturating_add(1);
    match value {
        NxValue::Array(items) => {
            for item in items {
                find_inert(item, found, deeper, stack)?;
            }
        }
        NxValue::Record {
            type_name,
            properties,
        } => {
            let owner = type_name.as_deref().unwrap_or("(record)");
            for (name, entry) in properties {
                if is_inert(entry) {
                    found.push(format!("{owner}.{name}"));
                } else {
                    find_inert(entry, found, deeper, stack)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// A record's fields without the inert handlers among them, at any depth.
fn fields_without_inert(
    fields: &BTreeMap<String, NxValue>,
    depth: u32,
    stack: &Stack,
) -> Result<BTreeMap<String, NxValue>> {
    let deeper = depth.saturating_add(1);
    fields
        .iter()
        .filter(|(_, entry)| !is_inert(entry))
        .map(|(name, entry)| Ok((name.clone(), without_inert(entry, deeper, stack)?)))
        .collect()
}

fn without_inert(value: &NxValue, depth: u32, stack: &Stack) -> Result<NxValue> {
    enter(value, depth, stack)?;
    Ok(match value {
        NxValue::Array(items) => NxValue::Array(
            items
                .iter()
                .map(|item| without_inert(item, depth.saturating_add(1), stack))
                .collect::<Result<_>>()?,
        ),
        NxValue::Record {
            type_name,
            properties,
        } => NxValue::Record {
            type_name: type_name.clone(),
            properties: fields_without_inert(properties, depth, stack)?,
        },
        other => other.clone(),
    })
}

/// A copy of a value for a node to keep.
fn copy(value: &NxValue, depth: u32, stack: &Stack) -> Result<NxValue> {
    enter(value, depth, stack)?;
    let deeper = depth.saturating_add(1);
    Ok(match value {
        NxValue::Array(items) => NxValue::Array(
            items
                .iter()
                .map(|item| copy(item, deeper, stack))
                .collect::<Result<_>>()?,
        ),
        NxValue::Record {
            type_name,
            properties,
        } => NxValue::Record {
            type_name: type_name.clone(),
            properties: properties
                .iter()
                .map(|(name, entry)| Ok((name.clone(), copy(entry, deeper, stack)?)))
                .collect::<Result<_>>()?,
        },
        other => other.clone(),
    })
}

/// Whether two values are the same value, as `==` on them says, for a descriptor a node holds
/// and one a visit brings.
fn same(left: &NxValue, right: &NxValue, depth: u32, stack: &Stack) -> Result<bool> {
    enter(right, depth, stack)?;
    let deeper = depth.saturating_add(1);
    match (left, right) {
        (NxValue::Array(left), NxValue::Array(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            for (left, right) in left.iter().zip(right) {
                if !same(left, right, deeper, stack)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (
            NxValue::Record {
                type_name: left_type,
                properties: left,
            },
            NxValue::Record {
                type_name: right_type,
                properties: right,
            },
        ) => {
            if left_type != right_type || left.len() != right.len() {
                return Ok(false);
            }
            // Both maps are in name order, so the same fields meet pairwise.
            for ((left_name, left), (right_name, right)) in left.iter().zip(right) {
                if left_name != right_name || !same(left, right, deeper, stack)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (NxValue::Array(_) | NxValue::Record { .. }, _)
        | (_, NxValue::Array(_) | NxValue::Record { .. }) => Ok(false),
        (left, right) => Ok(left == right),
    }
}

//! The instance tree: one instance for each use of an authored component in a program's output,
//! kept as that output changes.
//!
//! <para>The lifecycle renders one instance at a time and composes nothing. A component's
//! rendered output can hold descriptors of other authored components, each of which needs an
//! instance of its own: created under the instance whose output held the descriptor, so a
//! handler the parent bound reaches the child; initialized again with the state it holds when
//! the parent renders again and hands it other props, or kept and rebound to the parent's new
//! handlers when it hands the same ones; and dropped when the output no longer has its place.
//! A handler found in one instance's output may have been bound by an enclosing component's
//! body and passed down as content, and an action a component emits belongs to the handler its
//! parent bound for it. [`InstanceTree`] is that composition, made of
//! [`Program::initialize_component`] and [`Program::dispatch_component_actions`], and of
//! rebinding a kept instance, which evaluates nothing.</para>
//!
//! <para>The tree knows nothing of what a host does with the output. One host draws it; another
//! keeps a document, an index or some other structure derived from it. Either names each node
//! by a key of its own choosing for the node's place in the output, reads what the node
//! rendered, and visits the authored descriptors it finds there. The tree says which nodes have
//! rendered since the host last walked them ([`InstanceTree::is_settled`]), so a host that
//! follows changes walks only where something changed.</para>

use crate::component::{
    handler_property, ComponentInit, ComponentInstance, InitProps, NormalizedProps, Rebinding,
};
use crate::environment::{Environment, ENV_FIELD};
use crate::error::{fail, fail_limit, Limit, Result};
use crate::eval::{Meter, RuntimeOptions, Stack};
use crate::kept::{Input, Session, Store, User};
use crate::module::{ComponentDecl, DeclarationKind};
use crate::program::Program;
use crate::value::{
    get_field, handlers_equal, too_deep, Handler, Value, ACTION_HANDLER_TYPE,
    HANDLER_INVOCATION_TYPE, MAX_VALUE_DEPTH,
};
use nx_value::NxValue;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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

/// What changed among the tree's nodes since the host last asked: a diff by key between the
/// tree then and the tree now, each list in key order.
///
/// <para>A key with no node then and a node now is added, and one with a node then and none now
/// is removed, every node dropped with a node removed or replaced listed under its own key. A key
/// whose node was replaced in between, by a visit with another component or under another parent,
/// or removed and then visited again, is both. A key that holds the same node then and now is
/// rendered when the node's output changed in any way since, by a render or by a rebinding that
/// changed what its tokens name; a node that only took a new descriptor or new state is not
/// listed.</para>
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeChanges {
    pub rendered: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl TreeChanges {
    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        self.rendered.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }
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
    /// The overrides of the node's place: those of the node above, then the descriptor's `$env`.
    /// Its root values are not used; the tree's are.
    environment: Environment,
    /// Whether the node has rendered since a pass last visited it: on being created, on being
    /// initialized again, and on a dispatch against it. Its output is then one no pass has
    /// walked, so the nodes under it may be ones that output no longer has a place for.
    unwalked: bool,
    /// Whether the node or a node under it is unwalked. A node that is neither is settled.
    /// Dropping a node can leave this set on the nodes above it until a pass ends, which costs a
    /// host a walk it did not need and nothing else.
    unsettled: bool,
    /// The node's identity: one no other node of the tree has had, kept while the node renders
    /// again, so the change set tells a node replaced at its key from one that rendered.
    id: u64,
    /// The direct inputs of the node's last render, in the order it read them.
    record: Arc<[Input]>,
    /// The revision at which the record was last found valid in the node's environment.
    verified: u64,
}

impl Node {
    /// Whether the record can never be found valid: it holds a read beneath the render's own
    /// overrides that failed there.
    fn always_stale(&self) -> bool {
        self.record
            .iter()
            .any(|input| matches!(input, Input::Beneath { value: None, .. }))
    }
}

/// What [`InstanceTree::keep_node`] made of a visit of a node whose record holds.
enum Kept {
    /// The node kept its render, and this is its output.
    Rendered(Arc<NxValue>),
    /// A prop its render read differs: it is initialized again from these props.
    Normalized(NormalizedProps),
}

/// A node a rebinding changes: the instance it holds, the one replacing it, and the instance of
/// its parent its `through` is to name.
struct Change {
    key: Arc<str>,
    before: ComponentInstance,
    after: ComponentInstance,
    through: ComponentInstance,
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
    /// The root values every node's environment starts from.
    roots: Environment,
    /// The derived results renders evaluated, kept across renders, with the revision counter and
    /// the indexes of what read each root value and what used each kept result.
    store: Store,
    /// The identity the next node created gets.
    next_id: u64,
    /// The identity of the node at each key when the host last took the change set.
    taken: HashMap<Arc<str>, u64>,
    /// The keys whose node's output changed since the host last took the change set.
    rendered_since: HashSet<Arc<str>>,
}

impl InstanceTree {
    /// An empty tree over `program`.
    pub fn new(program: Program) -> Self {
        Self {
            program,
            nodes: HashMap::new(),
            visited: HashMap::new(),
            inert: Vec::new(),
            roots: Environment::default(),
            store: Store::default(),
            next_id: 0,
            taken: HashMap::new(),
            rendered_since: HashSet::new(),
        }
    }

    /// An empty tree over `program` whose nodes start from the root values of `roots`. Its
    /// overrides are ignored. The values are checked when a node first renders.
    pub fn with_environment(program: Program, roots: Environment) -> Self {
        let mut tree = Self::new(program);
        tree.roots = Environment::root(roots.roots().iter().cloned());
        tree
    }

    /// The options a call for a node runs under: `options` with the node's environment.
    fn options_for(&self, environment: &Environment, options: &RuntimeOptions) -> RuntimeOptions {
        RuntimeOptions {
            environment: environment.with_roots(&self.roots),
            ..options.clone()
        }
    }

    /// A new node's identity.
    fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Puts `node` at `key`, keeping the indexes current.
    fn put(&mut self, key: Arc<str>, node: Node) {
        let user = User::Node(Arc::clone(&key));
        if let Some(old) = self.nodes.get(&key) {
            let record = Arc::clone(&old.record);
            self.store.unlink(&user, &record);
        }
        self.store.link(&user, &node.record);
        self.nodes.insert(key, node);
    }

    /// Takes the node at `key` out, keeping the indexes current.
    fn take(&mut self, key: &str) -> Option<Node> {
        let (key, node) = self.nodes.remove_entry(key)?;
        self.store.unlink(&User::Node(key), &node.record);
        Some(node)
    }

    /// A session over the tree's kept results for one place of one operation.
    fn session(&self) -> Session {
        Session::new(self.store.clone())
    }

    /// Takes on what `session` found and kept.
    fn commit(&mut self, session: Session) {
        self.store = session.into_store();
    }

    /// Whether `node`'s record of its last render holds in `environment`, checked by the rule on
    /// kept results with `session`. A record found valid at the current revision in the
    /// environment the node holds needs no check.
    fn record_holds(
        &self,
        node: &Node,
        environment: &Environment,
        options: &RuntimeOptions,
        session: &Session,
    ) -> Result<bool> {
        if node.always_stale() {
            return Ok(false);
        }
        if node.environment == *environment && node.verified == self.store.revision {
            return Ok(true);
        }
        self.program.record_holds(
            &node.record,
            node.verified,
            &self.options_for(environment, options),
            session,
        )
    }

    /// The change set: what changed among the nodes since the host last took it, which this
    /// clears.
    ///
    /// <para>A host that derives something from the nodes, such as native controls or an index,
    /// updates exactly the nodes listed and needs no walk. [`is_settled`](Self::is_settled)
    /// remains for hosts that walk, and the two agree: a node is unsettled exactly when it or a
    /// node under it rendered or was added since a pass last walked it.</para>
    pub fn take_changes(&mut self) -> TreeChanges {
        let now: HashMap<Arc<str>, u64> = self
            .nodes
            .iter()
            .map(|(key, node)| (Arc::clone(key), node.id))
            .collect();
        let mut changes = TreeChanges::default();
        for (key, id) in &now {
            match self.taken.get(key) {
                Some(before) if before == id => {
                    if self.rendered_since.contains(key) {
                        changes.rendered.push(key.to_string());
                    }
                }
                _ => changes.added.push(key.to_string()),
            }
        }
        for (key, id) in &self.taken {
            if now.get(key) != Some(id) {
                changes.removed.push(key.to_string());
            }
        }
        changes.rendered.sort();
        changes.added.sort();
        changes.removed.sort();
        self.taken = now;
        self.rendered_since.clear();
        changes
    }

    /// The value of the environment value `name` at the node at `key`: what its body reads for
    /// `env.name` outside any override it writes itself.
    pub fn environment_value(
        &self,
        key: &str,
        name: &str,
        options: &RuntimeOptions,
    ) -> Result<NxValue> {
        let (_, node) = self.node(key)?;
        self.program
            .environment_value(name, &self.options_for(&node.environment, options))
    }

    /// Replaces the tree's root values and renders again, with the state it holds, each node
    /// whose last render read a value that is now different at its place. The nodes to check
    /// are found through the index from the values whose root value changed. A node rendered
    /// this way holds output no pass has walked, as after a dispatch. All or nothing: when a
    /// value is refused or a render fails, the tree is as it was.
    pub fn set_environment(&mut self, roots: Environment, options: &RuntimeOptions) -> Result<()> {
        self.atomically(|tree| tree.change_environment(roots, options))
    }

    fn change_environment(&mut self, roots: Environment, options: &RuntimeOptions) -> Result<()> {
        let roots = Environment::root(roots.roots().iter().cloned());
        let before = self.program.root_values(&self.roots, options)?;
        let after = self.program.root_values(&roots, options)?;
        let stack = Stack::begin(options.max_stack_bytes);
        let mut changed: BTreeSet<Arc<str>> = BTreeSet::new();
        for (name, value) in &after {
            let same = before.iter().any(|(known, old)| {
                known == name
                    && crate::value::values_equal(old, value, &Meter::free(), &stack)
                        .unwrap_or(false)
            });
            if !same {
                changed.insert(Arc::clone(name));
            }
        }
        for (name, _) in &before {
            if !after.iter().any(|(known, _)| known == name) {
                changed.insert(Arc::clone(name));
            }
        }
        self.roots = roots;
        let keys: BTreeSet<_> = changed
            .iter()
            .filter_map(|name| self.program.environment_key(name))
            .collect();
        if keys.is_empty() {
            return Ok(());
        }
        // Push: the root values take a new revision, and what the indexes reach from them is
        // possibly stale. Nothing is evaluated.
        let previous = self.store.revision;
        let stale = self.store.change_roots(&keys);
        let revision = self.store.revision;
        for (key, node) in &mut self.nodes {
            if node.verified == previous && !stale.contains(key) {
                node.verified = revision;
            }
        }
        // Pull: from the nodes nearest the root, each possibly stale node's record is checked
        // at its place, and the node renders again at the first input that does not hold.
        let mut candidates: Vec<Arc<str>> = stale.into_iter().collect();
        candidates.sort_by_key(|key| self.depth(key));
        for key in candidates {
            let Some(node) = self.nodes.get(&key).cloned() else {
                continue;
            };
            let session = self.session();
            let node_options = self.options_for(&node.environment, options);
            if !node.always_stale()
                && self.program.record_holds(
                    &node.record,
                    node.verified,
                    &node_options,
                    &session,
                )?
            {
                self.commit(session);
                if let Some(node) = self.nodes.get_mut(&key) {
                    node.verified = revision;
                }
                continue;
            }
            let NxValue::Record { properties, .. } = &*node.descriptor else {
                continue;
            };
            let (result, record) = self.program.initialize_tracked(
                &node.component,
                InitProps::Host(properties),
                &ComponentInit {
                    parent: node.through.as_ref(),
                    state: Some(&node.state),
                },
                &node_options,
                &session,
            )?;
            self.commit(session);
            let renewed = Node {
                instance: result.instance,
                state: Arc::new(result.state),
                rendered: Arc::new(result.rendered),
                unwalked: true,
                unsettled: true,
                record: Arc::from(record),
                verified: self.store.revision,
                ..node
            };
            let parent = renewed.parent.clone();
            self.rendered_since.insert(Arc::clone(&key));
            self.put(key, renewed);
            self.unsettle(parent);
        }
        self.store.sweep();
        Ok(())
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
        let gone: Vec<Arc<str>> = self
            .nodes
            .keys()
            .filter(|key| !kept.get(*key).copied().unwrap_or(false))
            .cloned()
            .collect();
        for key in gone {
            self.take(&key);
        }

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
        self.store.sweep();
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
        let removed = self.take(key).is_some();
        self.store.sweep();
        removed
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
    /// instance. One that is equal to the descriptor the node under it holds, with a handler the
    /// body can see compared as `==` compares it and one bound for an emit only there in both,
    /// leaves that node unrendered: it is rebound to the parent's new handlers, with the nodes
    /// initialized through it, and keeps its output, its tokens and whether it is settled. Any
    /// other descriptor initializes the node again, and so on down: a render reaches what under
    /// it was handed something new. What is beside it, and above it unless an emit carried a
    /// change there, is left alone.</para>
    ///
    /// <para>Under no parent, a handler record without a token names nothing, so it is left out
    /// of the fields the node is initialized from and reported through
    /// [`inert`](Self::inert).</para>
    ///
    /// <para>A node may be at most [`RuntimeOptions::max_component_depth`] deep, counting itself
    /// and the nodes above it. A visit for a deeper one fails with `nx-ir-resource-limit` naming
    /// `maxComponentDepth` and changes nothing, whether or not the tree holds the node already:
    /// each visit is a call of its own, so this is the only limit a component that renders
    /// itself meets.</para>
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
        // How deep the node is, counting itself and the nodes above it.
        let mut depth: usize = 1;
        let above = match parent {
            None => None,
            Some(parent) => {
                let (parent, node) = self.node(parent)?;
                let Some(above) = self.depth_apart_from(&parent, &key) else {
                    return fail(
                        "nx-ir-instance-key",
                        format!("The node at '{key}' cannot be visited under '{parent}', which is that node or one under it."),
                    );
                };
                depth = above.saturating_add(1);
                Some((parent, node.instance.clone()))
            }
        };
        let limit = options.max_component_depth;
        if depth > usize::try_from(limit).unwrap_or(usize::MAX) {
            return fail_limit(
                Limit {
                    name: "maxComponentDepth",
                    value: Some(u64::from(limit)),
                },
                format!("The '{component}' component would be nested more than {limit} component instances deep. A component that renders itself nests without end."),
            );
        }

        // A descriptor under no instance came from pure evaluation, so a handler record in it
        // has no token and names nothing: it is reported, and left out rather than refused.
        let mut inert = Vec::new();
        if above.is_none() {
            find_inert(descriptor, &mut inert, 0, &stack)?;
        }

        // The node's place: the overrides above it, then those its descriptor carries.
        let environment = match &above {
            Some((parent, _)) => self.node(parent)?.1.environment.enter(descriptor)?,
            None => Environment::default().enter(descriptor)?,
        };
        let existing = self
            .nodes
            .get(&key)
            .filter(|node| {
                *node.component == **component
                    && node.parent.as_deref() == above.as_ref().map(|(parent, _)| &**parent)
            })
            .cloned();
        let session = self.session();
        let stripped;
        let fields = if inert.is_empty() {
            properties
        } else {
            stripped = fields_without_inert(properties, 0, &stack)?;
            &stripped
        };
        let mut normalized: Option<NormalizedProps> = None;
        if let Some(node) = &existing {
            let through_same = match (&node.through, &above) {
                (Some(before), Some((_, now))) => before.is(now),
                (None, None) => true,
                _ => false,
            };
            let holds = self.record_holds(node, &environment, options, &session)?;
            if holds {
                self.commit(session);
                let revision = self.store.revision;
                if let Some(node) = self.nodes.get_mut(&key) {
                    node.verified = revision;
                }
                let kept = self.keep_node(
                    &key,
                    node,
                    descriptor,
                    fields,
                    above.as_ref().map(|(_, now)| now),
                    through_same,
                    &environment,
                    options,
                    &stack,
                )?;
                match kept {
                    Kept::Rendered(rendered) => {
                        self.inert.append(&mut inert);
                        self.store.sweep();
                        return Ok(rendered);
                    }
                    Kept::Normalized(props) => normalized = Some(props),
                }
                return self.initialize_node(
                    key,
                    existing.as_ref(),
                    component,
                    descriptor,
                    fields,
                    normalized.as_ref(),
                    above,
                    environment,
                    &mut inert,
                    options,
                    &stack,
                );
            }
        }
        self.commit(session);
        self.initialize_node(
            key,
            existing.as_ref(),
            component,
            descriptor,
            fields,
            normalized.as_ref(),
            above,
            environment,
            &mut inert,
            options,
            &stack,
        )
    }

    /// Initializes the node at `key` from `descriptor`, whose fields less the inert handlers are
    /// `fields`, or from the props already `normalized` from them, with the state `existing`
    /// holds when it is the node of the same component under the same parent; otherwise the node
    /// is new and what stood at the key goes with what was under it.
    #[allow(clippy::too_many_arguments)]
    fn initialize_node(
        &mut self,
        key: Arc<str>,
        existing: Option<&Node>,
        component: &str,
        descriptor: &NxValue,
        fields: &BTreeMap<String, NxValue>,
        normalized: Option<&NormalizedProps>,
        above: Option<(Arc<str>, ComponentInstance)>,
        environment: Environment,
        inert: &mut Vec<String>,
        options: &RuntimeOptions,
        stack: &Stack,
    ) -> Result<Arc<NxValue>> {
        let session = self.session();
        let props = match normalized {
            Some(normalized) => InitProps::Normalized(normalized),
            None => InitProps::Host(fields),
        };
        let (result, record) = self.program.initialize_tracked(
            component,
            props,
            &ComponentInit {
                parent: above.as_ref().map(|(_, instance)| instance),
                state: existing.map(|node| &*node.state),
            },
            &self.options_for(&environment, options),
            &session,
        )?;
        let descriptor = Arc::new(copy(descriptor, 0, stack)?);
        self.commit(session);
        let id = match existing {
            Some(node) => node.id,
            None => self.new_id(),
        };
        let (parent, through) = above.unzip();
        let node = Node {
            component: Arc::from(component),
            parent: parent.clone(),
            descriptor,
            through,
            instance: result.instance,
            state: Arc::new(result.state),
            rendered: Arc::new(result.rendered),
            environment,
            unwalked: true,
            unsettled: true,
            id,
            record: Arc::from(record),
            verified: self.store.revision,
        };
        let rendered = Arc::clone(&node.rendered);
        if existing.is_none() {
            // Whatever stood here was another component or under another parent: the nodes
            // under it belong to output this place no longer has.
            self.take(&key);
            self.drop_under(&key);
        }
        // The pass reached the node only now that the visit succeeded: a visit that is refused
        // leaves the record of the pass, like the nodes, as it was.
        self.visited.insert(Arc::clone(&key), node.instance.clone());
        self.rendered_since.insert(Arc::clone(&key));
        self.put(key, node);
        self.unsettle(parent);
        self.inert.append(inert);
        self.store.sweep();
        Ok(rendered)
    }

    /// Keeps the render of `node`, at `key`, whose record holds at its new place `environment`,
    /// when `descriptor` from the parent's instance `now` gives it nothing its render read
    /// anew: with the same descriptor through the same instance it is left alone; with one equal
    /// to it through another it is rebound; and with one whose recorded props, normalized, equal
    /// the ones it holds it takes the new props and is rebound when it needs to be. Otherwise it
    /// says the node is to be initialized again, from the props normalized for the comparison
    /// when there are some.
    #[allow(clippy::too_many_arguments)]
    fn keep_node(
        &mut self,
        key: &Arc<str>,
        node: &Node,
        descriptor: &NxValue,
        fields: &BTreeMap<String, NxValue>,
        now: Option<&ComponentInstance>,
        through_same: bool,
        environment: &Environment,
        options: &RuntimeOptions,
        stack: &Stack,
    ) -> Result<Kept> {
        let equal = same_descriptor(&node.descriptor, descriptor, stack)?;
        if through_same && equal {
            let rendered = Arc::clone(&node.rendered);
            let instance = node.instance.clone();
            // Only the overrides may differ: the node takes them, so a later change is judged
            // against its new place.
            if node.environment != *environment || !same(&node.descriptor, descriptor, 0, stack)? {
                let kept = copy(descriptor, 0, stack)?;
                if let Some(node) = self.nodes.get_mut(key) {
                    node.descriptor = Arc::new(kept);
                    node.environment = environment.clone();
                }
            }
            self.visited.insert(Arc::clone(key), instance);
            return Ok(Kept::Rendered(rendered));
        }
        let component = &*node.component;
        let bound = bound_properties(&self.program, component);
        if let (Some(before), Some(now), false) = (&node.through, now, through_same) {
            // Read from another instance of the parent, tokens of equal handlers can read
            // otherwise: the descriptors are compared handler by handler.
            let changes = self.program.metered(component, options, |meter, stack| {
                let mut rebinding = Rebinding::default();
                let across = Across {
                    before,
                    now,
                    meter,
                    stack,
                };
                if !across.descriptors(&node.descriptor, descriptor, &bound, &mut rebinding)? {
                    return Ok(None);
                }
                self.rebinding(key, now, rebinding, meter, stack).map(Some)
            })?;
            if let Some(changes) = changes {
                let kept = copy(descriptor, 0, stack)?;
                if let Some(node) = self.nodes.get_mut(key) {
                    node.environment = environment.clone();
                }
                return self.rebind(key, kept, changes).map(Kept::Rendered);
            }
        }
        // Another descriptor: the node keeps its render when no prop it read differs.
        let normalized = self.program.normalize_component_props(
            component,
            fields,
            now,
            &self.options_for(environment, options),
        )?;
        let changes = self.program.metered(component, options, |meter, stack| {
            let mut rebinding = Rebinding::default();
            let props = Props {
                meter,
                stack,
                rebinding: &mut rebinding,
            };
            if !props.read_ones_equal(&node.instance, &normalized, &bound)? {
                return Ok(None);
            }
            match now {
                Some(now) => self.rebinding(key, now, rebinding, meter, stack).map(Some),
                None => Ok(Some(Vec::new())),
            }
        })?;
        let Some(mut changes) = changes else {
            return Ok(Kept::Normalized(normalized));
        };
        let kept = copy(descriptor, 0, stack)?;
        match changes.first_mut() {
            Some(first) => {
                first.after = first
                    .after
                    .with_props(normalized.props, normalized.handler_props);
            }
            None => {
                if let Some(node) = self.nodes.get_mut(key) {
                    node.instance = node
                        .instance
                        .with_props(normalized.props, normalized.handler_props);
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(key) {
            node.environment = environment.clone();
        }
        self.rebind(key, kept, changes).map(Kept::Rendered)
    }

    /// What keeping the node at `key` changes. Its new descriptor equals the one it holds but was
    /// read from `now`, another instance of its parent than the one it was initialized through:
    /// the node and every node initialized through it, and through those in turn, swap the
    /// handlers `rebinding` replaces for their replacements instead of rendering again. Each
    /// change is a node, the instance it holds, the one replacing it, and the instance its
    /// `through` is to name. Nothing changes here, so a walk that fails leaves the tree as it was.
    fn rebinding(
        &self,
        key: &Arc<str>,
        now: &ComponentInstance,
        mut rebinding: Rebinding,
        meter: &Meter<'_, '_>,
        stack: &Stack,
    ) -> Result<Vec<Change>> {
        let (key, node) = self.node(key)?;
        let first = node
            .instance
            .rebound(&mut rebinding, meter, stack)?
            .unwrap_or_else(|| node.instance.clone());
        let mut changes = vec![Change {
            key,
            before: node.instance.clone(),
            after: first,
            through: now.clone(),
        }];
        let mut children: Option<HashMap<Arc<str>, Vec<Arc<str>>>> = None;
        let mut next = 0;
        while let Some(change) = changes.get(next) {
            next = next.saturating_add(1);
            if change.before.is(&change.after) {
                continue;
            }
            let (parent, before, after) = (
                Arc::clone(&change.key),
                change.before.clone(),
                change.after.clone(),
            );
            let children = children.get_or_insert_with(|| {
                let mut index: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
                for (key, node) in &self.nodes {
                    if let Some(parent) = &node.parent {
                        index
                            .entry(Arc::clone(parent))
                            .or_default()
                            .push(Arc::clone(key));
                    }
                }
                index
            });
            for child in children.get(&parent).into_iter().flatten() {
                let Some(node) = self.nodes.get(child) else {
                    continue;
                };
                // A node initialized through an instance the parent held before is one a pass has
                // yet to visit again; it is left as it is.
                if !node
                    .through
                    .as_ref()
                    .is_some_and(|through| through.is(&before))
                {
                    continue;
                }
                let rebound = node
                    .instance
                    .rebound(&mut rebinding, meter, stack)?
                    .unwrap_or_else(|| node.instance.clone());
                changes.push(Change {
                    key: Arc::clone(child),
                    before: node.instance.clone(),
                    after: rebound,
                    through: after.clone(),
                });
            }
        }
        Ok(changes)
    }

    /// Makes the `changes` [`rebinding`](Self::rebinding) planned for the node at `key`, which
    /// now holds `descriptor`, and returns what it rendered: the output it held. Each node keeps
    /// its rendered output, its state and whether it is walked and settled, and a visit of the
    /// pass that named its earlier instance names the new one.
    fn rebind(
        &mut self,
        key: &Arc<str>,
        descriptor: NxValue,
        changes: Vec<Change>,
    ) -> Result<Arc<NxValue>> {
        for change in changes {
            if !change.before.is(&change.after) {
                // The output's tokens name other handlers now.
                self.rendered_since.insert(Arc::clone(&change.key));
            }
            if self
                .visited
                .get(&change.key)
                .is_some_and(|seen| seen.is(&change.before))
            {
                self.visited
                    .insert(Arc::clone(&change.key), change.after.clone());
            }
            if let Some(node) = self.nodes.get_mut(&change.key) {
                node.instance = change.after;
                node.through = Some(change.through);
            }
        }
        let node = self.nodes.get_mut(key).map(|node| {
            node.descriptor = Arc::new(descriptor);
            (Arc::clone(&node.rendered), node.instance.clone())
        });
        match node {
            Some((rendered, instance)) => {
                self.visited.insert(Arc::clone(key), instance);
                Ok(rendered)
            }
            None => fail(
                "nx-ir-instance-key",
                format!("The instance tree holds no node at '{key}'."),
            ),
        }
    }

    /// Runs `work` as one change to the tree: when it fails, the nodes, what they hold and the
    /// record of what the pass visited are put back as they were before it, so what the host
    /// made of the tree before it still names the instances its events dispatch against. So are
    /// the kept results, the revision counter, the indexes of readers and users, and the change
    /// set.
    pub fn atomically<T, E>(
        &mut self,
        work: impl FnOnce(&mut Self) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        let nodes = self.nodes.clone();
        let visited = self.visited.clone();
        let inert = self.inert.clone();
        let roots = self.roots.clone();
        let store = self.store.clone();
        let next_id = self.next_id;
        let taken = self.taken.clone();
        let rendered_since = self.rendered_since.clone();
        let result = work(self);
        if result.is_err() {
            self.nodes = nodes;
            self.visited = visited;
            self.inert = inert;
            self.roots = roots;
            self.store = store;
            self.next_id = next_id;
            self.taken = taken;
            self.rendered_since = rendered_since;
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
            let node = node.clone();
            let session = self.session();
            let holds = self.record_holds(&node, &node.environment, options, &session)?;
            let node_options = self.options_for(&node.environment, options);
            let (result, record) = self.program.dispatch_tracked(
                &node.instance,
                &batch,
                !holds,
                &node_options,
                &session,
            )?;
            self.commit(session);
            let revision = self.store.revision;
            let rendered = result.rendered.is_some();
            let node = match result.rendered {
                Some(output) => Node {
                    instance: result.instance,
                    state: Arc::new(result.state),
                    rendered: Arc::new(output),
                    unwalked: true,
                    unsettled: true,
                    record: Arc::from(record),
                    verified: revision,
                    ..node
                },
                // Nothing the render read changed: the node keeps its output, its tokens and
                // whether it is walked and settled, and takes the new state.
                None => Node {
                    instance: result.instance,
                    state: Arc::new(result.state),
                    verified: revision,
                    ..node
                },
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
            if rendered {
                self.rendered_since.insert(Arc::clone(&deepest));
            }
            self.put(deepest, node);
            if rendered {
                self.unsettle(parent);
            }
        }
        self.store.sweep();
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

    /// How deep the node at `key` is, counting itself and the nodes above it; `None` when the
    /// node at `other` is one of them, so that `key` names that node or one under it.
    fn depth_apart_from(&self, key: &str, other: &str) -> Option<usize> {
        let mut depth: usize = 0;
        let mut next = Some(key);
        while let Some(key) = next {
            if key == other {
                return None;
            }
            depth = depth.saturating_add(1);
            next = self.nodes.get(key).and_then(|node| node.parent.as_deref());
        }
        Some(depth)
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
            self.take(&key);
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

/// The properties a parent binds handlers for the emits of the component `name` under
/// (`onLogged` for `emits Logged`). Their handlers are never in the body's scope.
fn bound_properties(program: &Program, name: &str) -> Vec<String> {
    component(program, name)
        .map(|component| {
            component
                .emits
                .iter()
                .map(|emit| handler_property(&emit.name))
                .collect()
        })
        .unwrap_or_default()
}

/// Compares a descriptor a node holds, read from the output of the parent's instance `before`,
/// with one a visit brings, read from the output of `now`, pairing the handlers they hold.
struct Across<'a, 'm, 'p> {
    before: &'a ComponentInstance,
    now: &'a ComponentInstance,
    /// What comparing handlers costs, as `==` charges it.
    meter: &'a Meter<'m, 'p>,
    stack: &'a Stack,
}

impl Across<'_, '_, '_> {
    /// Whether the two descriptors are equal, so that the node's body would render the same
    /// output from either. A handler the body can see is compared as `==` compares it; one
    /// bound for an emit, named in `bound`, need only be there in both. Every pair of handlers
    /// is recorded in `rebinding`, and a pairing that is not one to one makes them unequal.
    fn descriptors(
        &self,
        old: &NxValue,
        new: &NxValue,
        bound: &[String],
        rebinding: &mut Rebinding,
    ) -> Result<bool> {
        enter(new, 0, self.stack)?;
        let (
            NxValue::Record {
                type_name: old_type,
                properties: old,
            },
            NxValue::Record {
                type_name: new_type,
                properties: new,
            },
        ) = (old, new)
        else {
            return Ok(false);
        };
        // The overrides a descriptor carries are not props: the environment they make is
        // compared by what the node read.
        let old: Vec<_> = old.iter().filter(|(name, _)| *name != ENV_FIELD).collect();
        let new: Vec<_> = new.iter().filter(|(name, _)| *name != ENV_FIELD).collect();
        if old_type != new_type || old.len() != new.len() {
            return Ok(false);
        }
        for ((old_name, old), (new_name, new)) in old.into_iter().zip(new) {
            if old_name != new_name {
                return Ok(false);
            }
            let equal = if bound.contains(old_name) {
                match (token(old), token(new)) {
                    (Some(old), Some(new)) => self.handlers(old, new, false, rebinding)?,
                    _ => self.values(old, new, 1, rebinding)?,
                }
            } else {
                self.values(old, new, 1, rebinding)?
            };
            if !equal {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn values(
        &self,
        old: &NxValue,
        new: &NxValue,
        depth: u32,
        rebinding: &mut Rebinding,
    ) -> Result<bool> {
        enter(new, depth, self.stack)?;
        if let (Some(old), Some(new)) = (token(old), token(new)) {
            return self.handlers(old, new, true, rebinding);
        }
        let deeper = depth.saturating_add(1);
        match (old, new) {
            (NxValue::Array(old), NxValue::Array(new)) => {
                if old.len() != new.len() {
                    return Ok(false);
                }
                for (old, new) in old.iter().zip(new) {
                    if !self.values(old, new, deeper, rebinding)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (
                NxValue::Record {
                    type_name: old_type,
                    properties: old,
                },
                NxValue::Record {
                    type_name: new_type,
                    properties: new,
                },
            ) => {
                if old_type != new_type || old.len() != new.len() {
                    return Ok(false);
                }
                for ((old_name, old), (new_name, new)) in old.iter().zip(new) {
                    if old_name != new_name || !self.values(old, new, deeper, rebinding)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (NxValue::Array(_) | NxValue::Record { .. }, _)
            | (_, NxValue::Array(_) | NxValue::Record { .. }) => Ok(false),
            // The same number, sign of zero included: a body can tell `-0.0` from `0.0`.
            (NxValue::Float(old), NxValue::Float(new)) => Ok(old.to_bits() == new.to_bits()),
            (old, new) => Ok(old == new),
        }
    }

    /// Whether the handler `before` holds under `old` and the one `now` holds under `new` may
    /// stand for each other: equal as `==` finds them when `compare` asks, answering the same
    /// emit when it does not, and never one earlier handler replaced by two new ones. A token
    /// either instance does not hold makes them unequal; the visit then initializes the node,
    /// which reports the token.
    fn handlers(
        &self,
        old: &str,
        new: &str,
        compare: bool,
        rebinding: &mut Rebinding,
    ) -> Result<bool> {
        let (Some(old), Some(new)) = (self.before.handler(old), self.now.handler(new)) else {
            return Ok(false);
        };
        if compare {
            if !Arc::ptr_eq(old, new) && !handlers_equal(old, new, self.meter, self.stack)? {
                return Ok(false);
            }
        } else if old.component != new.component
            || old.emit != new.emit
            || old.action_name != new.action_name
        {
            // A handler bound for an emit answers that emit, as initializing the node checks.
            return Ok(false);
        }
        Ok(rebinding.pair(old, new))
    }
}

/// Compares the props a node's render read with the ones a new descriptor gives it, normalized,
/// as [`Across`] compares two descriptors: a handler compared as `==` compares it and paired with
/// the one replacing it, a handler bound for an emit bound in both or in neither and answering the
/// same emit, and a number of the same kind with the same sign of zero.
struct Props<'a, 'm, 'p> {
    meter: &'a Meter<'m, 'p>,
    stack: &'a Stack,
    rebinding: &'a mut Rebinding,
}

impl Props<'_, '_, '_> {
    /// Whether every prop the render behind `held` read, and every handler bound for an emit
    /// named in `bound`, is the same in `new`. A render whose reads are unknown read every prop.
    fn read_ones_equal(
        mut self,
        held: &ComponentInstance,
        new: &NormalizedProps,
        bound: &[String],
    ) -> Result<bool> {
        let names: Vec<Arc<str>> = match held.render_reads() {
            Some(reads) => reads.props.clone(),
            None => held
                .props()
                .iter()
                .chain(new.props.iter())
                .map(|(name, _)| Arc::clone(name))
                .collect(),
        };
        let empty = Value::empty();
        for name in &names {
            let old = get_field(held.props(), name).unwrap_or(&empty);
            let now = get_field(&new.props, name).unwrap_or(&empty);
            if !self.values(old, now, 0)? {
                return Ok(false);
            }
        }
        for property in bound {
            let find = |handlers: &[(Arc<str>, Arc<Handler>)]| {
                handlers
                    .iter()
                    .find(|(name, _)| **name == **property)
                    .map(|(_, handler)| Arc::clone(handler))
            };
            match (find(held.handler_props()), find(&new.handler_props)) {
                (None, None) => {}
                (Some(old), Some(now)) => {
                    if old.component != now.component
                        || old.emit != now.emit
                        || old.action_name != now.action_name
                        || !self.rebinding.pair(&old, &now)
                    {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn values(&mut self, old: &Value, new: &Value, depth: u32) -> Result<bool> {
        if matches!(new, Value::Seq(_) | Value::Record(_)) {
            if depth > MAX_VALUE_DEPTH {
                return too_deep();
            }
            if !self.stack.within() {
                return self.stack.value_too_deep();
            }
        }
        let deeper = depth.saturating_add(1);
        Ok(match (old, new) {
            (Value::Handler(old), Value::Handler(new)) => {
                (Arc::ptr_eq(old, new) || handlers_equal(old, new, self.meter, self.stack)?)
                    && self.rebinding.pair(old, new)
            }
            (Value::Seq(old), Value::Seq(new)) => {
                if old.len() != new.len() {
                    return Ok(false);
                }
                for (old, new) in old.iter().zip(new.iter()) {
                    if !self.values(old, new, deeper)? {
                        return Ok(false);
                    }
                }
                true
            }
            (Value::Record(old), Value::Record(new)) => {
                if old.type_name != new.type_name || old.fields.len() != new.fields.len() {
                    return Ok(false);
                }
                for ((old_name, old), (new_name, new)) in old.fields.iter().zip(new.fields.iter()) {
                    if old_name != new_name || !self.values(old, new, deeper)? {
                        return Ok(false);
                    }
                }
                true
            }
            // The same number, sign of zero included: a body can tell `-0.0` from `0.0`.
            (Value::Float(old), Value::Float(new)) => old.to_bits() == new.to_bits(),
            (Value::Int(old), Value::Int(new)) => old == new,
            (Value::Bool(old), Value::Bool(new)) => old == new,
            (Value::Str(old), Value::Str(new)) => old == new,
            (Value::Case(old), Value::Case(new)) => old.union == new.union && old.case == new.case,
            (Value::Function(old), Value::Function(new)) => old == new,
            _ => false,
        })
    }
}

/// The token of a handler record a render wrote, when `value` is one.
fn token(value: &NxValue) -> Option<&str> {
    match value {
        NxValue::Record {
            type_name: Some(name),
            properties,
        } if name == ACTION_HANDLER_TYPE => match properties.get("token") {
            Some(NxValue::String(token)) => Some(token),
            _ => None,
        },
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

/// Whether two descriptors are the same leaving out the overrides both carry in `$env`.
fn same_descriptor(left: &NxValue, right: &NxValue, stack: &Stack) -> Result<bool> {
    match (left, right) {
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
            let left: Vec<_> = left.iter().filter(|(name, _)| *name != ENV_FIELD).collect();
            let right: Vec<_> = right
                .iter()
                .filter(|(name, _)| *name != ENV_FIELD)
                .collect();
            if left_type != right_type || left.len() != right.len() {
                return Ok(false);
            }
            for ((left_name, left), (right_name, right)) in left.into_iter().zip(right) {
                if left_name != right_name || !same(left, right, 1, stack)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => same(left, right, 0, stack),
    }
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

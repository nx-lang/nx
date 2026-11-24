//! Derived results the instance tree keeps across renders, checked by revision.
//!
//! <para>Within one call a derived result is reused while every value it read has the same token
//! ([`crate::reuse`]). The instance tree keeps the results its renders evaluated beyond the call,
//! as incremental engines such as Salsa do: each *variant* of a kept result records its direct
//! inputs in the order it read them, the revision at which its value last changed and the
//! revision at which it was last found valid. A root read holds while the root value has not
//! changed since then and every kept result that supplied it holds; an edge to another kept
//! result is checked by validating that result first, evaluating it again when it is not valid,
//! and comparing the revision at which it last changed; a read an override of the place supplies
//! is compared by the value the lookup gives there. A result evaluated again to the same value as
//! a host would see it written keeps its old revision, so nothing that used it is evaluated or
//! rendered again.</para>
//!
//! <para>One result can come out differently at different places, so a result keeps one variant
//! for each way its reads are supplied, and a use takes the first variant valid there. A render
//! takes a variant only when no layer the render pushed touches a value the variant read at any
//! depth, and the variant then enters the call's own reuse as if it had been evaluated at the
//! call's start.</para>

use crate::environment::{EnvKey, Layer};
use crate::error::{Result, RESOURCE_LIMIT};
use crate::eval::{Cx, Machine, Meter, Stack};
use crate::reuse::Derived;
use crate::value::{handlers_equal, Value};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// Which variant of a kept result: a number no other variant of the tree has had.
pub(crate) type VariantId = u64;

/// One direct input of a node's record of its last render or of a variant's record.
#[derive(Debug, Clone)]
pub(crate) enum Input {
    /// A read of an environment value no layer of the place sets: the root value's revision is
    /// checked, then each kept result that supplied the value (a provide or the default).
    Root {
        key: EnvKey,
        edges: Arc<[VariantId]>,
    },
    /// A kept result used directly: a top-level value.
    Edge(VariantId),
    /// A read a layer of the place sets, wholly or at a field path, with the value found and the
    /// values its lookup read.
    Override {
        key: EnvKey,
        value: Value,
        lookup: Arc<[EnvKey]>,
    },
    /// A read of a node's render that a layer of the place sets, made while evaluating a source
    /// under the render's own overrides of other values: the value found beneath those
    /// overrides, or none when reading it there failed, which counts as changed. The direct
    /// inputs of what it evaluated follow it in the record.
    Beneath { key: EnvKey, value: Option<Value> },
}

/// What depends on a kept result or on a root value: a node, by key, or a variant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum User {
    Node(Arc<str>),
    Variant(VariantId),
}

/// One variant of a kept result.
#[derive(Debug, Clone)]
pub(crate) struct Variant {
    pub derived: Derived,
    pub value: Value,
    /// The revision at which the value last changed.
    pub changed: u64,
    /// The revision at which the variant was last found valid.
    pub verified: u64,
    pub inputs: Arc<[Input]>,
    /// Whether the value was made in a call that collects origins, so its records carry them.
    pub origins: bool,
}

/// The tree's kept results, its revision counter and its reverse indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct Store {
    pub revision: u64,
    next: VariantId,
    /// The revision at which each root value last changed: set, changed or cleared.
    roots: HashMap<EnvKey, u64>,
    pub variants: HashMap<VariantId, Variant>,
    /// The variants of each kept result, in the order they were kept.
    by_derived: HashMap<Derived, Vec<VariantId>>,
    /// For each variant, the nodes and variants whose records hold an edge to it.
    users: HashMap<VariantId, BTreeSet<User>>,
    /// For each environment value, the nodes and variants with a root read of it, an
    /// override-supplied read whose lookup read it, or a read beneath a render's own overrides.
    readers: HashMap<EnvKey, BTreeSet<User>>,
    /// The variants that were kept or lost a user since the last sweep: the only ones a sweep
    /// can find unused.
    orphans: BTreeSet<VariantId>,
}

impl Store {
    /// The revision at which the root value of `key` last changed.
    fn root_revision(&self, key: EnvKey) -> u64 {
        self.roots.get(&key).copied().unwrap_or(0)
    }

    /// Starts a new revision in which the root values of `keys` changed, and returns what that
    /// marks possibly stale: the nodes and variants the indexes reach from them, transitively.
    /// Everything else that was valid at the revision before is valid at the new one, since
    /// nothing it read changed.
    pub(crate) fn change_roots(&mut self, keys: &BTreeSet<EnvKey>) -> BTreeSet<Arc<str>> {
        let previous = self.revision;
        self.revision = self.revision.saturating_add(1);
        for key in keys {
            self.roots.insert(*key, self.revision);
        }
        let mut stale_nodes = BTreeSet::new();
        let mut stale_variants: BTreeSet<VariantId> = BTreeSet::new();
        let mut queue: Vec<User> = keys
            .iter()
            .filter_map(|key| self.readers.get(key))
            .flatten()
            .cloned()
            .collect();
        while let Some(user) = queue.pop() {
            match user {
                User::Node(key) => {
                    stale_nodes.insert(key);
                }
                User::Variant(id) => {
                    if stale_variants.insert(id) {
                        queue.extend(self.users.get(&id).into_iter().flatten().cloned());
                    }
                }
            }
        }
        let revision = self.revision;
        for (id, variant) in &mut self.variants {
            if variant.verified == previous && !stale_variants.contains(id) {
                variant.verified = revision;
            }
        }
        stale_nodes
    }

    /// Records that `user` holds `inputs`, in the reverse indexes.
    pub(crate) fn link(&mut self, user: &User, inputs: &[Input]) {
        for input in inputs {
            for edge in edges_of(input) {
                self.users.entry(edge).or_default().insert(user.clone());
            }
            for key in keys_of(input) {
                self.readers.entry(key).or_default().insert(user.clone());
            }
        }
    }

    /// Takes `user`'s `inputs` out of the reverse indexes.
    pub(crate) fn unlink(&mut self, user: &User, inputs: &[Input]) {
        for input in inputs {
            for edge in edges_of(input) {
                if let Some(users) = self.users.get_mut(&edge) {
                    users.remove(user);
                    if users.is_empty() {
                        self.users.remove(&edge);
                        self.orphans.insert(edge);
                    }
                }
            }
            for key in keys_of(input) {
                if let Some(readers) = self.readers.get_mut(&key) {
                    readers.remove(user);
                    if readers.is_empty() {
                        self.readers.remove(&key);
                    }
                }
            }
        }
    }

    /// Drops every variant no node's record and no other variant's record holds an edge to,
    /// and in turn what only those held. Done when a tree operation ends, starting from the
    /// variants kept or left without a user since the last sweep, so it costs what changed.
    pub(crate) fn sweep(&mut self) {
        while let Some(id) = self.orphans.pop_first() {
            if self.users.contains_key(&id) {
                continue;
            }
            let Some(variant) = self.variants.remove(&id) else {
                continue;
            };
            if let Some(ids) = self.by_derived.get_mut(&variant.derived) {
                ids.retain(|kept| *kept != id);
                if ids.is_empty() {
                    self.by_derived.remove(&variant.derived);
                }
            }
            // What only this variant used becomes an orphan in turn.
            self.unlink(&User::Variant(id), &variant.inputs);
        }
    }

    /// Takes on what a session found and kept.
    pub(crate) fn commit(&mut self, changes: Committed) {
        if let Some(store) = changes.store {
            *self = store;
        }
        for (id, verified) in changes.verified {
            if let Some(variant) = self.variants.get_mut(&id) {
                variant.verified = verified;
            }
        }
    }

    /// The values variant `id` read at any depth, through its edges included.
    fn leaves(&self, id: VariantId, into: &mut BTreeSet<EnvKey>, seen: &mut BTreeSet<VariantId>) {
        if !seen.insert(id) {
            return;
        }
        let Some(variant) = self.variants.get(&id) else {
            return;
        };
        for input in variant.inputs.iter() {
            into.extend(keys_of(input));
            for edge in edges_of(input) {
                self.leaves(edge, into, seen);
            }
        }
    }
}

/// The variants an input holds an edge to.
fn edges_of(input: &Input) -> Vec<VariantId> {
    match input {
        Input::Root { edges, .. } => edges.to_vec(),
        Input::Edge(id) => vec![*id],
        Input::Override { .. } | Input::Beneath { .. } => Vec::new(),
    }
}

/// The environment values an input is indexed under.
fn keys_of(input: &Input) -> Vec<EnvKey> {
    match input {
        Input::Root { key, .. } | Input::Beneath { key, .. } => vec![*key],
        Input::Override { key, lookup, .. } => {
            let mut keys = lookup.to_vec();
            if !keys.contains(key) {
                keys.push(*key);
            }
            keys
        }
        Input::Edge(_) => Vec::new(),
    }
}

/// Whether a variant holds at a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Validity {
    Valid,
    /// Something it read changed: evaluating it again at the place gives its replacement.
    Stale,
    /// It was kept for another place: a read is supplied otherwise here, or an override gives
    /// another value.
    Other,
}

/// One tree operation's use of the store at one place: the tree's store, read in place and copied
/// only when the operation keeps a result, the revisions at which variants were found valid, and
/// what has been found of each variant there. What it found is committed when the operation
/// succeeds.
#[derive(Debug)]
pub(crate) struct Session<'s> {
    base: &'s Store,
    /// The working copy, made at the first change other than a variant found valid.
    own: RefCell<Option<Store>>,
    /// The revision at which each variant was found valid in the session.
    verified: RefCell<HashMap<VariantId, u64>>,
    memo: RefCell<HashMap<VariantId, Validity>>,
}

/// What a session changes in the tree's store.
#[derive(Debug)]
pub(crate) struct Committed {
    store: Option<Store>,
    verified: HashMap<VariantId, u64>,
}

impl<'s> Session<'s> {
    pub(crate) fn new(base: &'s Store) -> Self {
        Self {
            base,
            own: RefCell::new(None),
            verified: RefCell::new(HashMap::new()),
            memo: RefCell::new(HashMap::new()),
        }
    }

    /// What the session changed, for [`Store::commit`].
    pub(crate) fn into_changes(self) -> Committed {
        Committed {
            store: self.own.into_inner(),
            verified: self.verified.into_inner(),
        }
    }

    /// `look` over the store as the session stands.
    fn read<T>(&self, look: impl FnOnce(&Store) -> T) -> T {
        match &*self.own.borrow() {
            Some(store) => look(store),
            None => look(self.base),
        }
    }

    /// `change` over the session's working copy of the store, made now if there is none.
    fn write<T>(&self, change: impl FnOnce(&mut Store) -> T) -> T {
        let mut own = self.own.borrow_mut();
        change(own.get_or_insert_with(|| self.base.clone()))
    }

    /// The revision at which variant `variant` was last found valid.
    fn verified_of(&self, id: VariantId, variant: &Variant) -> u64 {
        self.verified
            .borrow()
            .get(&id)
            .copied()
            .unwrap_or(variant.verified)
    }
}

/// Whether two values are the same as a host would see them written: the same number kind,
/// `-0.0` distinct from `0.0`, a NaN the same as nothing, and, when `origins`, equal origins.
/// Charged as `==` charges a comparison.
pub(crate) fn same_written(
    left: &Value,
    right: &Value,
    origins: bool,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<bool> {
    meter.charge(1)?;
    if matches!(left, Value::Seq(_) | Value::Record(_)) && !stack.within() {
        return stack.value_too_deep();
    }
    Ok(match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => {
            !left.is_nan() && left.to_bits() == right.to_bits()
        }
        (Value::Str(left), Value::Str(right)) => left == right,
        (Value::Case(left), Value::Case(right)) => {
            left.union == right.union && left.case == right.case
        }
        (Value::Function(left), Value::Function(right)) => left == right,
        (Value::Handler(left), Value::Handler(right)) => {
            Arc::ptr_eq(left, right) || handlers_equal(left, right, meter, stack)?
        }
        (Value::Seq(left), Value::Seq(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            for (left, right) in left.iter().zip(right.iter()) {
                if !same_written(left, right, origins, meter, stack)? {
                    return Ok(false);
                }
            }
            true
        }
        (Value::Record(left), Value::Record(right)) => {
            if left.type_name != right.type_name || left.fields.len() != right.fields.len() {
                return Ok(false);
            }
            if origins {
                let same_origin = match (&left.origin, &right.origin) {
                    (None, None) => true,
                    (Some(left), Some(right)) => {
                        Arc::ptr_eq(&left.module, &right.module) && left.node == right.node
                    }
                    _ => false,
                };
                if !same_origin {
                    return Ok(false);
                }
            }
            for ((left_name, left), (right_name, right)) in
                left.fields.iter().zip(right.fields.iter())
            {
                if left_name != right_name || !same_written(left, right, origins, meter, stack)? {
                    return Ok(false);
                }
            }
            true
        }
        _ => false,
    })
}

/// Whether a failure is a limit, which a check passes on rather than taking for a change.
fn is_limit(error: &crate::error::NxIrRuntimeError) -> bool {
    error.code() == RESOURCE_LIMIT
}

/// The context a charge of the store's own work names.
fn store_cx() -> Cx {
    Cx {
        module: 0,
        declaration: 0,
        depth: 0,
    }
}

impl<'p> Machine<'p> {
    /// Whether a layer of the place, one the host gave, sets `key` wholly or at a path.
    fn place_sets(&self, key: EnvKey) -> bool {
        let state = self.env.borrow();
        state
            .layers
            .get(..state.base)
            .unwrap_or_default()
            .iter()
            .any(|layer| matches!(layer, Layer::Set { key: set, .. } if *set == key))
    }

    /// Whether the machine stands at its place with nothing under way: no layer of its own on
    /// the stack and nothing being evaluated, so a lookup made now is the place's.
    fn at_place(&self) -> bool {
        let state = self.env.borrow();
        state.layers.len() == state.base && state.tracks.is_empty() && state.evaluating.is_empty()
    }

    /// The kept variant of `derived` this use may take, entered into the call's own reuse, or
    /// `None` when the tree keeps none valid here or the machine keeps nothing.
    pub(crate) fn take_kept(&self, derived: Derived, start: usize) -> Result<Option<Value>> {
        let Some(session) = self.session else {
            return Ok(None);
        };
        if matches!(derived, Derived::Formula(_)) {
            return Ok(None);
        }
        let ids = session.read(|store| store.by_derived.get(&derived).cloned().unwrap_or_default());
        for id in ids {
            let mut leaves = BTreeSet::new();
            session.read(|store| store.leaves(id, &mut leaves, &mut BTreeSet::new()));
            if self.pushed_touches(&leaves, start) {
                continue;
            }
            if self.validity_here(id)? != Validity::Valid {
                continue;
            }
            let Some(value) =
                session.read(|store| store.variants.get(&id).map(|variant| variant.value.clone()))
            else {
                continue;
            };
            let mut state = self.env.borrow_mut();
            state.enter_kept(derived, start, value, id, leaves);
            return Ok(state.reuse(&derived, start));
        }
        Ok(None)
    }

    /// Whether a layer the call pushed itself, below `start`, sets or caps a value in `keys`.
    fn pushed_touches(&self, keys: &BTreeSet<EnvKey>, start: usize) -> bool {
        let state = self.env.borrow();
        state
            .layers
            .get(state.base..start)
            .unwrap_or_default()
            .iter()
            .any(|layer| match layer {
                Layer::Set { key, .. } | Layer::Cap { key, .. } => keys.contains(key),
            })
    }

    /// Whether variant `id` holds at this machine's place: found once per session, at the place
    /// itself, in a machine of its own when this one is in the middle of something.
    fn validity_here(&self, id: VariantId) -> Result<Validity> {
        let Some(session) = self.session else {
            return Ok(Validity::Other);
        };
        if let Some(known) = session.memo.borrow().get(&id) {
            return Ok(*known);
        }
        if self.at_place() {
            return self.validate(id);
        }
        let fork = self.fork()?;
        let result = fork.validate(id);
        self.absorb(&fork);
        result
    }

    /// Validates variant `id` here, at the place: its inputs in the order they were read, each
    /// classified at the place first, stopping at the first that does not hold.
    fn validate(&self, id: VariantId) -> Result<Validity> {
        let Some(session) = self.session else {
            return Ok(Validity::Other);
        };
        if let Some(known) = session.memo.borrow().get(&id) {
            return Ok(*known);
        }
        let found = session.read(|store| {
            store.variants.get(&id).map(|variant| {
                (
                    Arc::clone(&variant.inputs),
                    session.verified_of(id, variant),
                    variant.origins,
                )
            })
        });
        let Some((inputs, verified, origins)) = found else {
            return Ok(Validity::Other);
        };
        let mut validity = if origins == self.tracks_origins() {
            Validity::Valid
        } else {
            Validity::Other
        };
        if validity == Validity::Valid {
            for (position, input) in inputs.iter().enumerate() {
                validity = self.input_holds(input, verified)?;
                if validity == Validity::Stale {
                    // Replaceable here only if it is this place's: the reads after the one that
                    // changed are classified too, which costs nothing.
                    let rest = inputs.get(position.saturating_add(1)..).unwrap_or_default();
                    if !rest.iter().all(|input| self.classified_as_recorded(input)) {
                        validity = Validity::Other;
                    }
                }
                if validity != Validity::Valid {
                    break;
                }
            }
        }
        if validity == Validity::Valid {
            let revision = session.read(|store| store.revision);
            session.verified.borrow_mut().insert(id, revision);
        }
        session.memo.borrow_mut().insert(id, validity);
        Ok(validity)
    }

    /// Whether a node's record of its last render, found valid at revision `verified`, holds at
    /// this machine's place.
    pub(crate) fn record_holds(&self, record: &[Input], verified: u64) -> Result<bool> {
        for input in record {
            if self.input_holds(input, verified)? != Validity::Valid {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether a read is classified at this machine's place as it was recorded: a root read where
    /// no layer of the place sets the value, and an override-supplied read where one does. An
    /// edge and a read beneath the render's own overrides are classified by what they lead to.
    fn classified_as_recorded(&self, input: &Input) -> bool {
        match input {
            Input::Root { key, .. } => !self.place_sets(*key),
            Input::Override { key, .. } => self.place_sets(*key),
            Input::Edge(_) | Input::Beneath { .. } => true,
        }
    }

    /// Whether one input of a record found valid at revision `verified` holds here.
    fn input_holds(&self, input: &Input, verified: u64) -> Result<Validity> {
        let Some(session) = self.session else {
            return Ok(Validity::Other);
        };
        match input {
            Input::Root { key, edges } => {
                if self.place_sets(*key) {
                    return Ok(Validity::Other);
                }
                if session.read(|store| store.root_revision(*key)) > verified {
                    return Ok(Validity::Stale);
                }
                for edge in edges.iter() {
                    let holds = self.edge_holds(*edge, verified)?;
                    if holds != Validity::Valid {
                        return Ok(holds);
                    }
                }
                Ok(Validity::Valid)
            }
            Input::Edge(edge) => self.edge_holds(*edge, verified),
            Input::Override { key, value, .. } => {
                if !self.place_sets(*key) {
                    return Ok(Validity::Other);
                }
                Ok(if self.lookup_equals(*key, value)? {
                    Validity::Valid
                } else {
                    Validity::Other
                })
            }
            Input::Beneath { key, value } => Ok(match value {
                Some(value) if self.lookup_equals(*key, value)? => Validity::Valid,
                _ => Validity::Stale,
            }),
        }
    }

    /// Whether what the lookup of `key` gives at the place is the same as `recorded` as a host
    /// would see it written, origins included when the call collects them, charged as the skip
    /// check's reads and comparisons are. A lookup that fails is not the same.
    fn lookup_equals(&self, key: EnvKey, recorded: &Value) -> Result<bool> {
        let cx = Cx {
            module: key.0,
            declaration: key.1,
            depth: 0,
        };
        match self.environment_value(cx, None, key) {
            Ok(current) => same_written(
                &current,
                recorded,
                self.tracks_origins(),
                &self.meter(cx, None),
                &self.stack,
            ),
            Err(error) if is_limit(&error) => Err(error),
            Err(_) => Ok(false),
        }
    }

    /// Whether the edge to variant `edge` of a record found valid at revision `verified` holds:
    /// the variant validated first, evaluated again here with the cutoff when it is stale, and
    /// then changed no later than the record was found valid.
    fn edge_holds(&self, edge: VariantId, verified: u64) -> Result<Validity> {
        let Some(session) = self.session else {
            return Ok(Validity::Other);
        };
        let changed = |session: &Session<'_>| {
            session.read(|store| store.variants.get(&edge).map(|variant| variant.changed))
        };
        match self.validate(edge)? {
            Validity::Valid => Ok(match changed(session) {
                Some(changed) if changed <= verified => Validity::Valid,
                _ => Validity::Stale,
            }),
            Validity::Other => Ok(Validity::Other),
            Validity::Stale => {
                let Some(derived) =
                    session.read(|store| store.variants.get(&edge).map(|variant| variant.derived))
                else {
                    return Ok(Validity::Other);
                };
                match self.derived(derived, || self.evaluate_derived(derived, 0)) {
                    Ok(_) => {}
                    Err(error) if is_limit(&error) => return Err(error),
                    Err(_) => return Ok(Validity::Stale),
                }
                if self.env.borrow().resolved(&derived) != Some(edge) {
                    // Evaluated here, it is another variant than the one the edge names.
                    return Ok(Validity::Other);
                }
                Ok(match changed(session) {
                    Some(changed) if changed <= verified => Validity::Valid,
                    _ => Validity::Stale,
                })
            }
        }
    }

    /// Keeps the result of `derived` just evaluated at a use that began at `start`, with the
    /// direct inputs it read, when the machine keeps results and nothing the call pushed itself
    /// supplied what it read: in place of a stale variant for the same place, with the cutoff,
    /// or as a new variant. Returns the variant.
    pub(crate) fn keep(
        &self,
        derived: Derived,
        value: &Value,
        inputs: Arc<[Input]>,
    ) -> Result<Option<VariantId>> {
        let Some(session) = self.session else {
            return Ok(None);
        };
        if matches!(derived, Derived::Formula(_)) {
            return Ok(None);
        }
        let origins = self.tracks_origins();
        let target = session.read(|store| {
            let memo = session.memo.borrow();
            store.by_derived.get(&derived).and_then(|ids| {
                ids.iter()
                    .find(|id| memo.get(id) == Some(&Validity::Stale))
                    .copied()
            })
        });
        let id = match target {
            Some(id) => {
                let old = session
                    .read(|store| store.variants.get(&id).map(|variant| variant.value.clone()));
                let same = match &old {
                    Some(old) => same_written(
                        old,
                        value,
                        origins,
                        &self.meter(store_cx(), None),
                        &self.stack,
                    )?,
                    None => false,
                };
                session.write(|store| {
                    let revision = store.revision;
                    let user = User::Variant(id);
                    if let Some(previous) = store
                        .variants
                        .get(&id)
                        .map(|variant| Arc::clone(&variant.inputs))
                    {
                        store.unlink(&user, &previous);
                    }
                    store.link(&user, &inputs);
                    if let Some(variant) = store.variants.get_mut(&id) {
                        if !same {
                            variant.value = value.clone();
                            variant.changed = revision;
                        }
                        variant.inputs = inputs;
                        variant.verified = revision;
                    }
                });
                session.verified.borrow_mut().remove(&id);
                id
            }
            None => session.write(|store| {
                let id = store.next;
                store.next = store.next.saturating_add(1);
                let revision = store.revision;
                store.link(&User::Variant(id), &inputs);
                store.variants.insert(
                    id,
                    Variant {
                        derived,
                        value: value.clone(),
                        changed: revision,
                        verified: revision,
                        inputs,
                        origins,
                    },
                );
                store.by_derived.entry(derived).or_default().push(id);
                // Kept for a use whose record has not been linked yet.
                store.orphans.insert(id);
                id
            }),
        };
        session.memo.borrow_mut().insert(id, Validity::Valid);
        Ok(Some(id))
    }
}

//! Reusing derived results within one call.
//!
//! <para>A *derived result* is a top-level value's value checked against its type, an environment
//! value's default checked against its type, a targeted entry-module provide's value, an
//! entry-module bundle provide's bundle for the value being read, and a captured formula's value.
//! None of them reads a local slot, so the only input that can differ between two uses in one call
//! is the environment. The call keeps the last result of each, with the environment values its
//! evaluation read at any depth, each with its *token*: the identity of the topmost layer below
//! where the evaluation began that sets or caps the value, or none. A later use reuses the result
//! while every token is still the same, which costs nothing.</para>
//!
//! <para>An evaluation begins at the stack as it stands when the result is asked for, before its
//! own cap is pushed. The layers above that position are the ones the evaluation itself pushes,
//! which its identity and the layers below fix. Because the stack only grows and shrinks at the
//! top, a layer still present has the same layers beneath it as when it was pushed, so two reads
//! with the same token find the value from the same sources.</para>
//!
//! <para>A reused result's reads are reads of every derived result being evaluated around the use,
//! with their tokens computed again over the layers below where each began.</para>
//!
//! <para>In a call the instance tree makes, the call also *tracks* what it reads, for the tree's
//! records ([`crate::kept`]): each derived evaluation, each environment read and the render
//! itself keeps its direct inputs in the order it read them, the values it read at any depth,
//! and the lowest layer the call pushed itself that a read under it met. A reused result adds to
//! what encloses its use exactly what evaluating it added, so the tree's records come out alike
//! whether a result was evaluated or reused.</para>

use crate::environment::{EnvKey, EnvState, Layer};
use crate::error::Result;
use crate::eval::Machine;
use crate::kept::{Input, VariantId};
use crate::value::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

/// What identifies a derived result within one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Derived {
    /// A top-level value declaration: its module and declaration index.
    Value(u32, u32),
    /// An environment value's default.
    Default(EnvKey),
    /// A targeted entry-module provide, by index.
    Provide(usize),
    /// A bundle entry-module provide, by index, for the value being read.
    Bundle(usize, EnvKey),
    /// A captured formula, by the identity of the layer that holds it.
    Formula(u64),
}

/// What one derived evaluation has read so far, or read in all once kept.
#[derive(Debug, Default, Clone)]
pub(crate) struct Reads {
    /// Each value read, once, with its token over the layers below where the evaluation began.
    tokens: Vec<(EnvKey, Option<u64>)>,
}

impl Reads {
    fn note(&mut self, key: EnvKey, token: Option<u64>) {
        if self.tokens.iter().all(|(known, _)| *known != key) {
            self.tokens.push((key, token));
        }
    }
}

/// A derived result being evaluated: where it began and what it has read.
#[derive(Debug)]
pub(crate) struct Recording {
    start: usize,
    reads: Reads,
}

/// The last result of a derived result, with what its evaluation read.
#[derive(Debug)]
pub(crate) struct Kept {
    value: Value,
    reads: Reads,
    /// What the evaluation added to what tracked its use, for a call that tracks.
    tracked: Tracked,
}

/// What a derived result's use adds to what encloses it, in a call that tracks.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tracked {
    /// The tree's variant of the result, when it may be kept.
    variant: Option<VariantId>,
    /// The lowest layer the call pushed, below where the evaluation began, that a read in it
    /// met: such a result is not kept.
    passed: Option<usize>,
    /// The values the evaluation read at any depth, but for those a layer the call pushed
    /// supplied wholly. `None` in a call that does not track.
    leaves: Option<Arc<BTreeSet<EnvKey>>>,
}

/// What a track is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackKind {
    /// The render itself: its record is the node's.
    Render,
    /// A derived evaluation that began at this position of the stack.
    Derived(usize),
    /// One environment read.
    Read,
}

/// What one tracked part of a call has read.
#[derive(Debug)]
pub(crate) struct Track {
    kind: TrackKind,
    /// The direct inputs, in the order they were read: a render's and a derived evaluation's.
    inputs: Vec<Input>,
    /// The variants a read used on its way to the value.
    edges: Vec<VariantId>,
    leaves: BTreeSet<EnvKey>,
    /// The lowest position of a layer the call pushed that a read under the track met, outside
    /// a derived evaluation that began below it.
    low: Option<usize>,
}

impl Track {
    pub(crate) fn new(kind: TrackKind) -> Self {
        Self {
            kind,
            inputs: Vec::new(),
            edges: Vec::new(),
            leaves: BTreeSet::new(),
            low: None,
        }
    }

    /// The direct inputs the track recorded, in the order they were read.
    pub(crate) fn into_inputs(self) -> Vec<Input> {
        self.inputs
    }

    fn input(&mut self, input: Input) {
        let known = self.inputs.iter().any(|known| match (known, &input) {
            (Input::Edge(known), Input::Edge(new)) => known == new,
            (
                Input::Root { key: known, .. }
                | Input::Override { key: known, .. }
                | Input::Beneath { key: known, .. },
                Input::Root { key: new, .. }
                | Input::Override { key: new, .. }
                | Input::Beneath { key: new, .. },
            ) => known == new,
            _ => false,
        });
        if !known {
            self.inputs.push(input);
        }
    }

    fn edge(&mut self, variant: VariantId) {
        match self.kind {
            TrackKind::Read => {
                if !self.edges.contains(&variant) {
                    self.edges.push(variant);
                }
            }
            TrackKind::Render | TrackKind::Derived(_) => self.input(Input::Edge(variant)),
        }
    }
}

/// How one environment read found its value, for its track.
#[derive(Debug, Default)]
pub(crate) struct ReadInfo {
    /// The position of a layer the call pushed that set the value wholly, when one did.
    pub own_whole: Option<usize>,
    /// The lowest position of a layer the call pushed that the read met.
    pub low: Option<usize>,
    /// Whether a layer of the place, one the host gave, sets the value wholly or at a path.
    pub host: bool,
    /// Whether the read met a cap of the value: it is a read inside one of its own sources.
    pub capped: bool,
    /// The value found beneath the overrides the call pushed itself.
    pub beneath: Option<Value>,
}

fn lowest(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    }
}

impl EnvState {
    /// Pushes a layer, giving it the next identity of the call.
    pub(crate) fn push(&mut self, layer: Layer) {
        self.layers.push(layer);
        self.ids.push(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
    }

    /// Drops the layers above `depth`.
    pub(crate) fn truncate(&mut self, depth: usize) {
        self.layers.truncate(depth);
        self.ids.truncate(depth);
    }

    /// The identity of the layer at `position`.
    pub(crate) fn id(&self, position: usize) -> Option<u64> {
        self.ids.get(position).copied()
    }

    /// The token of `key` over the layers below `start`: the identity of the topmost of them that
    /// sets or caps `key`, or none.
    pub(crate) fn token(&self, key: EnvKey, start: usize) -> Option<u64> {
        let below = self.layers.get(..start).unwrap_or(&self.layers);
        below
            .iter()
            .rposition(|layer| match layer {
                Layer::Set { key: set, .. } => *set == key,
                Layer::Cap { key: capped, .. } => *capped == key,
            })
            .and_then(|position| self.id(position))
    }

    /// Notes a read of `key` in every derived evaluation under way.
    pub(crate) fn note_read(&mut self, key: EnvKey) {
        if self.evaluating.is_empty() {
            return;
        }
        let tokens: Vec<Option<u64>> = self
            .evaluating
            .iter()
            .map(|recording| self.token(key, recording.start))
            .collect();
        for (recording, token) in self.evaluating.iter_mut().zip(tokens) {
            recording.reads.note(key, token);
        }
    }

    /// Adds to the innermost track what a derived result's use adds: its variant as an edge,
    /// the lowest layer the call pushed that it met, and the values it read. A result used
    /// directly by a render that may not be kept gives the render a read beneath its own
    /// overrides of each value it read.
    fn add_use(&mut self, tracked: &Tracked) {
        let Some(top) = self.tracks.last_mut() else {
            return;
        };
        let leaves = tracked.leaves.as_deref();
        top.leaves.extend(leaves.into_iter().flatten().copied());
        top.low = lowest(top.low, tracked.passed);
        match tracked.variant {
            Some(variant) => top.edge(variant),
            None if top.kind == TrackKind::Render && tracked.passed.is_some() => {
                for key in leaves.into_iter().flatten() {
                    top.input(Input::Beneath {
                        key: *key,
                        value: None,
                    });
                }
            }
            None => {}
        }
    }

    /// Ends the track of a read of `key` that found its value as `info` says, and adds the read
    /// to the track around it.
    pub(crate) fn end_read(&mut self, key: EnvKey, info: ReadInfo) {
        let Some(track) = self.tracks.pop() else {
            return;
        };
        let Some(parent) = self.tracks.last_mut() else {
            return;
        };
        let mut leaves = track.leaves;
        if info.own_whole.is_none() {
            leaves.insert(key);
        }
        parent.leaves.extend(leaves.iter().copied());
        // What the read met on its way, nested evaluations included.
        let nested = track.low;
        parent.low = lowest(parent.low, lowest(nested, info.low));
        if parent.kind == TrackKind::Read {
            for edge in track.edges {
                parent.edge(edge);
            }
            return;
        }
        if info.own_whole.is_some() {
            return;
        }
        if info.capped {
            for edge in track.edges {
                parent.edge(edge);
            }
            return;
        }
        if parent.kind == TrackKind::Render && nested.is_some() {
            // A source was evaluated under the render's own overrides of other values: what it
            // found may differ beneath them.
            parent.input(Input::Beneath { key, value: None });
            return;
        }
        if info.host {
            let Some(value) = info.beneath else {
                return;
            };
            parent.input(Input::Override {
                key,
                value,
                lookup: leaves.into_iter().collect(),
            });
            return;
        }
        parent.input(Input::Root {
            key,
            edges: Arc::from(track.edges),
        });
    }

    /// Enters variant `variant` of the tree's kept results into the call's reuse as the result
    /// of `derived`, as if it had been evaluated at the call's start: its reads with the tokens
    /// they have at a use that begins at `start`.
    pub(crate) fn enter_kept(
        &mut self,
        derived: Derived,
        start: usize,
        value: Value,
        variant: VariantId,
        leaves: BTreeSet<EnvKey>,
    ) {
        let tokens = leaves
            .iter()
            .map(|key| (*key, self.token(*key, start)))
            .collect();
        self.derived.insert(
            derived,
            Kept {
                value,
                reads: Reads { tokens },
                tracked: Tracked {
                    variant: Some(variant),
                    passed: None,
                    leaves: Some(Arc::new(leaves)),
                },
            },
        );
    }

    /// The tree's variant the call's last result of `derived` is.
    pub(crate) fn resolved(&self, derived: &Derived) -> Option<VariantId> {
        self.derived
            .get(derived)
            .and_then(|kept| kept.tracked.variant)
    }

    /// The kept result of `derived` when every value it read has the same token at a use that
    /// begins at `start`. Its reads are then added to every derived evaluation under way and to
    /// what tracks the use, as evaluating it would have added them.
    pub(crate) fn reuse(&mut self, derived: &Derived, start: usize) -> Option<Value> {
        let kept = self.derived.get(derived)?;
        if kept
            .reads
            .tokens
            .iter()
            .any(|(key, token)| self.token(*key, start) != *token)
        {
            return None;
        }
        let value = kept.value.clone();
        let reads = kept.reads.clone();
        let tracked = kept.tracked.clone();
        for (key, _) in &reads.tokens {
            self.note_read(*key);
        }
        self.add_use(&tracked);
        Some(value)
    }
}

impl Machine<'_> {
    /// `derived`'s kept result when it can be reused here, or else `work` evaluated, kept and
    /// returned. A failure ends the call, so it is never kept.
    ///
    /// <para>In a call the instance tree makes, a result the call has not kept may be taken
    /// from the tree's, and one the call evaluates is offered to the tree.</para>
    pub(crate) fn derived(
        &self,
        derived: Derived,
        work: impl FnOnce() -> Result<Value>,
    ) -> Result<Value> {
        let start = {
            let mut state = self.env.borrow_mut();
            let start = state.layers.len();
            if let Some(value) = state.reuse(&derived, start) {
                return Ok(value);
            }
            start
        };
        if let Some(value) = self.take_kept(derived, start)? {
            return Ok(value);
        }
        let tracks = {
            let mut state = self.env.borrow_mut();
            state.evaluating.push(Recording {
                start,
                reads: Reads::default(),
            });
            let tracks = state.tracks.len();
            if self.session.is_some() {
                state.tracks.push(Track::new(TrackKind::Derived(start)));
            }
            tracks
        };
        let result = work();
        let (recording, track) = {
            let mut state = self.env.borrow_mut();
            let recording = state.evaluating.pop();
            let track = if self.session.is_some() {
                state.tracks.truncate(tracks.saturating_add(1));
                let track = state.tracks.pop();
                state.tracks.truncate(tracks);
                track
            } else {
                None
            };
            (recording, track)
        };
        let value = result?;
        let tracked = match track {
            Some(track) => {
                // A layer at or above where the evaluation began is its own.
                let passed = track.low.filter(|position| *position < start);
                let variant = match passed {
                    None => self.keep(derived, &value, track.inputs)?,
                    Some(_) => None,
                };
                Tracked {
                    variant,
                    passed,
                    leaves: Some(Arc::new(track.leaves)),
                }
            }
            None => Tracked::default(),
        };
        let mut state = self.env.borrow_mut();
        state.add_use(&tracked);
        if let Some(recording) = recording.filter(|recording| recording.start == start) {
            state.derived.insert(
                derived,
                Kept {
                    value: value.clone(),
                    reads: recording.reads,
                    tracked,
                },
            );
        }
        Ok(value)
    }
}

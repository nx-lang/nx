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
//! with their tokens computed again over the layers below where each began, and of the call's own
//! read record when the instance tree keeps one, so the tree's skip check sees the same reads
//! whether a result was evaluated or reused.</para>

use crate::environment::{EnvKey, EnvState, Layer};
use crate::error::Result;
use crate::eval::Machine;
use crate::value::Value;

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

/// A read the instance tree's read list would record, as a derived evaluation made it.
#[derive(Debug, Clone)]
pub(crate) struct TreeRead {
    key: EnvKey,
    /// The value found beneath the overrides the call pushed itself.
    value: Value,
    /// Whether the read evaluated a default, a provide or a formula on its way.
    evaluated: bool,
    /// Whether an override the call pushed itself was in effect above where the derived
    /// evaluation began.
    inside: bool,
}

/// What one derived evaluation has read so far, or read in all once kept.
#[derive(Debug, Default, Clone)]
pub(crate) struct Reads {
    /// Each value read, once, with its token over the layers below where the evaluation began.
    tokens: Vec<(EnvKey, Option<u64>)>,
    /// The reads the instance tree's read list would record, first of each value only.
    tree: Vec<TreeRead>,
}

impl Reads {
    fn note(&mut self, key: EnvKey, token: Option<u64>) {
        if self.tokens.iter().all(|(known, _)| *known != key) {
            self.tokens.push((key, token));
        }
    }

    fn note_tree(&mut self, read: TreeRead) {
        if self.tree.iter().all(|known| known.key != read.key) {
            self.tree.push(read);
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
    fn token(&self, key: EnvKey, start: usize) -> Option<u64> {
        let below = self.layers.get(..start).unwrap_or(&self.layers);
        below
            .iter()
            .rposition(|layer| match layer {
                Layer::Set { key: set, .. } => *set == key,
                Layer::Cap { key: capped, .. } => *capped == key,
            })
            .and_then(|position| self.id(position))
    }

    /// Whether an override the call pushed itself is among the layers from `from` to `to`.
    fn own_override_between(&self, from: usize, to: usize) -> bool {
        let from = from.max(self.base);
        self.layers
            .get(from..to)
            .unwrap_or_default()
            .iter()
            .any(|layer| matches!(layer, Layer::Set { .. }))
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

    /// Notes, in every derived evaluation under way, a read the instance tree's read list
    /// records: `value` found beneath the call's own overrides, after evaluating a source when
    /// `evaluated`.
    pub(crate) fn note_tree_read(&mut self, key: EnvKey, value: &Value, evaluated: bool) {
        if self.evaluating.is_empty() {
            return;
        }
        let top = self.layers.len();
        let inside: Vec<bool> = self
            .evaluating
            .iter()
            .map(|recording| self.own_override_between(recording.start, top))
            .collect();
        for (recording, inside) in self.evaluating.iter_mut().zip(inside) {
            recording.reads.note_tree(TreeRead {
                key,
                value: value.clone(),
                evaluated,
                inside,
            });
        }
    }

    /// The kept result of `derived` when every value it read has the same token at a use that
    /// begins at `start`. Its reads are then added to every derived evaluation under way and to
    /// the call's read record, as evaluating it would have added them.
    fn reuse(&mut self, derived: &Derived, start: usize) -> Option<Value> {
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
        for (key, _) in &reads.tokens {
            self.note_read(*key);
        }
        let below_start: Vec<bool> = self
            .evaluating
            .iter()
            .map(|recording| self.own_override_between(recording.start, start))
            .collect();
        let pushed = self.own_override_between(0, start);
        for read in &reads.tree {
            for (recording, below) in self.evaluating.iter_mut().zip(below_start.iter()) {
                recording.reads.note_tree(TreeRead {
                    inside: read.inside || *below,
                    ..read.clone()
                });
            }
            if let Some(record) = self.reads.as_mut() {
                let found =
                    (!(read.evaluated && (read.inside || pushed))).then(|| read.value.clone());
                record.push((read.key, found));
            }
        }
        Some(value)
    }
}

impl Machine<'_> {
    /// `derived`'s kept result when it can be reused here, or else `work` evaluated, kept and
    /// returned. A failure ends the call, so it is never kept.
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
            state.evaluating.push(Recording {
                start,
                reads: Reads::default(),
            });
            start
        };
        let result = work();
        let mut state = self.env.borrow_mut();
        let recording = state.evaluating.pop();
        let value = result?;
        if let Some(recording) = recording.filter(|recording| recording.start == start) {
            state.derived.insert(
                derived,
                Kept {
                    value: value.clone(),
                    reads: recording.reads,
                },
            );
        }
        Ok(value)
    }
}

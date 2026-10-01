//! The serialized form of a component instance.
//!
//! <para>An instance shares most of what it holds: every handler of a rendered list captures the
//! same list. The serialized form keeps that by writing each value once, in a table whose entries
//! name the values they hold by index, always an earlier one. The table is flat, so the form
//! nests no deeper for a value that does, and reading it back cannot recurse; the same order
//! makes a cycle impossible to write down.</para>
//!
//! <para>A handler is stored as the node it is and what it captured. Everything else a handler
//! knows is read from the node again when the instance is restored.</para>

use crate::component::InstanceData;
use crate::error::{fail, Result};
use crate::eval::{Cx, Machine};
use crate::module::{DeclarationKind, Field, Node};
use crate::value::{CaseValue, Fields, FunctionRef, Handler, Value};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// How deeply the values of a serialized instance may nest, a handler's capture counting as one
/// level. An instance holds what its parents' handlers captured, so it can nest deeper than one
/// value crossing the boundary may; this is what keeps a restored one within the native stack.
pub(crate) const MAX_STORED_DEPTH: u32 = 1024;

/// How many values one value of a serialized instance may hold, counting a value once for every
/// place it appears. The table writes a shared value once, so a small table can name a value
/// that is exponentially large written out, which is how dispatch returns state to the host.
pub(crate) const MAX_STORED_SIZE: u64 = 1 << 24;

/// How deeply an entry nests and how many values it holds written out.
#[derive(Clone, Copy)]
struct Extent {
    depth: u32,
    size: u64,
}

type StoredFields = Vec<(Arc<str>, u32)>;

#[derive(Serialize, Deserialize)]
pub(crate) struct Stored {
    pub program: Vec<(Arc<str>, u64)>,
    pub component: Arc<str>,
    generation: u64,
    /// Every value the instance holds. An entry names only entries before it.
    values: Vec<StoredValue>,
    props: StoredFields,
    handler_props: StoredFields,
    state: StoredFields,
    handlers: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
enum StoredValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    Seq(Vec<u32>),
    Record {
        type_name: Option<Arc<str>>,
        fields: StoredFields,
    },
    Case {
        union: Arc<str>,
        case: Arc<str>,
    },
    Function {
        module: Arc<str>,
        name: Arc<str>,
    },
    Handler {
        module: Arc<str>,
        declaration: Arc<str>,
        node: u32,
        captured: Vec<Option<u32>>,
    },
}

/// What makes two values one entry: the allocation they share, or the scalar they are.
#[derive(PartialEq, Eq, Hash)]
enum Key {
    Shared(usize),
    Bool(bool),
    Int(i64),
    Float(u64),
}

fn shared<T: ?Sized>(value: &Arc<T>) -> Key {
    Key::Shared(Arc::as_ptr(value).cast::<u8>() as usize)
}

#[derive(Default)]
struct Writer {
    values: Vec<StoredValue>,
    written: HashMap<Key, u32>,
}

impl Writer {
    fn value(&mut self, value: &Value) -> u32 {
        let key = match value {
            Value::Bool(value) => Key::Bool(*value),
            Value::Int(value) => Key::Int(*value),
            Value::Float(value) => Key::Float(value.to_bits()),
            Value::Str(value) => shared(value),
            Value::Seq(value) => shared(value),
            Value::Record(value) => shared(value),
            Value::Case(value) => shared(value),
            Value::Function(value) => shared(value),
            Value::Handler(value) => shared(value),
        };
        if let Some(index) = self.written.get(&key) {
            return *index;
        }
        let stored = match value {
            Value::Bool(value) => StoredValue::Bool(*value),
            Value::Int(value) => StoredValue::Int(*value),
            Value::Float(value) => StoredValue::Float(*value),
            Value::Str(value) => StoredValue::Str(Arc::clone(value)),
            Value::Seq(items) => {
                StoredValue::Seq(items.iter().map(|item| self.value(item)).collect())
            }
            Value::Record(record) => StoredValue::Record {
                type_name: record.type_name.clone(),
                fields: self.fields(&record.fields),
            },
            Value::Case(case) => StoredValue::Case {
                union: Arc::clone(&case.union),
                case: Arc::clone(&case.case),
            },
            Value::Function(function) => StoredValue::Function {
                module: Arc::clone(&function.module),
                name: Arc::clone(&function.name),
            },
            Value::Handler(handler) => StoredValue::Handler {
                module: Arc::clone(&handler.module),
                declaration: Arc::clone(&handler.declaration),
                node: handler.node,
                captured: handler
                    .captured
                    .iter()
                    .map(|slot| slot.as_ref().map(|value| self.value(value)))
                    .collect(),
            },
        };
        let index = self.values.len() as u32;
        self.values.push(stored);
        self.written.insert(key, index);
        index
    }

    fn fields(&mut self, fields: &[(Arc<str>, Value)]) -> StoredFields {
        fields
            .iter()
            .map(|(name, value)| (Arc::clone(name), self.value(value)))
            .collect()
    }

    fn handler(&mut self, handler: &Arc<Handler>) -> u32 {
        self.value(&Value::Handler(Arc::clone(handler)))
    }
}

impl Stored {
    pub(crate) fn of(instance: &InstanceData) -> Self {
        let mut writer = Writer::default();
        let props = writer.fields(&instance.props);
        let handler_props = instance
            .handler_props
            .iter()
            .map(|(name, handler)| (Arc::clone(name), writer.handler(handler)))
            .collect();
        let state = writer.fields(&instance.state);
        let handlers = instance
            .handlers
            .iter()
            .map(|handler| writer.handler(handler))
            .collect();
        Self {
            program: instance.program.to_vec(),
            component: Arc::clone(&instance.component),
            generation: instance.generation,
            values: writer.values,
            props,
            handler_props,
            state,
            handlers,
        }
    }

    /// Rebuilds the instance, checking every function and handler it names against the program.
    /// The props, the state and the handler properties are as they were stored; the caller
    /// validates them against the component.
    pub(crate) fn restore(self, machine: &Machine<'_>) -> Result<InstanceData> {
        let mut reader = Reader {
            machine,
            values: Vec::with_capacity(self.values.len()),
            reachable: HashMap::new(),
        };
        for stored in self.values {
            let entry = reader.entry(stored)?;
            reader.values.push(entry);
        }
        let handlers = |indices: &mut dyn Iterator<Item = u32>| {
            indices
                .map(|index| reader.handler(index))
                .collect::<Result<Vec<_>>>()
        };
        let handler_values = handlers(&mut self.handler_props.iter().map(|(_, index)| *index))?;
        Ok(InstanceData {
            program: Arc::from(self.program),
            component: self.component,
            props: reader.fields(&self.props)?,
            handler_props: self
                .handler_props
                .into_iter()
                .map(|(name, _)| name)
                .zip(handler_values)
                .collect(),
            state: reader.fields(&self.state)?,
            handlers: handlers(&mut self.handlers.iter().copied())?,
            generation: self.generation,
        })
    }
}

pub(crate) fn refuse<T>(what: impl fmt::Display) -> Result<T> {
    fail(
        "nx-ir-component",
        format!("The serialized instance does not belong to this program: {what}."),
    )
}

struct Reader<'m, 'p> {
    machine: &'m Machine<'p>,
    /// The entries read so far, each with its extent.
    values: Vec<(Value, Extent)>,
    /// The nodes each declaration reaches, by module and declaration, computed when first asked.
    reachable: HashMap<(u32, u32), HashSet<u32>>,
}

impl Reader<'_, '_> {
    /// An earlier entry, which is the only kind an entry may name.
    fn get(&self, index: u32) -> Result<&(Value, Extent)> {
        match self.values.get(index as usize) {
            Some(entry) => Ok(entry),
            None => refuse(format_args!("value {index} is named before it is written")),
        }
    }

    fn fields(&self, fields: &[(Arc<str>, u32)]) -> Result<Fields> {
        fields
            .iter()
            .map(|(name, index)| Ok((Arc::clone(name), self.get(*index)?.0.clone())))
            .collect()
    }

    fn handler(&self, index: u32) -> Result<Arc<Handler>> {
        match &self.get(index)?.0 {
            Value::Handler(handler) => Ok(Arc::clone(handler)),
            _ => refuse(format_args!("value {index} is not a handler")),
        }
    }

    fn entry(&mut self, stored: StoredValue) -> Result<(Value, Extent)> {
        let mut extent = Extent { depth: 0, size: 1 };
        let mut child = |reader: &Self, index: u32| -> Result<Value> {
            let (value, nested) = reader.get(index)?;
            extent.depth = extent.depth.max(nested.depth.saturating_add(1));
            extent.size = extent.size.saturating_add(nested.size);
            Ok(value.clone())
        };
        let value = match stored {
            StoredValue::Bool(value) => Value::Bool(value),
            StoredValue::Int(value) => Value::Int(value),
            StoredValue::Float(value) => Value::Float(value),
            StoredValue::Str(value) => Value::Str(value),
            StoredValue::Seq(items) => Value::seq(
                items
                    .into_iter()
                    .map(|index| child(self, index))
                    .collect::<Result<_>>()?,
            ),
            StoredValue::Record { type_name, fields } => Value::record(
                type_name,
                fields
                    .into_iter()
                    .map(|(name, index)| Ok((name, child(self, index)?)))
                    .collect::<Result<_>>()?,
            ),
            StoredValue::Case { union, case } => Value::Case(Arc::new(CaseValue { union, case })),
            StoredValue::Function { module, name } => {
                let function = FunctionRef { module, name };
                if let Err(error) = self.machine.resolve_function(&function, &"it") {
                    return refuse(error);
                }
                Value::Function(Arc::new(function))
            }
            StoredValue::Handler {
                module,
                declaration,
                node,
                captured,
            } => {
                let captured = captured
                    .into_iter()
                    .map(|slot| slot.map(|index| child(self, index)).transpose())
                    .collect::<Result<_>>()?;
                Value::Handler(Arc::new(self.read_handler(
                    &module,
                    &declaration,
                    node,
                    captured,
                )?))
            }
        };
        if extent.depth > MAX_STORED_DEPTH {
            return refuse(format_args!(
                "a value nests more than {MAX_STORED_DEPTH} levels deep"
            ));
        }
        if extent.size > MAX_STORED_SIZE {
            return refuse(format_args!(
                "a value holds more than {MAX_STORED_SIZE} values"
            ));
        }
        Ok((value, extent))
    }

    /// The handler that node `node` of `declaration` is, which it must be.
    fn read_handler(
        &mut self,
        module_identity: &str,
        declaration_name: &str,
        node: u32,
        captured: Vec<Option<Value>>,
    ) -> Result<Handler> {
        let program = self.machine.program;
        let Some(module) = program.by_identity.get(module_identity).copied() else {
            return refuse(format_args!("it names module '{module_identity}'"));
        };
        let Some(linked) = program.linked(module) else {
            return refuse("it names a module the program does not link");
        };
        let Some((declaration, _)) = linked.module.find(declaration_name) else {
            return refuse(format_args!("it names declaration '{declaration_name}'"));
        };
        let Ok(Node::ActionHandler(handler)) = linked.module.node(node) else {
            return refuse(format_args!("node {node} is not a handler"));
        };
        if !self.reaches(module, declaration, node) {
            return refuse(format_args!(
                "node {node} is not a handler of '{declaration_name}'"
            ));
        }
        let cx = Cx {
            module,
            declaration,
            depth: 0,
        };
        self.machine
            .handler(cx, node, handler, captured)
            .or_else(refuse)
    }

    /// Whether `declaration` of `module` reaches node `target` from its body or a default.
    fn reaches(&mut self, module: u32, declaration: u32, target: u32) -> bool {
        let program = self.machine.program;
        self.reachable
            .entry((module, declaration))
            .or_insert_with(|| {
                let mut seen = HashSet::new();
                let Some(linked) = program.linked(module) else {
                    return seen;
                };
                let mut pending: Vec<u32> = Vec::new();
                if let Some(declaration) = linked.module.declaration(declaration) {
                    let defaults = |fields: &[Field], pending: &mut Vec<u32>| {
                        pending.extend(fields.iter().filter_map(|field| field.default));
                    };
                    match &declaration.kind {
                        DeclarationKind::Function(function) => {
                            pending.push(function.body);
                            pending
                                .extend(function.params.iter().filter_map(|param| param.default));
                        }
                        DeclarationKind::Value { value, .. } => pending.push(*value),
                        DeclarationKind::Record(record) => defaults(&record.fields, &mut pending),
                        DeclarationKind::Component(component) => {
                            pending.extend(component.body);
                            defaults(&component.props, &mut pending);
                            defaults(&component.state, &mut pending);
                        }
                        DeclarationKind::Union(union) => {
                            union
                                .cases
                                .iter()
                                .for_each(|case| defaults(&case.fields, &mut pending));
                        }
                        DeclarationKind::TypeAlias => {}
                    }
                }
                while let Some(node) = pending.pop() {
                    if (node as usize) < linked.module.node_count() && seen.insert(node) {
                        if let Ok(decoded) = linked.module.node(node) {
                            decoded.children(&mut |child| pending.push(child));
                        }
                    }
                }
                seen
            })
            .contains(&target)
    }
}

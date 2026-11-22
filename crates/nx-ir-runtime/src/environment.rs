//! Environment values: the host's root values, the overrides a component descriptor carries in
//! `$env`, and the lookup that finds a value at a read.
//!
//! <para>An evaluation holds one stack of layers. The bottom layers are the overrides of the
//! environment the host gave, outermost first; an `envProvide` node pushes its overrides while its
//! body runs. A read walks the stack from the top to the first layer that sets the whole value,
//! then falls back to the host's root value, the entry module's provides, last first, and the
//! declaration's default. Overrides that set a field path on the way down are applied to that
//! base, outermost first.</para>

use crate::error::{fail, NxIrRuntimeError, Result};
use crate::eval::{Cx, Machine, Quoted};
use crate::module::{DeclarationKind, Override, Type};
use crate::normalize::Path;
use crate::program::ProgramData;
use crate::update;
use crate::value::{from_host, set_field, values_equal, Record, Value};
use nx_value::NxValue;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// The reserved field of a component descriptor that carries the overrides around it.
pub(crate) const ENV_FIELD: &str = "$env";
/// The `$type` of an `env.Update` bundle, and the field holding its entries.
pub(crate) const BUNDLE_TYPE: &str = "env.Update";
pub(crate) const BUNDLE_ENTRIES: &str = "$entries";
/// The prefix an environment declaration's name carries in an image.
const DECLARATION_PREFIX: &str = "env.";

/// One override a component descriptor carries in `$env`, in host form.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentEntry {
    /// The environment value: its module identity and name, `identity::name`.
    pub value: String,
    /// The field path the override sets; empty for the whole value.
    pub path: Vec<String>,
    /// The value it sets, in canonical form.
    pub to: NxValue,
}

/// The environment an evaluation runs in: the host's root values, and the overrides of every
/// component descriptor entered on the way to the component being evaluated, outermost first.
///
/// <para>It is immutable: [`Environment::enter`] makes a new one. A host makes the root with
/// [`Environment::root`] and, for each component it renders, enters the descriptor it renders the
/// component from, so the component's body sees the overrides written around it. An evaluation
/// given the default environment runs with no root values.</para>
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    roots: Arc<[(String, NxValue)]>,
    entries: Arc<[EnvironmentEntry]>,
}

impl Environment {
    /// An environment with these root values and no overrides. A value is named by its simple
    /// name when exactly one environment value in the program has it, and as `identity::name`
    /// otherwise; names and values are checked when an evaluation begins.
    pub fn root<N: Into<String>>(values: impl IntoIterator<Item = (N, NxValue)>) -> Self {
        Self {
            roots: values
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
            entries: Arc::from(Vec::new()),
        }
    }

    /// The environment a component rendered from `descriptor` runs in: this one, with the
    /// descriptor's `$env` overrides inside it. A descriptor without `$env` enters nothing. A
    /// `$env` that is not a list of `{value, path, to}` records fails with `nx-ir-boundary-type`.
    pub fn enter(&self, descriptor: &NxValue) -> Result<Self> {
        let NxValue::Record { properties, .. } = descriptor else {
            return Ok(self.clone());
        };
        let Some(carried) = properties.get(ENV_FIELD) else {
            return Ok(self.clone());
        };
        let entries = parse_entries(carried)?;
        if entries.is_empty() {
            return Ok(self.clone());
        }
        let mut all: Vec<EnvironmentEntry> = self.entries.to_vec();
        all.extend(entries);
        Ok(Self {
            roots: Arc::clone(&self.roots),
            entries: Arc::from(all),
        })
    }

    /// The root values, as given.
    pub fn roots(&self) -> &[(String, NxValue)] {
        &self.roots
    }

    /// The overrides entered, outermost first.
    pub fn entries(&self) -> &[EnvironmentEntry] {
        &self.entries
    }

    /// This environment with `roots` in place of its root values and the same overrides.
    pub fn with_roots(&self, roots: &Environment) -> Self {
        Self {
            roots: Arc::clone(&roots.roots),
            entries: Arc::clone(&self.entries),
        }
    }
}

fn malformed_env(message: impl Into<String>) -> NxIrRuntimeError {
    NxIrRuntimeError::new(
        "nx-ir-boundary-type",
        format!(
            "A component descriptor's '$env' is malformed: {}.",
            message.into()
        ),
    )
}

/// The entries of a `$env` value in host form.
fn parse_entries(carried: &NxValue) -> Result<Vec<EnvironmentEntry>> {
    let items: &[NxValue] = match carried {
        NxValue::Array(items) => items,
        NxValue::Null => &[],
        _ => return Err(malformed_env("it is not a list")),
    };
    items
        .iter()
        .map(|item| {
            let NxValue::Record { properties, .. } = item else {
                return Err(malformed_env("an entry is not a record"));
            };
            let Some(NxValue::String(value)) = properties.get("value") else {
                return Err(malformed_env("an entry has no 'value' string"));
            };
            let path = match properties.get("path") {
                None | Some(NxValue::Null) => Vec::new(),
                Some(NxValue::String(field)) => vec![field.clone()],
                Some(NxValue::Array(fields)) => fields
                    .iter()
                    .map(|field| match field {
                        NxValue::String(field) => Ok(field.clone()),
                        _ => Err(malformed_env("a 'path' entry is not a string")),
                    })
                    .collect::<Result<_>>()?,
                Some(_) => return Err(malformed_env("an entry's 'path' is not a list")),
            };
            let Some(to) = properties.get("to") else {
                return Err(malformed_env("an entry has no 'to'"));
            };
            Ok(EnvironmentEntry {
                value: value.clone(),
                path,
                to: to.clone(),
            })
        })
        .collect()
}

/// An environment declaration across the program: its module and declaration index.
pub(crate) type EnvKey = (u32, u32);

/// Every environment declaration of a linked program, by qualified name and by simple name.
#[derive(Debug, Default)]
pub(crate) struct EnvironmentIndex {
    qualified: HashMap<Arc<str>, EnvKey>,
    names: HashMap<EnvKey, Arc<str>>,
    simple: HashMap<Arc<str>, Vec<EnvKey>>,
}

impl EnvironmentIndex {
    pub(crate) fn build(program: &ProgramData) -> Self {
        let mut index = Self::default();
        for (module, linked) in program.modules.iter().enumerate() {
            let module = module as u32;
            for (declaration, entry) in linked.module.declarations.iter().enumerate() {
                if !matches!(entry.kind, DeclarationKind::Environment { .. }) {
                    continue;
                }
                let Some(simple) = entry.name.strip_prefix(DECLARATION_PREFIX) else {
                    continue;
                };
                let key = (module, declaration as u32);
                let qualified: Arc<str> =
                    Arc::from(format!("{}::{simple}", linked.module.identity));
                index.qualified.insert(Arc::clone(&qualified), key);
                index.names.insert(key, qualified);
                index.simple.entry(Arc::from(simple)).or_default().push(key);
            }
        }
        index
    }

    /// The declaration a qualified name, `identity::name`, names.
    pub(crate) fn by_qualified(&self, name: &str) -> Option<EnvKey> {
        self.qualified.get(name).copied()
    }

    /// The qualified name of a declaration.
    pub(crate) fn qualified(&self, key: EnvKey) -> Arc<str> {
        self.names
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Arc::from(""))
    }

    /// The declaration a host names a root value by: a qualified name, or a simple name exactly
    /// one environment value of the program has.
    fn by_host_name(&self, name: &str) -> Result<EnvKey> {
        if let Some(key) = self.by_qualified(name) {
            return Ok(key);
        }
        match self.simple.get(name).map(Vec::as_slice) {
            Some([only]) => Ok(*only),
            Some(several) if several.len() > 1 => {
                let mut modules: Vec<&str> = several
                    .iter()
                    .filter_map(|key| self.names.get(key))
                    .map(|qualified| qualified.as_ref())
                    .collect();
                modules.sort_unstable();
                fail(
                    "nx-ir-environment-name",
                    format!(
                        "Environment value '{name}' is declared more than once in the program ({}); name it as 'identity::{name}'.",
                        modules.join(", ")
                    ),
                )
            }
            _ => fail(
                "nx-ir-environment-name",
                format!("The program declares no environment value named '{name}'."),
            ),
        }
    }
}

/// One layer of the environment stack.
#[derive(Debug, Clone)]
pub(crate) enum Layer {
    /// An override: the value it sets at `path` of the value `key`.
    Set {
        key: EnvKey,
        path: Arc<[Arc<str>]>,
        value: Value,
    },
    /// While the entry module's provide at `index` is evaluated for `key`: a read of `key` sees
    /// only the provides before it and the default.
    Cap { key: EnvKey, index: usize },
}

/// One pending field override on the way to a base value.
struct Pending {
    path: Arc<[Arc<str>]>,
    value: Value,
    /// An update record from a bundle, applied as a patch rather than set.
    patch: bool,
}

/// The environment state of one evaluation.
#[derive(Debug, Default)]
pub(crate) struct EnvState {
    pub layers: Vec<Layer>,
    /// How many of `layers` came from the host's environment, below any `envProvide`.
    pub base: usize,
    pub roots: HashMap<EnvKey, Value>,
    /// How deeply reads are nested in defaults and provides being evaluated for a read.
    pub nested: u32,
    /// The reads the evaluation made at its own level, with the value each found beneath the
    /// overrides the evaluation pushed itself, when the caller records them.
    pub reads: Option<Vec<(EnvKey, Value)>>,
}

impl<'p> Machine<'p> {
    /// The program's environment index, built on first use.
    pub(crate) fn environment_index(&self) -> &'p EnvironmentIndex {
        self.program.environment_index()
    }

    /// Resolves and checks the environment the options carry, before anything is evaluated. A
    /// root value of an unknown name, or of the wrong type, fails the call.
    pub(crate) fn begin_environment(&self) -> Result<()> {
        let environment = &self.options.environment;
        if environment.roots.is_empty() && environment.entries.is_empty() {
            return Ok(());
        }
        let index = self.environment_index();
        let mut roots = HashMap::new();
        for (name, value) in environment.roots.iter() {
            let key = index.by_host_name(name)?;
            let value = self.host_environment_value(key, &[], value, name)?;
            roots.insert(key, value);
        }
        let mut layers = Vec::with_capacity(environment.entries.len());
        for entry in environment.entries.iter() {
            let Some(key) = index.by_qualified(&entry.value) else {
                return fail(
                    "nx-ir-environment-name",
                    format!(
                        "A '$env' entry names '{}', which the program does not declare.",
                        entry.value
                    ),
                );
            };
            let path: Arc<[Arc<str>]> = entry
                .path
                .iter()
                .map(|field| Arc::from(field.as_str()))
                .collect();
            let value = self.host_environment_value(key, &path, &entry.to, &entry.value)?;
            layers.push(Layer::Set { key, path, value });
        }
        let mut state = self.env.borrow_mut();
        state.base = layers.len();
        state.layers = layers;
        state.roots = roots;
        Ok(())
    }

    /// Starts recording the reads this evaluation makes at its own level.
    pub(crate) fn record_environment_reads(&self) {
        self.env.borrow_mut().reads = Some(Vec::new());
    }

    /// The reads recorded since [`Self::record_environment_reads`], by qualified name.
    pub(crate) fn take_environment_reads(&self) -> Vec<(Arc<str>, Value)> {
        let reads = self.env.borrow_mut().reads.take().unwrap_or_default();
        let index = self.environment_index();
        let mut seen: Vec<(Arc<str>, Value)> = Vec::new();
        for (key, value) in reads {
            let name = index.qualified(key);
            if !seen.iter().any(|(known, _)| *known == name) {
                seen.push((name, value));
            }
        }
        seen
    }

    /// Whether every recorded read gives the same value in this machine's environment, read at
    /// its base. Each read re-evaluated is paid for as a read is.
    pub(crate) fn environment_reads_hold(&self, reads: &[(Arc<str>, Value)]) -> Result<bool> {
        let index = self.environment_index();
        let meter = self.meter(crate::component::entry_cx(0), None);
        for (name, recorded) in reads {
            let Some(key) = index.by_qualified(name) else {
                return Ok(false);
            };
            let cx = Cx {
                module: key.0,
                declaration: key.1,
                depth: 0,
            };
            let current = self.environment_value(cx, None, key)?;
            if !values_equal(&current, recorded, &meter, &self.stack)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// A value the host gave for `key` at `path`, converted and checked against the type there.
    fn host_environment_value(
        &self,
        key: EnvKey,
        path: &[Arc<str>],
        value: &NxValue,
        name: &str,
    ) -> Result<Value> {
        let (cx, ty) = self.environment_type_at(key, path)?;
        let value = from_host(value, &self.stack)?;
        self.normalize(
            cx,
            &ty,
            value,
            &Path::Root(&Quoted("environment value '", name, "'")),
        )
    }

    /// The declaration of `key`'s type and default.
    fn environment_declaration(&self, key: EnvKey) -> Result<(u32, &'p Arc<Type>)> {
        let declaration = self
            .program
            .linked(key.0)
            .and_then(|linked| linked.module.declaration(key.1));
        match declaration.map(|declaration| &declaration.kind) {
            Some(DeclarationKind::Environment { default, ty }) => Ok((*default, ty)),
            _ => fail(
                "nx-ir-reference",
                "An environment reference names a declaration that is not an environment value.",
            ),
        }
    }

    /// The type of `key`'s value at `path`, with the context its references resolve in.
    fn environment_type_at(&self, key: EnvKey, path: &[Arc<str>]) -> Result<(Cx, Arc<Type>)> {
        let (_, ty) = self.environment_declaration(key)?;
        let mut cx = Cx {
            module: key.0,
            declaration: key.1,
            depth: 0,
        };
        let mut ty = Arc::clone(ty);
        for field in path {
            let mut item = Arc::clone(&ty);
            while let Type::Seq { item: inner, .. } = item.as_ref() {
                item = Arc::clone(inner);
            }
            let Type::Nominal(reference) = item.as_ref() else {
                return fail(
                    "nx-ir-environment",
                    format!("Environment path field '{field}' names no record field."),
                );
            };
            let (module, index, declaration) = self
                .program
                .resolve(cx.module, reference)
                .map_err(|message| NxIrRuntimeError::new("nx-ir-reference", message))?;
            let DeclarationKind::Record(record) = &declaration.kind else {
                return fail(
                    "nx-ir-environment",
                    format!("Environment path field '{field}' names no record field."),
                );
            };
            let Some(found) = record
                .fields
                .iter()
                .find(|candidate| candidate.name == *field)
            else {
                return fail(
                    "nx-ir-environment",
                    format!(
                        "Record '{}' has no field '{field}' for an environment override.",
                        declaration.name
                    ),
                );
            };
            cx = Cx {
                module,
                declaration: index,
                depth: 0,
            };
            ty = Arc::clone(&found.ty);
        }
        Ok((cx, ty))
    }

    /// The environment value `key` at a read in `cx`.
    pub(crate) fn environment_value(
        &self,
        cx: Cx,
        node: Option<u32>,
        key: EnvKey,
    ) -> Result<Value> {
        let mut pending: Vec<Pending> = Vec::new();
        let mut local: usize = 0;
        let mut base: Option<Value> = None;
        let mut cap: Option<usize> = None;
        let record;
        {
            let state = self.env.borrow();
            record = state.nested == 0 && state.reads.is_some();
            for (position, layer) in state.layers.iter().enumerate().rev() {
                match layer {
                    Layer::Set {
                        key: set,
                        path,
                        value,
                    } if *set == key => {
                        if path.is_empty() {
                            base = Some(value.clone());
                            if position >= state.base {
                                // Set by the evaluation itself: nothing beneath it is read.
                                return self.apply_pending(cx, node, value.clone(), pending);
                            }
                            break;
                        }
                        if position >= state.base {
                            local = local.saturating_add(1);
                        }
                        pending.push(Pending {
                            path: Arc::clone(path),
                            value: value.clone(),
                            patch: false,
                        });
                    }
                    Layer::Cap { key: capped, index } if *capped == key => {
                        cap = Some(*index);
                        break;
                    }
                    _ => {}
                }
            }
            if base.is_none() && cap.is_none() {
                base = state.roots.get(&key).cloned();
            }
        }
        let base = match base {
            Some(base) => base,
            None => {
                {
                    let mut state = self.env.borrow_mut();
                    state.nested = state.nested.saturating_add(1);
                }
                let found = self.provided_or_default(cx, node, key, cap.unwrap_or(usize::MAX));
                let mut state = self.env.borrow_mut();
                state.nested = state.nested.saturating_sub(1);
                drop(state);
                found?
            }
        };
        // The pending overrides were collected innermost first; the evaluation's own are the
        // first `local` of them, and are applied last.
        let own: Vec<Pending> = pending.drain(..local.min(pending.len())).collect();
        let beneath = self.apply_pending(cx, node, base, pending)?;
        if record {
            if let Some(reads) = self.env.borrow_mut().reads.as_mut() {
                reads.push((key, beneath.clone()));
            }
        }
        self.apply_pending(cx, node, beneath, own)
    }

    /// `base` with `pending`, collected innermost first, applied outermost first.
    fn apply_pending(
        &self,
        cx: Cx,
        node: Option<u32>,
        base: Value,
        pending: Vec<Pending>,
    ) -> Result<Value> {
        let mut value = base;
        for entry in pending.into_iter().rev() {
            value = self.set_path(cx, node, value, &entry.path, entry.value, entry.patch)?;
        }
        Ok(value)
    }

    /// `value` with `to` set at `path`, or with `patch` applied there as an update record.
    fn set_path(
        &self,
        cx: Cx,
        node: Option<u32>,
        value: Value,
        path: &[Arc<str>],
        to: Value,
        patch: bool,
    ) -> Result<Value> {
        let Some((field, rest)) = path.split_first() else {
            if !patch {
                return Ok(to);
            }
            return match (&value, &to) {
                (Value::Record(record), Value::Record(update)) => {
                    update::apply(record, update, self.carried(record))
                }
                _ => Err(self.error(
                    Some(cx),
                    node,
                    "nx-ir-environment",
                    "An environment update record applies only to a record value.",
                )),
            };
        };
        let Value::Record(record) = &value else {
            return Err(self.error(
                Some(cx),
                node,
                "nx-ir-environment",
                format!("Environment override of field '{field}' needs a record value."),
            ));
        };
        let current = record.get(field).cloned();
        let next = match current {
            Some(current) => self.set_path(cx, node, current, rest, to, patch)?,
            None if rest.is_empty() && !patch => to,
            None => {
                return Err(self.error(
                    Some(cx),
                    node,
                    "nx-ir-environment",
                    format!("Environment override of field '{field}' found the field empty."),
                ))
            }
        };
        let mut fields = record.fields.clone();
        set_field(&mut fields, Arc::clone(field), next);
        Ok(Value::Record(Arc::new(Record {
            type_name: record.type_name.clone(),
            fields,
            origin: record.origin.clone(),
        })))
    }

    /// The value `key` gets from the entry module's provides before `below`, or else from its
    /// default, each evaluated in the environment of the read.
    fn provided_or_default(
        &self,
        cx: Cx,
        node: Option<u32>,
        key: EnvKey,
        below: usize,
    ) -> Result<Value> {
        let mut pending: Vec<Pending> = Vec::new();
        if let Some((declaration, provides)) = self.entry_provides() {
            let mut index = below.min(provides.len());
            let provides_cx = Cx {
                module: 0,
                declaration,
                depth: cx.depth,
            };
            while index > 0 {
                index = index.saturating_sub(1);
                let Some(provide) = provides.get(index) else {
                    continue;
                };
                match &provide.target {
                    Some(target) => {
                        let (module, found, _) =
                            self.program.resolve(0, target).map_err(|message| {
                                self.error(Some(provides_cx), None, "nx-ir-reference", message)
                            })?;
                        if (module, found) != key {
                            continue;
                        }
                        let value = self.capped(key, index, || {
                            self.eval(provides_cx, &mut Vec::new(), provide.value)
                        })?;
                        let path: Arc<[Arc<str>]> = provide.path.iter().cloned().collect();
                        if path.is_empty() {
                            return self.apply_pending(cx, node, value, pending);
                        }
                        pending.push(Pending {
                            path,
                            value,
                            patch: false,
                        });
                    }
                    None => {
                        let bundle = self.capped(key, index, || {
                            self.eval(provides_cx, &mut Vec::new(), provide.value)
                        })?;
                        let entries = self.bundle_entries(provides_cx, provide.value, &bundle)?;
                        for (set, path, value) in entries.into_iter().rev() {
                            if set != key {
                                continue;
                            }
                            let patch = is_update_record(&value);
                            if path.is_empty() && !patch {
                                return self.apply_pending(cx, node, value, pending);
                            }
                            pending.push(Pending { path, value, patch });
                        }
                    }
                }
            }
        }
        let (default, ty) = self.environment_declaration(key)?;
        let declared = Cx {
            module: key.0,
            declaration: key.1,
            depth: cx.depth,
        };
        let value = self.eval(declared, &mut Vec::new(), default)?;
        let name = self.environment_index().qualified(key);
        let value = self.normalize(
            declared,
            ty,
            value,
            &Path::Root(&Quoted("environment value '", &name, "'")),
        )?;
        self.apply_pending(cx, node, value, pending)
    }

    /// The entry module's provides declaration, if it has one.
    fn entry_provides(&self) -> Option<(u32, &'p [crate::module::Provide])> {
        let linked = self.program.linked(0)?;
        let (index, declaration) = linked.module.find("$provides")?;
        match &declaration.kind {
            DeclarationKind::Provides(provides) => Some((index, provides)),
            _ => None,
        }
    }

    /// Runs `work` with a read of `key` capped below the provide at `index`.
    fn capped(
        &self,
        key: EnvKey,
        index: usize,
        work: impl FnOnce() -> Result<Value>,
    ) -> Result<Value> {
        self.env.borrow_mut().layers.push(Layer::Cap { key, index });
        let result = work();
        self.env.borrow_mut().layers.pop();
        result
    }

    /// The entries of a bundle value, in order.
    fn bundle_entries(
        &self,
        cx: Cx,
        node: u32,
        bundle: &Value,
    ) -> Result<Vec<(EnvKey, Arc<[Arc<str>]>, Value)>> {
        let entries = match bundle {
            Value::Record(record) if record.type_name() == Some(BUNDLE_TYPE) => {
                record.get(BUNDLE_ENTRIES).cloned()
            }
            _ => None,
        };
        let Some(Value::Seq(entries)) = entries else {
            return Err(self.error(
                Some(cx),
                Some(node),
                "nx-ir-environment",
                format!(
                    "A bundle provide needs an 'env.Update' value, got {}.",
                    bundle.describe()
                ),
            ));
        };
        let index = self.environment_index();
        let mut out = Vec::with_capacity(entries.len());
        for entry in entries.iter() {
            let (Some(Value::Str(name)), Some(path), Some(to)) = (
                field(entry, "value"),
                field(entry, "path"),
                field(entry, "to"),
            ) else {
                return Err(self.error(
                    Some(cx),
                    Some(node),
                    "nx-ir-environment",
                    "An 'env.Update' entry is malformed.",
                ));
            };
            let Some(key) = index.by_qualified(name) else {
                return Err(self.error(
                    Some(cx),
                    Some(node),
                    "nx-ir-environment-name",
                    format!(
                        "An 'env.Update' entry names '{name}', which the program does not declare."
                    ),
                ));
            };
            let path: Arc<[Arc<str>]> = match path {
                Value::Seq(items) => items
                    .iter()
                    .filter_map(|item| match item {
                        Value::Str(field) => Some(Arc::clone(field)),
                        _ => None,
                    })
                    .collect(),
                _ => Arc::from(Vec::new()),
            };
            out.push((key, path, to.clone()));
        }
        Ok(out)
    }

    /// The overrides of an `envProvide` or `envUpdate` node, evaluated in order outside it.
    pub(crate) fn eval_overrides(
        &self,
        cx: Cx,
        frame: &mut crate::eval::Frame,
        node: u32,
        overrides: &[Override],
    ) -> Result<Vec<(EnvKey, Arc<[Arc<str>]>, Value)>> {
        let mut out = Vec::with_capacity(overrides.len());
        for entry in overrides {
            let (module, index, declaration) = self
                .program
                .resolve(cx.module, &entry.target)
                .map_err(|message| self.error(Some(cx), Some(node), "nx-ir-reference", message))?;
            if !matches!(declaration.kind, DeclarationKind::Environment { .. }) {
                return Err(self.error(
                    Some(cx),
                    Some(node),
                    "nx-ir-reference",
                    format!("'{}' is not an environment value.", entry.target.name),
                ));
            }
            let value = self.eval(cx, frame, entry.value)?;
            out.push(((module, index), entry.path.iter().cloned().collect(), value));
        }
        Ok(out)
    }

    /// An `envProvide` node: the overrides evaluated outside, the body inside them, and the
    /// overrides added to every component descriptor the result reaches.
    pub(crate) fn eval_provide(
        &self,
        cx: Cx,
        frame: &mut crate::eval::Frame,
        node: u32,
        overrides: &[Override],
        body: u32,
    ) -> Result<Value> {
        let evaluated = self.eval_overrides(cx, frame, node, overrides)?;
        let depth = {
            let mut state = self.env.borrow_mut();
            let depth = state.layers.len();
            for (key, path, value) in &evaluated {
                state.layers.push(Layer::Set {
                    key: *key,
                    path: Arc::clone(path),
                    value: value.clone(),
                });
            }
            depth
        };
        let result = self.eval(cx, frame, body);
        self.env.borrow_mut().layers.truncate(depth);
        let result = result?;
        let entries = self.entry_values(&evaluated);
        self.stamp(cx, node, result, &entries)
    }

    /// An `envUpdate` node: the overrides, evaluated, as one bundle value.
    pub(crate) fn eval_bundle(
        &self,
        cx: Cx,
        frame: &mut crate::eval::Frame,
        node: u32,
        overrides: &[Override],
    ) -> Result<Value> {
        let evaluated = self.eval_overrides(cx, frame, node, overrides)?;
        let entries = self.entry_values(&evaluated);
        Ok(Value::record_from(
            Some(Arc::from(BUNDLE_TYPE)),
            vec![(Arc::from(BUNDLE_ENTRIES), Value::seq(entries))],
            self.origin(cx, node),
        ))
    }

    /// The `$env` entries for evaluated overrides.
    fn entry_values(&self, evaluated: &[(EnvKey, Arc<[Arc<str>]>, Value)]) -> Vec<Value> {
        let index = self.environment_index();
        evaluated
            .iter()
            .map(|(key, path, value)| {
                Value::record(
                    None,
                    vec![
                        (Arc::from("value"), Value::Str(index.qualified(*key))),
                        (
                            Arc::from("path"),
                            Value::seq(
                                path.iter()
                                    .map(|field| Value::Str(Arc::clone(field)))
                                    .collect(),
                            ),
                        ),
                        (Arc::from("to"), value.clone()),
                    ],
                )
            })
            .collect()
    }

    /// `value` with `entries` in front of the `$env` of every component descriptor it reaches,
    /// walking sequences and the fields of every other record. Each descriptor costs one
    /// operation.
    fn stamp(&self, cx: Cx, node: u32, value: Value, entries: &[Value]) -> Result<Value> {
        if entries.is_empty() {
            return Ok(value);
        }
        if !self.stack.within() {
            return self.stack.value_too_deep();
        }
        match value {
            Value::Seq(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items.iter() {
                    out.push(self.stamp(cx, node, item.clone(), entries)?);
                }
                Ok(Value::seq(out))
            }
            Value::Record(record) => {
                let is_descriptor = record
                    .type_name()
                    .is_some_and(|name| self.program.is_component_name(name));
                let mut fields = record.fields.clone();
                if is_descriptor {
                    self.charge(cx, Some(node), 1)?;
                    let mut carried: Vec<Value> = entries.to_vec();
                    if let Some(Value::Seq(existing)) = record.get(ENV_FIELD) {
                        carried.extend(existing.iter().cloned());
                    }
                    set_field(&mut fields, Arc::from(ENV_FIELD), Value::seq(carried));
                } else {
                    for (_, field) in fields.iter_mut() {
                        let taken = std::mem::replace(field, Value::empty());
                        *field = self.stamp(cx, node, taken, entries)?;
                    }
                }
                Ok(Value::Record(Arc::new(Record {
                    type_name: record.type_name.clone(),
                    fields,
                    origin: record.origin.clone(),
                })))
            }
            other => Ok(other),
        }
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Record(record) => record.get(name),
        _ => None,
    }
}

fn is_update_record(value: &Value) -> bool {
    matches!(value, Value::Record(record) if record.type_name().is_some_and(|name| name.ends_with(".Update")))
}

/// `properties` without the reserved `$env` field, which is not a prop: a host may pass a
/// descriptor's fields as props as they are.
pub(crate) fn without_env(
    properties: &BTreeMap<String, NxValue>,
) -> std::borrow::Cow<'_, BTreeMap<String, NxValue>> {
    if properties.contains_key(ENV_FIELD) {
        let mut properties = properties.clone();
        properties.remove(ENV_FIELD);
        std::borrow::Cow::Owned(properties)
    } else {
        std::borrow::Cow::Borrowed(properties)
    }
}

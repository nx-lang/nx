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

/// One entry a component descriptor carries in `$env`, in host form.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvironmentEntry {
    /// `{value, path, to}`: the value an override set, evaluated where it was written.
    Value {
        /// The environment value: its module identity and name, `identity::name`.
        value: String,
        /// The field path the override sets; empty for the whole value.
        path: Vec<String>,
        /// The value it sets, in canonical form.
        to: NxValue,
    },
    /// `{value, path, formula}`: an override computed where the value is read. The formula is
    /// `{module, declaration, node, fingerprint, locals}`; only its shape is checked when it is
    /// entered, and a formula of another build is refused at the read that evaluates it.
    Formula {
        value: String,
        path: Vec<String>,
        formula: NxValue,
    },
    /// `{bundle: [...]}`: the entries of a bundle applied by `provide env with`, in order.
    Bundle(Vec<EnvironmentEntry>),
}

/// An evaluated override: the key it sets, the field path inside the value, and what it sets.
type EvaluatedOverride = (EnvKey, Arc<[Arc<str>]>, Source);

/// What an `envProvide` or `envUpdate` override gives once evaluated where it is written.
enum Evaluated {
    /// An override of one value.
    Set(EvaluatedOverride),
    /// A bundle: its entries, and the `$entries` list they came from.
    Bundle(Vec<EvaluatedOverride>, Arc<[Value]>),
}

/// What an override or bundle entry sets: a value, or a formula computed where it is read.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    Value(Value),
    Formula(Arc<Formula>),
}

/// A captured formula: the node of an override's value, in the declaration it was written in,
/// with the locals it reads.
#[derive(Debug)]
pub(crate) struct Formula {
    pub module: u32,
    pub declaration: u32,
    pub node: u32,
    /// The locals the node reads from around it, by name, in name order.
    pub locals: Vec<(Arc<str>, Value)>,
    /// Why the formula is not one of this program's, when it came from another build or names
    /// what the program does not have: it is refused when it is evaluated.
    pub refused: Option<String>,
    /// The formula in canonical form, as `$env` and `$entries` carry it.
    pub data: Value,
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
    /// `$env` that is not a list of `{value, path, to}`, `{value, path, formula}` and
    /// `{bundle}` records fails with `nx-ir-boundary-type`. A formula's shape is checked here;
    /// whether it is one of the program's is checked at the read that evaluates it.
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
    items.iter().map(parse_entry).collect()
}

fn parse_entry(item: &NxValue) -> Result<EnvironmentEntry> {
    let NxValue::Record { properties, .. } = item else {
        return Err(malformed_env("an entry is not a record"));
    };
    if let Some(bundle) = properties.get("bundle") {
        let NxValue::Array(entries) = bundle else {
            return Err(malformed_env("a 'bundle' entry is not a list"));
        };
        return Ok(EnvironmentEntry::Bundle(
            entries.iter().map(parse_entry).collect::<Result<_>>()?,
        ));
    }
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
    if let Some(formula) = properties.get("formula") {
        check_formula_shape(formula)?;
        return Ok(EnvironmentEntry::Formula {
            value: value.clone(),
            path,
            formula: formula.clone(),
        });
    }
    let Some(to) = properties.get("to") else {
        return Err(malformed_env("an entry has no 'to' and no 'formula'"));
    };
    Ok(EnvironmentEntry::Value {
        value: value.clone(),
        path,
        to: to.clone(),
    })
}

/// Checks that a `formula` is `{module, declaration, node, fingerprint, locals}`.
fn check_formula_shape(formula: &NxValue) -> Result<()> {
    let NxValue::Record { properties, .. } = formula else {
        return Err(malformed_env("a 'formula' is not a record"));
    };
    for name in ["module", "declaration", "fingerprint"] {
        if !matches!(properties.get(name), Some(NxValue::String(_))) {
            return Err(malformed_env(format!("a 'formula' has no '{name}' string")));
        }
    }
    if !matches!(properties.get("node"), Some(NxValue::Int(node)) if *node >= 0) {
        return Err(malformed_env("a 'formula' has no 'node' index"));
    }
    if !matches!(
        properties.get("locals"),
        None | Some(NxValue::Null) | Some(NxValue::Record { .. })
    ) {
        return Err(malformed_env("a 'formula's 'locals' is not a record"));
    }
    Ok(())
}

/// An environment declaration across the program: its module and declaration index.
pub(crate) type EnvKey = (u32, u32);

/// Every environment declaration of a linked program, by qualified name and by simple name.
#[derive(Debug, Default)]
pub(crate) struct EnvironmentIndex {
    qualified: HashMap<Arc<str>, EnvKey>,
    names: HashMap<EnvKey, Arc<str>>,
    simple: HashMap<Arc<str>, Vec<EnvKey>>,
    /// The value nodes of every formula override, by module: the only nodes a formula may name.
    formulas: std::collections::HashSet<(u32, u32)>,
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
            for node in 0..linked.module.node_count() as u32 {
                let overrides = match linked.module.node(node) {
                    Ok(crate::module::Node::EnvProvide { overrides, .. })
                    | Ok(crate::module::Node::EnvUpdate { overrides, .. }) => overrides,
                    _ => continue,
                };
                for entry in overrides.iter().filter(|entry| entry.formula) {
                    index.formulas.insert((module, entry.value));
                }
            }
        }
        index
    }

    /// Whether node `node` of `module` is the value of a formula override.
    pub(crate) fn is_formula(&self, module: u32, node: u32) -> bool {
        self.formulas.contains(&(module, node))
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

/// Where a read of a value continues while one of that value's sources is being evaluated: the
/// sources after the one being evaluated.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Below {
    /// The layers beneath this position, then the root value, the provides and the default.
    Layer(usize),
    /// The entry module's provides before this index, then the default.
    Provide(usize),
    /// Nothing: the default is being evaluated, so a read of the value is a cycle.
    Exhausted,
}

/// One layer of the environment stack.
#[derive(Debug, Clone)]
pub(crate) enum Layer {
    /// An override: what it sets at `path` of the value `key`. An entry of a bundle whose value
    /// is an update record patches the value rather than replacing it.
    Set {
        key: EnvKey,
        path: Arc<[Arc<str>]>,
        source: Source,
        bundle: bool,
    },
    /// While a source of `key` is evaluated: a read of `key` sees only the sources after it.
    Cap { key: EnvKey, below: Below },
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
    /// The reads the evaluation made, those made for a default, a provide or a formula included,
    /// with the value each found beneath the overrides the evaluation pushed itself, when the
    /// caller records them. A read that evaluated a default, a provide or a formula while
    /// overrides the evaluation pushed were in effect has no value here: what it found depends
    /// on those overrides, so its value beneath them is read again once the evaluation is done.
    pub reads: Option<Vec<(EnvKey, Option<Value>)>>,
}

impl<'p> Machine<'p> {
    /// The program's environment index, built on first use.
    pub(crate) fn environment_index(&self) -> &'p EnvironmentIndex {
        self.program.environment_index()
    }

    /// Resolves and checks the environment the options carry, before anything is evaluated. A
    /// root value of an unknown name, or of the wrong type, fails the call. A formula is checked
    /// only for its shape here, and refused at the read that evaluates it if it is not one of
    /// the program's.
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
            self.host_layers(entry, false, &mut layers)?;
        }
        let mut state = self.env.borrow_mut();
        state.base = layers.len();
        state.layers = layers;
        state.roots = roots;
        Ok(())
    }

    /// The layers one host `$env` entry gives: one, or one for each entry of a bundle.
    fn host_layers(
        &self,
        entry: &EnvironmentEntry,
        bundle: bool,
        layers: &mut Vec<Layer>,
    ) -> Result<()> {
        let index = self.environment_index();
        let named = |value: &str| {
            index.by_qualified(value).ok_or_else(|| {
                NxIrRuntimeError::new(
                    "nx-ir-environment-name",
                    format!("A '$env' entry names '{value}', which the program does not declare."),
                )
            })
        };
        let path_of = |path: &[String]| -> Arc<[Arc<str>]> {
            path.iter().map(|field| Arc::from(field.as_str())).collect()
        };
        match entry {
            EnvironmentEntry::Value { value, path, to } => {
                let key = named(value)?;
                let path = path_of(path);
                // A bundle entry may hold an update record that patches the value, which the
                // value's own type does not describe.
                let to = if bundle {
                    from_host(to, &self.stack)?
                } else {
                    self.host_environment_value(key, &path, to, value)?
                };
                layers.push(Layer::Set {
                    key,
                    path,
                    source: Source::Value(to),
                    bundle,
                });
            }
            EnvironmentEntry::Formula {
                value,
                path,
                formula,
            } => {
                let key = named(value)?;
                let data = from_host(formula, &self.stack)?;
                layers.push(Layer::Set {
                    key,
                    path: path_of(path),
                    source: Source::Formula(Arc::new(self.formula_from_data(data)?)),
                    bundle,
                });
            }
            EnvironmentEntry::Bundle(entries) => {
                for entry in entries {
                    self.host_layers(entry, true, layers)?;
                }
            }
        }
        Ok(())
    }

    /// Starts recording the reads this evaluation makes at its own level.
    pub(crate) fn record_environment_reads(&self) {
        self.env.borrow_mut().reads = Some(Vec::new());
    }

    /// The reads recorded since [`Self::record_environment_reads`], by qualified name.
    ///
    /// <para>A read recorded without a value is read again here, in a machine of its own over
    /// the same environment with none of this evaluation's overrides, which is the read the skip
    /// check makes. It is not charged to this evaluation, as the check's reads are not. A read
    /// that fails there, such as a default that is valid only under the body's own override, is
    /// kept as `None`, which the skip check counts as changed, so tracking never fails a
    /// render.</para>
    pub(crate) fn take_environment_reads(&self) -> Vec<(Arc<str>, Option<Value>)> {
        let reads = self.env.borrow_mut().reads.take().unwrap_or_default();
        let index = self.environment_index();
        let mut seen: Vec<(Arc<str>, Option<Value>)> = Vec::new();
        let mut beneath: Option<Option<Machine<'_>>> = None;
        for (key, value) in reads {
            let name = index.qualified(key);
            if seen.iter().any(|(known, _)| *known == name) {
                continue;
            }
            let value = match value {
                Some(value) => Some(value),
                None => {
                    let machine = beneath.get_or_insert_with(|| {
                        let machine = Machine::new(self.program, self.options);
                        machine.begin_environment().ok().map(|()| machine)
                    });
                    let cx = Cx {
                        module: key.0,
                        declaration: key.1,
                        depth: 0,
                    };
                    machine
                        .as_ref()
                        .and_then(|machine| machine.environment_value(cx, None, key).ok())
                }
            };
            seen.push((name, value));
        }
        seen
    }

    /// Whether every recorded read gives the same value in this machine's environment, read at
    /// its base. Each read re-evaluated is paid for as a read is. A read recorded as `None`, or
    /// one that fails now, counts as changed, so the render runs again and reports any error.
    pub(crate) fn environment_reads_hold(
        &self,
        reads: &[(Arc<str>, Option<Value>)],
    ) -> Result<bool> {
        let index = self.environment_index();
        let meter = self.meter(crate::component::entry_cx(0), None);
        for (name, recorded) in reads {
            let (Some(key), Some(recorded)) = (index.by_qualified(name), recorded) else {
                return Ok(false);
            };
            let cx = Cx {
                module: key.0,
                declaration: key.1,
                depth: 0,
            };
            let Ok(current) = self.environment_value(cx, None, key) else {
                return Ok(false);
            };
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
    ///
    /// <para>The sources of a value, nearest first, are the layers of the stack from the top,
    /// then the host's root value, the entry module's provides, last first, and the default. The
    /// first source that sets the whole value is its base, and the field overrides on the way
    /// to it are applied to the base, outermost first. A formula, a provide and the default are
    /// evaluated at the read, each with a cap that makes a read of `key` inside it see only the
    /// sources after it; a read of `key` while its default is evaluated is a cycle.</para>
    pub(crate) fn environment_value(
        &self,
        cx: Cx,
        node: Option<u32>,
        key: EnvKey,
    ) -> Result<Value> {
        let (record, pushed, base_len, top) = {
            let state = self.env.borrow();
            (
                state.reads.is_some(),
                state
                    .layers
                    .get(state.base..)
                    .unwrap_or_default()
                    .iter()
                    .any(|layer| matches!(layer, Layer::Set { .. })),
                state.base,
                state.layers.len(),
            )
        };
        // The overrides the evaluation pushed itself, and the ones beneath them, innermost first.
        let mut own: Vec<Pending> = Vec::new();
        let mut beneath: Vec<Pending> = Vec::new();
        let mut evaluated = false;
        let mut capped = false;
        let mut provides_below: Option<usize> = None;
        let mut found: Option<Value> = None;
        let mut position = top;
        while position > 0 {
            position -= 1;
            let layer = self.env.borrow().layers.get(position).cloned();
            match layer {
                Some(Layer::Set {
                    key: set,
                    path,
                    source,
                    bundle,
                }) if set == key => {
                    let is_own = position >= base_len;
                    if !is_own && matches!(source, Source::Formula(_)) {
                        evaluated = true;
                    }
                    let value = self.source_value(cx, node, key, Below::Layer(position), &source)?;
                    let patch = bundle && is_update_record(&value);
                    if path.is_empty() && !patch {
                        if is_own {
                            // Set by the evaluation itself: nothing beneath it is read.
                            return self.apply_pending(cx, node, value, own);
                        }
                        found = Some(value);
                        break;
                    }
                    let pending = Pending { path, value, patch };
                    if is_own {
                        own.push(pending);
                    } else {
                        beneath.push(pending);
                    }
                }
                Some(Layer::Cap { key: capped_key, below }) if capped_key == key => {
                    capped = true;
                    match below {
                        Below::Layer(at) => position = at,
                        Below::Provide(index) => {
                            provides_below = Some(index);
                            break;
                        }
                        Below::Exhausted => return Err(self.environment_cycle(cx, node, key)),
                    }
                }
                _ => {}
            }
        }
        if found.is_none() && provides_below.is_none() {
            found = self.env.borrow().roots.get(&key).cloned();
        }
        let base = match found {
            Some(base) => base,
            None => {
                evaluated = true;
                self.provided_or_default(cx, node, key, provides_below.unwrap_or(usize::MAX))?
            }
        };
        let value = self.apply_pending(cx, node, base, beneath)?;
        if record && !capped {
            // A source evaluated under the evaluation's own overrides of other values found
            // what those overrides made it; beneath them it may differ.
            let found = (!(evaluated && pushed)).then(|| value.clone());
            if let Some(reads) = self.env.borrow_mut().reads.as_mut() {
                reads.push((key, found));
            }
        }
        self.apply_pending(cx, node, value, own)
    }

    /// The `nx-ir-environment-cycle` error for a read of `key` while its default is evaluated,
    /// naming the values whose sources are being evaluated on the way.
    #[cold]
    fn environment_cycle(&self, cx: Cx, node: Option<u32>, key: EnvKey) -> NxIrRuntimeError {
        let index = self.environment_index();
        let state = self.env.borrow();
        let start = state
            .layers
            .iter()
            .rposition(|layer| {
                matches!(layer, Layer::Cap { key: capped, below: Below::Exhausted } if *capped == key)
            })
            .unwrap_or(0);
        let mut path: Vec<Arc<str>> = vec![index.qualified(key)];
        for layer in &state.layers[start..] {
            if let Layer::Cap { key: capped, .. } = layer {
                let name = index.qualified(*capped);
                if path.last() != Some(&name) {
                    path.push(name);
                }
            }
        }
        path.push(index.qualified(key));
        let named = path
            .iter()
            .map(|name| format!("'{}'", short_name(name)))
            .collect::<Vec<_>>()
            .join(" → ");
        self.error(
            Some(cx),
            node,
            "nx-ir-environment-cycle",
            format!(
                "Environment value '{}' is read while its own default is being evaluated: {named}.",
                short_name(&index.qualified(key))
            ),
        )
    }

    /// What a source gives: its value, or a formula evaluated with reads of `key` capped below
    /// `below`.
    fn source_value(
        &self,
        cx: Cx,
        node: Option<u32>,
        key: EnvKey,
        below: Below,
        source: &Source,
    ) -> Result<Value> {
        match source {
            Source::Value(value) => Ok(value.clone()),
            Source::Formula(formula) => {
                self.capped(key, below, || self.eval_formula(cx, node, formula))
            }
        }
    }

    /// Evaluates a formula in the environment of the read, with its captured locals bound.
    fn eval_formula(&self, cx: Cx, node: Option<u32>, formula: &Formula) -> Result<Value> {
        if let Some(reason) = &formula.refused {
            return Err(self.error(
                Some(cx),
                node,
                "nx-ir-boundary-type",
                format!("A formula an environment override carries is not one of this program's: {reason}."),
            ));
        }
        let formula_cx = Cx {
            module: formula.module,
            declaration: formula.declaration,
            depth: cx.depth,
        };
        let mut frame: crate::eval::Frame = Vec::new();
        if !formula.locals.is_empty() {
            for (slot, name) in self.slots_read(formula.module, formula.node) {
                if let Some((_, value)) = formula.locals.iter().find(|(local, _)| *local == name) {
                    crate::eval::bind(&mut frame, slot as usize, value.clone())?;
                }
            }
        }
        self.eval(formula_cx, &mut frame, formula.node)
    }

    /// The slots node `root` of `module` and the nodes beneath it read, with their names.
    fn slots_read(&self, module: u32, root: u32) -> Vec<(u32, Arc<str>)> {
        let Some(linked) = self.program.linked(module) else {
            return Vec::new();
        };
        let mut read = Vec::new();
        let mut stack = vec![root];
        let mut seen = std::collections::HashSet::new();
        while let Some(index) = stack.pop() {
            if !seen.insert(index) {
                continue;
            }
            let Ok(node) = linked.module.node(index) else {
                continue;
            };
            if let crate::module::Node::Slot { slot, name } = node {
                read.push((*slot, Arc::clone(name)));
            }
            node.children(&mut |child| stack.push(child));
        }
        read
    }

    /// Captures the formula an override's value is, in the frame as it stands: the locals the
    /// node reads, by name. Capturing costs one operation.
    fn capture_formula(
        &self,
        cx: Cx,
        frame: &crate::eval::Frame,
        node: u32,
        value: u32,
    ) -> Result<Formula> {
        self.charge(cx, Some(node), 1)?;
        let mut locals: Vec<(Arc<str>, Value)> = Vec::new();
        for (slot, name) in self.slots_read(cx.module, value) {
            let Some(Some(bound)) = frame.get(slot as usize) else {
                continue;
            };
            if locals.iter().all(|(known, _)| *known != name) {
                locals.push((name, bound.clone()));
            }
        }
        locals.sort_by(|left, right| left.0.cmp(&right.0));
        let linked = self.program.linked(cx.module);
        let identity = linked
            .map(|linked| Arc::clone(&linked.module.identity))
            .unwrap_or_else(|| Arc::from(""));
        let fingerprint = linked.map(|linked| linked.module.fingerprint).unwrap_or(0);
        let declaration = linked
            .and_then(|linked| linked.module.declaration(cx.declaration))
            .map(|declaration| Arc::clone(&declaration.name))
            .unwrap_or_else(|| Arc::from(""));
        let data = Value::record(
            None,
            vec![
                (Arc::from("module"), Value::Str(identity)),
                (Arc::from("declaration"), Value::Str(declaration)),
                (Arc::from("node"), Value::Int(i64::from(value))),
                (
                    Arc::from("fingerprint"),
                    Value::Str(Arc::from(fingerprint.to_string())),
                ),
                (Arc::from("locals"), Value::record(None, locals.clone())),
            ],
        );
        Ok(Formula {
            module: cx.module,
            declaration: cx.declaration,
            node: value,
            locals,
            refused: None,
            data,
        })
    }

    /// A formula from its canonical form, `{module, declaration, node, fingerprint, locals}`,
    /// with the reason it is refused when it is not one of this program's.
    fn formula_from_data(&self, data: Value) -> Result<Formula> {
        let malformed = || {
            NxIrRuntimeError::new(
                "nx-ir-boundary-type",
                "An environment formula is malformed: it is not {module, declaration, node, fingerprint, locals}.",
            )
        };
        let (
            Some(Value::Str(identity)),
            Some(Value::Str(declaration_name)),
            Some(Value::Int(node)),
            Some(Value::Str(fingerprint)),
        ) = (
            field(&data, "module"),
            field(&data, "declaration"),
            field(&data, "node"),
            field(&data, "fingerprint"),
        )
        else {
            return Err(malformed());
        };
        let locals: Vec<(Arc<str>, Value)> = match field(&data, "locals") {
            Some(Value::Record(record)) => record.fields.clone(),
            None => Vec::new(),
            Some(Value::Seq(items)) if items.is_empty() => Vec::new(),
            Some(_) => return Err(malformed()),
        };
        let node = u32::try_from(*node).map_err(|_| malformed())?;
        let mut formula = Formula {
            module: 0,
            declaration: 0,
            node,
            locals,
            refused: None,
            data: data.clone(),
        };
        let Some(module) = self.program.by_identity.get(identity).copied() else {
            formula.refused = Some(format!("module '{identity}' is not in the program"));
            return Ok(formula);
        };
        let Some(linked) = self.program.linked(module) else {
            formula.refused = Some(format!("module '{identity}' is not in the program"));
            return Ok(formula);
        };
        formula.module = module;
        if fingerprint.as_ref() != linked.module.fingerprint.to_string() {
            formula.refused = Some(format!(
                "it was made by another build of '{identity}', whose fingerprint was {fingerprint}"
            ));
            return Ok(formula);
        }
        let Some((declaration, _)) = linked.module.find(declaration_name) else {
            formula.refused = Some(format!(
                "'{identity}' declares nothing named '{declaration_name}'"
            ));
            return Ok(formula);
        };
        formula.declaration = declaration;
        if !self
            .environment_index()
            .is_formula(module, node)
        {
            formula.refused = Some(format!(
                "node {node} of '{identity}' is not the value of a formula override"
            ));
        }
        Ok(formula)
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
                        let value = self.capped(key, Below::Provide(index), || {
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
                        let bundle = self.capped(key, Below::Provide(index), || {
                            self.eval(provides_cx, &mut Vec::new(), provide.value)
                        })?;
                        let entries = self.bundle_entries(provides_cx, provide.value, &bundle)?;
                        for (set, path, source) in entries.into_iter().rev() {
                            if set != key {
                                continue;
                            }
                            let value =
                                self.source_value(cx, node, key, Below::Provide(index), &source)?;
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
        let value = self.capped(key, Below::Exhausted, || {
            self.eval(declared, &mut Vec::new(), default)
        })?;
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

    /// Runs `work` with a read of `key` seeing only the sources `below` gives.
    fn capped(
        &self,
        key: EnvKey,
        below: Below,
        work: impl FnOnce() -> Result<Value>,
    ) -> Result<Value> {
        let depth = {
            let mut state = self.env.borrow_mut();
            state.layers.push(Layer::Cap { key, below });
            state.layers.len() - 1
        };
        let result = work();
        self.env.borrow_mut().layers.truncate(depth);
        result
    }

    /// The `$entries` of a bundle value.
    fn bundle_items(&self, cx: Cx, node: u32, bundle: &Value) -> Result<Arc<[Value]>> {
        let entries = match bundle {
            Value::Record(record) if record.type_name() == Some(BUNDLE_TYPE) => {
                record.get(BUNDLE_ENTRIES).cloned()
            }
            _ => None,
        };
        match entries {
            Some(Value::Seq(entries)) => Ok(entries),
            _ => Err(self.error(
                Some(cx),
                Some(node),
                "nx-ir-environment",
                format!(
                    "A bundle needs an 'env.Update' value, got {}.",
                    bundle.describe()
                ),
            )),
        }
    }

    /// The entries of a bundle value, in order, a bundle entry's own entries in its place.
    fn bundle_entries(&self, cx: Cx, node: u32, bundle: &Value) -> Result<Vec<EvaluatedOverride>> {
        let items = self.bundle_items(cx, node, bundle)?;
        let mut out = Vec::with_capacity(items.len());
        self.entries_of(cx, node, &items, &mut out)?;
        Ok(out)
    }

    fn entries_of(
        &self,
        cx: Cx,
        node: u32,
        items: &[Value],
        out: &mut Vec<EvaluatedOverride>,
    ) -> Result<()> {
        let malformed = || {
            self.error(
                Some(cx),
                Some(node),
                "nx-ir-environment",
                "An 'env.Update' entry is malformed.",
            )
        };
        let index = self.environment_index();
        for entry in items {
            if let Some(nested) = field(entry, "bundle") {
                let Value::Seq(nested) = nested else {
                    return Err(malformed());
                };
                self.entries_of(cx, node, nested, out)?;
                continue;
            }
            let (Some(Value::Str(name)), Some(path)) = (field(entry, "value"), field(entry, "path"))
            else {
                return Err(malformed());
            };
            let source = match (field(entry, "to"), field(entry, "formula")) {
                (Some(to), None) => Source::Value(to.clone()),
                (None, Some(formula)) => {
                    Source::Formula(Arc::new(self.formula_from_data(formula.clone())?))
                }
                _ => return Err(malformed()),
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
            out.push((key, path, source));
        }
        Ok(())
    }

    /// The overrides of an `envProvide` or `envUpdate` node, evaluated in order outside it: a
    /// formula captured, a bundle evaluated to its entries.
    fn eval_overrides(
        &self,
        cx: Cx,
        frame: &mut crate::eval::Frame,
        node: u32,
        overrides: &[Override],
    ) -> Result<Vec<Evaluated>> {
        let mut out = Vec::with_capacity(overrides.len());
        for entry in overrides {
            let Some(target) = &entry.target else {
                let bundle = self.eval(cx, frame, entry.value)?;
                let items = self.bundle_items(cx, node, &bundle)?;
                let mut entries = Vec::with_capacity(items.len());
                self.entries_of(cx, node, &items, &mut entries)?;
                out.push(Evaluated::Bundle(entries, items));
                continue;
            };
            let (module, index, declaration) = self
                .program
                .resolve(cx.module, target)
                .map_err(|message| self.error(Some(cx), Some(node), "nx-ir-reference", message))?;
            if !matches!(declaration.kind, DeclarationKind::Environment { .. }) {
                return Err(self.error(
                    Some(cx),
                    Some(node),
                    "nx-ir-reference",
                    format!("'{}' is not an environment value.", target.name),
                ));
            }
            let source = if entry.formula {
                Source::Formula(Arc::new(self.capture_formula(cx, frame, node, entry.value)?))
            } else {
                Source::Value(self.eval(cx, frame, entry.value)?)
            };
            out.push(Evaluated::Set((
                (module, index),
                entry.path.iter().cloned().collect(),
                source,
            )));
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
            for entry in &evaluated {
                match entry {
                    Evaluated::Set((key, path, source)) => state.layers.push(Layer::Set {
                        key: *key,
                        path: Arc::clone(path),
                        source: source.clone(),
                        bundle: false,
                    }),
                    Evaluated::Bundle(entries, _) => {
                        for (key, path, source) in entries {
                            state.layers.push(Layer::Set {
                                key: *key,
                                path: Arc::clone(path),
                                source: source.clone(),
                                bundle: true,
                            });
                        }
                    }
                }
            }
            depth
        };
        let result = self.eval(cx, frame, body);
        self.env.borrow_mut().layers.truncate(depth);
        let result = result?;
        let entries = self.entry_values(&evaluated, true);
        self.stamp(cx, node, result, &entries)
    }

    /// An `envUpdate` node: the base's entries, then the overrides, evaluated, as one bundle
    /// value.
    pub(crate) fn eval_bundle(
        &self,
        cx: Cx,
        frame: &mut crate::eval::Frame,
        node: u32,
        overrides: &[Override],
        base: Option<u32>,
    ) -> Result<Value> {
        let mut entries: Vec<Value> = Vec::new();
        if let Some(base) = base {
            let base = self.eval(cx, frame, base)?;
            entries.extend(self.bundle_items(cx, node, &base)?.iter().cloned());
        }
        let evaluated = self.eval_overrides(cx, frame, node, overrides)?;
        entries.extend(self.entry_values(&evaluated, false));
        Ok(Value::record_from(
            Some(Arc::from(BUNDLE_TYPE)),
            vec![(Arc::from(BUNDLE_ENTRIES), Value::seq(entries))],
            self.origin(cx, node),
        ))
    }

    /// The `$env` or `$entries` entries for evaluated overrides. In `$env` a bundle is one
    /// `{bundle}` entry; in a bundle's `$entries` its entries take its place.
    fn entry_values(&self, evaluated: &[Evaluated], nest: bool) -> Vec<Value> {
        let mut out = Vec::with_capacity(evaluated.len());
        for entry in evaluated {
            match entry {
                Evaluated::Set(set) => out.push(self.entry_value(set)),
                Evaluated::Bundle(_, items) if nest => out.push(Value::record(
                    None,
                    vec![(Arc::from("bundle"), Value::Seq(Arc::clone(items)))],
                )),
                Evaluated::Bundle(_, items) => out.extend(items.iter().cloned()),
            }
        }
        out
    }

    /// One entry in canonical form: `{value, path, to}` or `{value, path, formula}`.
    fn entry_value(&self, (key, path, source): &EvaluatedOverride) -> Value {
        let index = self.environment_index();
        let set = match source {
            Source::Value(value) => (Arc::from("to"), value.clone()),
            Source::Formula(formula) => (Arc::from("formula"), formula.data.clone()),
        };
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
                set,
            ],
        )
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

/// A qualified name, `identity::name`, without its module.
fn short_name(qualified: &str) -> &str {
    qualified.rsplit_once("::").map_or(qualified, |(_, name)| name)
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

impl crate::program::Program {
    /// The value of the environment value `name` in `options`' environment, with no override
    /// of an evaluation around it: what a body evaluated there reads for `env.name`. `name` is
    /// named as a host names a root value.
    pub fn environment_value(
        &self,
        name: &str,
        options: &crate::RuntimeOptions,
    ) -> Result<NxValue> {
        let machine = Machine::new(&self.data, options);
        machine.begin_environment()?;
        let key = machine.environment_index().by_host_name(name)?;
        let cx = Cx {
            module: key.0,
            declaration: key.1,
            depth: 0,
        };
        let value = machine.environment_value(cx, None, key)?;
        let meter = machine.meter(cx, None);
        machine.report(&value, None, &meter)
    }

    /// Whether every read gives the value it gave in `options`' environment.
    pub(crate) fn environment_reads_hold(
        &self,
        reads: &[(Arc<str>, Option<Value>)],
        options: &crate::RuntimeOptions,
    ) -> Result<bool> {
        if reads.is_empty() {
            return Ok(true);
        }
        let machine = Machine::new(&self.data, options);
        machine.begin_environment()?;
        machine.environment_reads_hold(reads)
    }

    /// The root values of `environment`, checked, by qualified name.
    pub(crate) fn root_values(
        &self,
        environment: &Environment,
        options: &crate::RuntimeOptions,
    ) -> Result<Vec<(Arc<str>, Value)>> {
        let options = crate::RuntimeOptions {
            environment: Environment {
                roots: Arc::clone(&environment.roots),
                entries: Arc::from(Vec::new()),
            },
            ..options.clone()
        };
        let machine = Machine::new(&self.data, &options);
        machine.begin_environment()?;
        let index = machine.environment_index();
        let state = machine.env.borrow();
        Ok(state
            .roots
            .iter()
            .map(|(key, value)| (index.qualified(*key), value.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(env: &str) -> NxValue {
        NxValue::from_json_str(&format!(
            r#"{{ "$type": "Card", "title": "t", "$env": {env} }}"#
        ))
        .unwrap()
    }

    #[test]
    fn entering_a_descriptor_appends_its_entries_inside_the_ones_held() {
        let outer = Environment::root([("mode", NxValue::String("dark".to_string()))])
            .enter(&descriptor(r#"[{ "value": "main::mode", "to": "dim" }]"#))
            .unwrap();
        let inner = outer
            .enter(&descriptor(
                r#"[{ "value": "main::size", "path": ["width"], "to": 4 }]"#,
            ))
            .unwrap();
        assert_eq!(inner.roots(), outer.roots());
        let values: Vec<_> = inner
            .entries()
            .iter()
            .map(|entry| match entry {
                EnvironmentEntry::Value { value, path, .. } => (value.as_str(), path.clone()),
                other => panic!("expected a value entry, got {other:?}"),
            })
            .collect();
        assert_eq!(
            values,
            [
                ("main::mode", Vec::new()),
                ("main::size", vec!["width".to_string()])
            ]
        );
        // A descriptor without `$env`, or with an empty one, keeps the environment.
        let plain = NxValue::from_json_str(r#"{ "$type": "Card" }"#).unwrap();
        assert_eq!(inner.enter(&plain).unwrap().entries().len(), 2);
        assert_eq!(inner.enter(&descriptor("[]")).unwrap().entries().len(), 2);
    }

    #[test]
    fn a_malformed_env_is_refused_as_a_boundary_type() {
        for env in [
            "5",
            "[3]",
            r#"[{ "to": 1 }]"#,
            r#"[{ "value": "main::mode" }]"#,
            r#"[{ "value": "main::mode", "path": 2, "to": 1 }]"#,
            r#"[{ "value": "main::mode", "path": [2], "to": 1 }]"#,
            r#"[{ "value": "main::mode", "formula": 1 }]"#,
            r#"[{ "value": "main::mode", "formula": { "module": "main", "node": 1 } }]"#,
            r#"[{ "bundle": 3 }]"#,
            r#"[{ "bundle": [{ "value": "main::mode" }] }]"#,
        ] {
            let error = Environment::default().enter(&descriptor(env)).unwrap_err();
            assert_eq!(error.code(), "nx-ir-boundary-type", "{env}: {error}");
        }
    }
}

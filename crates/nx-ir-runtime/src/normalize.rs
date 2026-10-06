//! Boundary normalization: where a value takes the shape its declared type gives it.
//!
//! <para>Host input and the program's own constructions go through the same rules, against the
//! schema the image carries: occurrences, records and their discriminators, abstract records,
//! unions, update records and function values.</para>

use crate::error::{fail, fail_limit, Result};
use crate::eval::{bind, Cx, Frame, Machine, STACK_LIMIT};
use crate::module::{DeclarationKind, Field, Primitive, Ref, Shape, Type};
use crate::value::{get_field, CaseValue, Fields, FunctionRef, Record, Value, FUNCTION_TYPE};
use std::fmt;
use std::sync::Arc;

/// Where a value sits, for a diagnostic. Built on the stack and printed only on failure.
#[derive(Clone, Copy)]
pub(crate) enum Path<'a> {
    Root(&'a dyn fmt::Display),
    Field(&'a Path<'a>, &'a str),
    Index(&'a Path<'a>, usize),
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Path::Root(root) => root.fmt(formatter),
            Path::Field(parent, name) => write!(formatter, "{parent}.{name}"),
            Path::Index(parent, index) => write!(formatter, "{parent}[{index}]"),
        }
    }
}

/// A name followed by a fixed word: `Counter props`, `Counter state`.
pub(crate) struct Labeled<'a>(pub &'a str, pub &'static str);

impl fmt::Display for Labeled<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.0, self.1)
    }
}

/// The record a value is, where a site needs one.
pub(crate) fn require_record<'a>(value: &'a Value, path: &dyn fmt::Display) -> Result<&'a Record> {
    match value {
        Value::Record(record) => Ok(record),
        _ => fail(
            "nx-ir-boundary-type",
            format!("Expected {path} to be an object."),
        ),
    }
}

/// The `Function` record a host supplies where a function value is expected, if `value` is one.
pub(crate) fn as_function_record(value: &Value) -> Option<FunctionRef> {
    let record = value.as_record()?;
    if record.type_name() != Some(FUNCTION_TYPE) {
        return None;
    }
    match (record.get("module"), record.get("name")) {
        (Some(Value::Str(module)), Some(Value::Str(name))) => Some(FunctionRef {
            module: Arc::clone(module),
            name: Arc::clone(name),
        }),
        _ => None,
    }
}

impl<'p> Machine<'p> {
    /// Checks that a `Function` record names a function of a linked module.
    pub(crate) fn resolve_function(
        &self,
        function: &FunctionRef,
        path: &dyn fmt::Display,
    ) -> Result<(u32, u32)> {
        let Some(module) = self.program.by_identity.get(&function.module).copied() else {
            return fail(
                "nx-ir-function-value",
                format!(
                    "{path} names function '{}' of module '{}', which the program does not link.",
                    function.name, function.module
                ),
            );
        };
        match self
            .program
            .linked(module)
            .and_then(|linked| linked.module.find(&function.name))
        {
            Some((index, declaration))
                if matches!(declaration.kind, DeclarationKind::Function(_)) =>
            {
                Ok((module, index))
            }
            _ => fail(
                "nx-ir-function-value",
                format!(
                    "{path} names function '{}', which module '{}' does not declare.",
                    function.name, function.module
                ),
            ),
        }
    }

    /// Normalizes a value to a declared type, which is where a value takes its site's occurrence.
    ///
    /// <para>At a `seq` site the value is read as its items: a single value is one item, the
    /// language's one-level lift, and no items is the empty value where the occurrence admits
    /// zero and an error where it does not. A `+` or `*` value is always a list and a `?` value
    /// that holds an item is the item itself. An exactly-one site takes exactly one value: a
    /// one-element sequence reaching it is read as its element. `object` is the one exception:
    /// it may hold a sequence, opaquely.</para>
    pub(crate) fn normalize(
        &self,
        cx: Cx,
        ty: &Type,
        value: Value,
        path: &Path<'_>,
    ) -> Result<Value> {
        if !self.within_stack() {
            return fail_limit(
                STACK_LIMIT,
                "A value nests too deeply to check within the stack an evaluation may use.",
            );
        }
        if let Type::Seq {
            item,
            may_be_empty,
            may_be_many,
        } = ty
        {
            let single;
            let items: &[Value] = match &value {
                Value::Seq(items) => items,
                other => {
                    single = [other.clone()];
                    &single
                }
            };
            if items.is_empty() {
                if *may_be_empty {
                    return Ok(Value::empty());
                }
                return fail(
                    "nx-ir-boundary-type",
                    format!("Expected {path} to hold at least one value, got the empty value."),
                );
            }
            if !may_be_many {
                return match items {
                    [only] => self.normalize(cx, item, only.clone(), &Path::Index(path, 0)),
                    _ => fail(
                        "nx-ir-boundary-type",
                        format!(
                            "Expected {path} to hold at most one value, got a sequence of {}.",
                            items.len()
                        ),
                    ),
                };
            }
            let mut normalized = Vec::with_capacity(items.len());
            for (index, element) in items.iter().enumerate() {
                normalized.push(self.normalize(
                    cx,
                    item,
                    element.clone(),
                    &Path::Index(path, index),
                )?);
            }
            return Ok(Value::seq(normalized));
        }

        let is_object = matches!(ty, Type::Primitive(Primitive::Object));
        let mut value = value;
        while let Value::Seq(items) = &value {
            if is_object {
                break;
            }
            value = match &**items {
                [only] => only.clone(),
                [] => {
                    return fail(
                        "nx-ir-boundary-type",
                        format!("Expected {path} to hold a value, got the empty value."),
                    )
                }
                more => {
                    return fail(
                        "nx-ir-boundary-type",
                        format!(
                            "Expected {path} to hold one value, got a sequence of {}.",
                            more.len()
                        ),
                    )
                }
            };
        }
        // Checking one value against the type it is to have costs one operation, charged before
        // the check: a record's fields are values of their own, and so are a sequence's items,
        // which is why a sequence type costs nothing above.
        self.charge(cx, None, 1)?;
        match ty {
            Type::Primitive(primitive) => normalize_primitive(*primitive, value, path),
            Type::UnknownPrimitive(name) => {
                fail("nx-ir-schema", format!("Unknown primitive type '{name}'."))
            }
            Type::Nominal(reference) => self.normalize_nominal(cx, reference, value, path),
            // A function value from the program is already a reference; one from a host is the
            // canonical record, resolved to the declaration it names. The checker related the
            // value's declaration to the type by name, so no parameter is re-checked here.
            Type::Function => match &value {
                Value::Function(_) => Ok(value),
                other => match as_function_record(other) {
                    Some(function) => {
                        self.resolve_function(&function, path)?;
                        Ok(Value::Function(Arc::new(function)))
                    }
                    None => fail(
                        "nx-ir-boundary-type",
                        format!("Expected {path} to be a function value."),
                    ),
                },
            },
            Type::Seq { .. } => Ok(value),
        }
    }

    fn normalize_nominal(
        &self,
        cx: Cx,
        reference: &Ref,
        value: Value,
        path: &Path<'_>,
    ) -> Result<Value> {
        let (module, index, declaration) = self
            .program
            .resolve(cx.module, reference)
            .map_err(|message| self.error(Some(cx), None, "nx-ir-reference", message))?;
        let display = &declaration.name;
        let declared = Cx {
            module,
            declaration: index,
            depth: cx.depth,
        };
        match &declaration.kind {
            DeclarationKind::Record(record) if record.update_target.is_some() => {
                // An update record has no subtypes, so a discriminator must name it exactly.
                let object = require_record(&value, path)?;
                if let Some(discriminator) = object.type_name().filter(|name| *name != &**display) {
                    return fail(
                        "nx-ir-boundary-type",
                        format!("Expected {path} to be a {display}, got '{discriminator}'."),
                    );
                }
                Ok(Value::record(
                    Some(Arc::clone(display)),
                    self.normalize_patch_fields(declared, &record.fields, &object.fields, path)?,
                ))
            }
            DeclarationKind::Record(record) => {
                let object = require_record(&value, path)?;
                // The declared type supplies the field list, so a discriminator carried by the
                // value selects nothing unless it names a derived record, which is acceptable
                // wherever its base is and is normalized against its own schema.
                if let Some(discriminator) = object.type_name().filter(|name| *name != &**display) {
                    let (sub_module, shape) =
                        self.resolve_subtype(discriminator, module, display, path)?;
                    let sub = Cx {
                        module: sub_module,
                        declaration: shape.declaration,
                        depth: cx.depth,
                    };
                    return Ok(Value::record(
                        object.type_name.clone(),
                        self.normalize_fields(
                            sub,
                            &shape.fields,
                            &object.fields,
                            &mut Vec::new(),
                            path,
                            false,
                        )?,
                    ));
                }
                // Nothing is an instance of an abstract record.
                if record.is_abstract {
                    return fail(
                        "nx-ir-boundary-type",
                        match object.type_name() {
                            None => format!(
                                "Expected {path} to be a concrete type extending {display}, got an object with no '$type' discriminator naming one."
                            ),
                            Some(_) => format!(
                                "Expected {path} to be a concrete type extending {display}, got abstract '{display}'."
                            ),
                        },
                    );
                }
                Ok(Value::record(
                    Some(Arc::clone(display)),
                    self.normalize_fields(
                        declared,
                        &record.fields,
                        &object.fields,
                        &mut Vec::new(),
                        path,
                        false,
                    )?,
                ))
            }
            DeclarationKind::Union(union) => {
                // A constant case arrives as its bare name rather than as a `$type` object.
                if let Some(text) = value.as_text() {
                    return match union.cases.iter().find(|case| case.is_constant && &*case.name == text) {
                        Some(case) => Ok(Value::Case(Arc::new(CaseValue {
                            union: Arc::clone(display),
                            case: Arc::clone(&case.name),
                        }))),
                        None => fail(
                            "nx-ir-boundary-type",
                            format!("Invalid constant union case for {path}: '{text}' is not a case of {display}."),
                        ),
                    };
                }
                let object = require_record(&value, path)?;
                let Some(type_name) = object.type_name() else {
                    return fail(
                        "nx-ir-boundary-type",
                        format!("Expected {path} to include a '$type' discriminator."),
                    );
                };
                let Some(case_name) = type_name
                    .strip_prefix(&**display)
                    .and_then(|rest| rest.strip_prefix('.'))
                else {
                    return fail(
                        "nx-ir-boundary-type",
                        format!("Expected {path} to be a {display} union case."),
                    );
                };
                let Some(case) = union.cases.iter().find(|case| &*case.name == case_name) else {
                    return fail(
                        "nx-ir-boundary-type",
                        format!("Invalid union case '{type_name}' for {path}."),
                    );
                };
                Ok(Value::record(
                    object.type_name.clone(),
                    self.normalize_fields(
                        declared,
                        &case.fields,
                        &object.fields,
                        &mut Vec::new(),
                        path,
                        false,
                    )?,
                ))
            }
            _ => Ok(value),
        }
    }

    /// Finds the shape a value's `$type` names, given that a value of `expected` was asked for.
    ///
    /// <para>The discriminator is a name, not an identity, so this can find more than one shape:
    /// two modules may each declare a `Card` extending the same base. That is reported rather
    /// than guessed at.</para>
    fn resolve_subtype(
        &self,
        discriminator: &str,
        expected_module: u32,
        expected: &str,
        path: &Path<'_>,
    ) -> Result<(u32, &'p Shape)> {
        let program = self.program;
        let mut candidates = program.shapes(discriminator).filter(|(module, shape)| {
            shape.bases.iter().any(|base| {
                &*base.name == expected
                    && program.target(*module, base.slot) == Some(expected_module)
            })
        });
        let Some((module, shape)) = candidates.next() else {
            return fail(
                "nx-ir-boundary-type",
                format!("Expected {path} to be a {expected}, got '{discriminator}'."),
            );
        };
        let others = candidates.count();
        if others > 0 {
            return fail(
                "nx-ir-boundary-type",
                format!(
                    "Ambiguous subtype at {path}: {} declarations named '{discriminator}' extend {expected}, and a '$type' discriminator cannot tell them apart.",
                    others.saturating_add(1)
                ),
            );
        }
        if shape.is_abstract {
            return fail(
                "nx-ir-boundary-type",
                format!("Expected {path} to be a concrete type extending {expected}, got abstract '{discriminator}'."),
            );
        }
        Ok((module, shape))
    }

    /// Normalizes input against a declaration's fields, binding each field's slot in `frame` as
    /// it goes so a later field's default can read an earlier field.
    ///
    /// <para>A field that is not written binds its default, or the empty value when its type
    /// admits zero, and is otherwise missing. An optional field whose value is empty, written or
    /// not, is stored as no entry at all: the canonical encoding omits it, and a read of the
    /// declared field yields the empty value. Its slot in `frame` still holds the empty value.
    /// `cx` is where the fields were declared: defaults are nodes of that module, and a nominal
    /// type resolves through that module's table.</para>
    pub(crate) fn normalize_fields(
        &self,
        cx: Cx,
        fields: &[Field],
        input: &[(Arc<str>, Value)],
        frame: &mut Frame,
        path: &dyn fmt::Display,
        require_explicit: bool,
    ) -> Result<Fields> {
        for (key, _) in input {
            if !fields.iter().any(|field| field.name == *key) {
                return fail(
                    "nx-ir-boundary-field",
                    format!("Unknown {path} field '{key}'."),
                );
            }
        }
        let root = Path::Root(path);
        let first_slot = frame.len();
        let mut output = Vec::with_capacity(fields.len());
        for (offset, field) in fields.iter().enumerate() {
            let at = Path::Field(&root, &field.name);
            let value = match (get_field(input, &field.name), field.default) {
                (Some(value), _) => self.normalize(cx, &field.ty, value.clone(), &at)?,
                (None, Some(default)) if !require_explicit => self
                    .eval(cx, frame, default)
                    .and_then(|value| self.normalize(cx, &field.ty, value, &at))
                    .map_err(|error| self.default_failed(error))?,
                _ if !field.is_required && field.ty.admits_empty() => Value::empty(),
                _ => {
                    return Err(self.error(
                        Some(cx),
                        None,
                        "nx-ir-boundary-field",
                        format!("Missing required {path} field '{}'.", field.name),
                    ))
                }
            };
            if !(field.ty.admits_empty() && value.is_empty()) {
                output.push((Arc::clone(&field.name), value.clone()));
            }
            bind(frame, first_slot.saturating_add(offset), value)?;
        }
        Ok(output)
    }

    /// Normalizes the fields of an update record: only the fields supplied, each checked against
    /// its declared type. An absent field means "unchanged", so it stays absent; a present empty
    /// value clears the field, and is accepted only where the target declares the field
    /// optional. A cleared field is stored as present and empty, so its presence carries
    /// "cleared"; the canonical encoder writes it as `null`.
    pub(crate) fn normalize_patch_fields(
        &self,
        cx: Cx,
        fields: &[Field],
        input: &[(Arc<str>, Value)],
        path: &dyn fmt::Display,
    ) -> Result<Fields> {
        for (key, _) in input {
            if !fields.iter().any(|field| field.name == *key) {
                return fail(
                    "nx-ir-boundary-field",
                    format!("Unknown {path} field '{key}'."),
                );
            }
        }
        let root = Path::Root(path);
        let mut output = Vec::new();
        for field in fields {
            let Some(value) = get_field(input, &field.name) else {
                continue;
            };
            if value.is_empty() && !field.ty.admits_empty() {
                return fail(
                    "nx-ir-boundary-type",
                    format!(
                        "Expected {path}.{} to hold a value; an update record clears a field only where the target declares it optional.",
                        field.name
                    ),
                );
            }
            output.push((
                Arc::clone(&field.name),
                self.normalize(
                    cx,
                    &field.ty,
                    value.clone(),
                    &Path::Field(&root, &field.name),
                )?,
            ));
        }
        Ok(output)
    }
}

/// The largest magnitude below which every integer is a `float64`: the range `int` is exact over.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// An integral float as the integer it is, when it is one `int` holds exactly.
pub(crate) fn integral(value: f64) -> Option<i64> {
    (value.fract() == 0.0 && value.abs() <= MAX_SAFE_INTEGER).then_some(value as i64)
}

fn normalize_primitive(primitive: Primitive, value: Value, path: &Path<'_>) -> Result<Value> {
    let not_a_number = || {
        fail(
            "nx-ir-boundary-type",
            format!("Expected {path} to be a number."),
        )
    };
    match primitive {
        // A host format such as JSON cannot spell the difference between `3` and `3.0`, so an
        // integral float is the integer at an integer site. Anything else numeric is left as it
        // is, as the TypeScript runtime leaves it.
        Primitive::Int | Primitive::Int64 => match value {
            Value::Int(_) => Ok(value),
            Value::Float(float) => Ok(integral(float).map(Value::Int).unwrap_or(value)),
            // Canonical JSON spells an integer outside JavaScript's safe range as
            // `{ "$type": "nx.int", "value": "<digits>" }`.
            Value::Record(record) if record.type_name() == Some("nx.int") => {
                match record
                    .get("value")
                    .and_then(Value::as_text)
                    .map(str::parse::<i64>)
                {
                    Some(Ok(integer)) => Ok(Value::Int(integer)),
                    _ => not_a_number(),
                }
            }
            _ => not_a_number(),
        },
        // A host cannot spell the narrow types either, so a number takes the width of its site
        // on the terms a literal written there does.
        Primitive::Int32 => {
            let (integer, shown) = match &value {
                Value::Int(integer) => (Some(*integer), integer.to_string()),
                Value::Float(float) => (integral(*float), crate::text::float64_text(*float)),
                _ => return not_a_number(),
            };
            match integer {
                None => fail(
                    "nx-ir-boundary-type",
                    format!("Expected {path} to be an int32, got {shown}."),
                ),
                Some(integer) if i32::try_from(integer).is_err() => fail(
                    "nx-ir-boundary-type",
                    format!(
                        "Expected {path} to be an int32, got {shown} (out of range for int32)."
                    ),
                ),
                Some(integer) => Ok(Value::Int(integer)),
            }
        }
        Primitive::Float64 => match value {
            Value::Float(_) => Ok(value),
            Value::Int(integer) => Ok(Value::Float(integer as f64)),
            _ => not_a_number(),
        },
        Primitive::Float32 => {
            let wide = match value {
                Value::Float(float) => float,
                Value::Int(integer) => integer as f64,
                _ => return not_a_number(),
            };
            let rounded = f64::from(wide as f32);
            if wide.fract() == 0.0 && rounded != wide {
                return fail(
                    "nx-ir-boundary-type",
                    format!(
                        "Expected {path} to be a float32, got {} (not exact as a float32).",
                        crate::text::float64_text(wide)
                    ),
                );
            }
            Ok(Value::Float(rounded))
        }
        Primitive::String => match value {
            Value::Str(_) | Value::Case(_) => Ok(value),
            _ => fail(
                "nx-ir-boundary-type",
                format!("Expected {path} to be a string."),
            ),
        },
        Primitive::Boolean => match value {
            Value::Bool(_) => Ok(value),
            _ => fail(
                "nx-ir-boundary-type",
                format!("Expected {path} to be a boolean."),
            ),
        },
        Primitive::Object => Ok(value),
    }
}

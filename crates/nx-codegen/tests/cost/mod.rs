//! What the cost tests share: the probe library, the seeded generator of host values, the cases
//! it makes, the Rust runtime's answers for a case, and the list of known findings.
//!
//! <para>The cost model says that work which is not charged does not grow with the size of a
//! value. Review found that broken several times, each time through a large or unusual value a
//! host passed. These tests look for such steps mechanically: generated host values are run
//! through a fixed library of small NX functions and components, the probes, in both IR runtimes,
//! and the count is compared between the runtimes (`cost_differential.rs`) and the work with the
//! count (`cost_allocation.rs`, and the time report in `cost_differential.rs`).</para>
//!
//! <para>The cases are generated once, here, and given to both runtimes as JSON, so both see the
//! same input. The generator is a small seeded pseudo-random one with no dependency: the same
//! seed yields the same cases on every run, and a failure names its seed.</para>

#![allow(dead_code)]

use nx_api::{build_program_artifact_from_source, ProgramBuildContext};
use nx_codegen::{emit_nx_ir, NxIrEmitOptions};
use nx_ir_runtime::{
    ComponentInit, LinkOptions, NxIrRuntimeError, PreparedModule, Program, RuntimeOptions, Usage,
};
use nx_value::NxValue;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The probe library: one function or component per operation a runtime performs on a host value.
pub const PROBE_SOURCE: &str = include_str!("probes.nx");

/// The identity the probe library is compiled under, and the entry of the program the cases run.
pub const PROBE_IDENTITY: &str = "probes.nx";

/// How often a probe repeats its step at the base scale.
pub const BASE_REPEATS: u64 = 16;

/// How much larger the second scale is: its value is this many times the size, and its step is
/// repeated this many times as often.
pub const SCALE: usize = 8;

/// The seed the blocking tests run with unless `NX_COST_SEED` names another.
pub const DEFAULT_SEED: u64 = 0x6e78_2d63_6f73_7431;

/// How many cases the blocking tests generate unless `NX_COST_CASES` names another number: set
/// from the measured time for each case in an unoptimized build so that the differential test
/// and the allocation test together take under a minute.
pub const DEFAULT_CASES: usize = 400;

/// A budget and an input limit no case reaches: what a host sets to read what a call used and
/// limit nothing.
pub const AMPLE: u64 = 1 << 40;

/// The seed of this run: `NX_COST_SEED`, in decimal or as `0x` and hexadecimal, or the default.
pub fn seed() -> u64 {
    match std::env::var("NX_COST_SEED") {
        Ok(text) => {
            let parsed = match text.strip_prefix("0x") {
                Some(hex) => u64::from_str_radix(hex, 16),
                None => text.parse(),
            };
            parsed.unwrap_or_else(|_| panic!("NX_COST_SEED is not a number: {text}"))
        }
        Err(_) => DEFAULT_SEED,
    }
}

/// The number of cases of this run: `NX_COST_CASES`, or the default.
pub fn case_count() -> usize {
    match std::env::var("NX_COST_CASES") {
        Ok(text) => text
            .parse()
            .unwrap_or_else(|_| panic!("NX_COST_CASES is not a number: {text}")),
        Err(_) => DEFAULT_CASES,
    }
}

// ------------------------------------------------------------------------------------------------
// The probe library, compiled
// ------------------------------------------------------------------------------------------------

/// The images of the probe library, by identity, with their debug sections so that a failure
/// under a budget carries its span.
pub fn probe_images() -> Vec<(String, Vec<u8>)> {
    let artifact = build_program_artifact_from_source(
        PROBE_SOURCE,
        PROBE_IDENTITY,
        &ProgramBuildContext::empty(),
    )
    .unwrap_or_else(|diagnostics| panic!("the probe library does not build: {diagnostics:#?}"));
    emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            debug: true,
        },
    )
    .unwrap_or_else(|error| panic!("the probe library does not emit: {:?}", error.diagnostics))
    .into_iter()
    .map(|image| (image.identity, image.bytes))
    .collect()
}

/// The probe library linked in the Rust runtime.
pub fn link(images: &[(String, Vec<u8>)]) -> Program {
    let modules: BTreeMap<String, PreparedModule> = images
        .iter()
        .map(|(identity, bytes)| {
            let module = PreparedModule::prepare(bytes.clone())
                .unwrap_or_else(|error| panic!("{identity}: {error}"));
            (identity.clone(), module)
        })
        .collect();
    Program::link(
        &modules[PROBE_IDENTITY],
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .unwrap_or_else(|error| panic!("the probe library does not link: {error}"))
}

// ------------------------------------------------------------------------------------------------
// The pseudo-random generator
// ------------------------------------------------------------------------------------------------

/// SplitMix64: small, fast, and the same on every platform.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// A number below `bound`, which is not zero.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    /// True `percent` times in a hundred.
    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

// ------------------------------------------------------------------------------------------------
// Host values
// ------------------------------------------------------------------------------------------------

/// What the characters of a generated string are drawn from, so that UTF-16 code units, UTF-8
/// bytes and characters all differ: an ASCII letter is one of each, `é` is one code unit and two
/// bytes, and an astral character is two code units and four bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alphabet {
    Ascii,
    TwoByte,
    Astral,
}

impl Alphabet {
    fn text(self, characters: usize) -> String {
        match self {
            Alphabet::Ascii => "a".repeat(characters),
            Alphabet::TwoByte => "é".repeat(characters),
            Alphabet::Astral => "😀".repeat(characters),
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Alphabet::Ascii => "alphabet:ascii",
            Alphabet::TwoByte => "alphabet:two-byte",
            Alphabet::Astral => "alphabet:astral",
        }
    }
}

/// What every other item of a list, or field of an object, is replaced with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gaps {
    None,
    Nulls,
    EmptyLists,
}

/// The type name of a generated object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TypeName {
    None,
    Short,
    /// As many code units long.
    Long(usize),
}

/// Which dimension of an object is eight times the size at the larger scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grow {
    Fields,
    Names,
    Values,
    TypeName,
}

/// A name a runtime gives a meaning to: a type name, or the TypeScript runtime's internal
/// marking. A host can spell every one of them, and what it puts in them is data like any other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reserved {
    /// `{ "$type": "nx.int", "value": … }`: the record canonical JSON spells a wide integer with.
    WideInteger,
    /// `{ "$type": "Function", "module": …, "name": … }`.
    Function,
    /// `{ "$type": "ActionHandler", "action": …, "token": … }`.
    ActionHandler,
    /// `{ "$type": "ActionHandlerInvocation", "token": …, "action": … }`.
    Invocation,
    /// `{ "$type": "Opt.Update", "held": … }`: a type name ending `.Update`.
    Update,
    /// `{ "$nxKind": "actionHandler", … }`.
    HandlerMarking,
    /// `{ "$nxKind": "functionReference", … }`.
    FunctionMarking,
}

pub const RESERVED: &[Reserved] = &[
    Reserved::WideInteger,
    Reserved::Function,
    Reserved::ActionHandler,
    Reserved::Invocation,
    Reserved::Update,
    Reserved::HandlerMarking,
    Reserved::FunctionMarking,
];

impl Reserved {
    pub fn tag(self) -> &'static str {
        match self {
            Reserved::WideInteger => "reserved:nx.int",
            Reserved::Function => "reserved:Function",
            Reserved::ActionHandler => "reserved:ActionHandler",
            Reserved::Invocation => "reserved:ActionHandlerInvocation",
            Reserved::Update => "reserved:.Update",
            Reserved::HandlerMarking => "reserved:$nxKind=actionHandler",
            Reserved::FunctionMarking => "reserved:$nxKind=functionReference",
        }
    }
}

/// The shape of a host value, from which the value is built at either scale.
///
/// <para>A size is what a shape has at the base scale. Building at the larger scale multiplies
/// one dimension of the shape by eight: a string's length, a list's length, and for an object the
/// dimension its `grow` names. Nesting is never scaled, so that a value stays within the 256
/// levels the Rust runtime accepts at both scales.</para>
#[derive(Clone, Debug, PartialEq)]
pub enum Spec {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// The position of the value in the list that holds it: a list of them is `0, 1, 2, …`.
    Index,
    Text {
        alphabet: Alphabet,
        length: usize,
    },
    List {
        length: usize,
        item: Box<Spec>,
        gaps: Gaps,
    },
    Object {
        type_name: TypeName,
        fields: usize,
        /// The length of each field name in code units, when that is more than its digits take.
        name_length: usize,
        /// What the names begin with, so that two objects share their names or do not.
        prefix: char,
        value: Box<Spec>,
        gaps: Gaps,
        grow: Grow,
    },
    /// `levels` lists or objects, each holding the next, around `leaf`.
    Nest {
        levels: usize,
        lists: bool,
        leaf: Box<Spec>,
    },
    /// A reserved form holding `content` code units of text, in the member the form defines or
    /// in one it does not.
    Reserved {
        form: Reserved,
        content: usize,
        defined: bool,
    },
    /// A record that fits the probe library's `Pair`: an integer and a string of `right` units.
    Pair {
        typed: bool,
        right: usize,
    },
}

/// How a case's sizes are reduced when a failure is shrunk: each size is divided by `divisor`.
#[derive(Clone, Copy, Debug)]
pub struct Sizing {
    pub scale: usize,
    pub divisor: usize,
    /// The least a size that must not be zero may be: a typed probe's list holds an item.
    pub least: usize,
}

impl Sizing {
    fn of(self, size: usize) -> usize {
        (size / self.divisor).max(self.least.min(size))
    }

    fn scaled(self, size: usize) -> usize {
        self.of(size) * self.scale
    }

    fn unscaled(self) -> Sizing {
        Sizing { scale: 1, ..self }
    }
}

impl Spec {
    /// The value this shape has under `sizing`, as JSON.
    pub fn build(&self, sizing: Sizing) -> Value {
        self.build_at(sizing, 0)
    }

    fn build_at(&self, sizing: Sizing, index: usize) -> Value {
        match self {
            Spec::Null => Value::Null,
            Spec::Bool(value) => json!(value),
            Spec::Int(value) => json!(value),
            Spec::Float(value) => json!(value),
            Spec::Index => json!(index),
            Spec::Text { alphabet, length } => json!(alphabet.text(sizing.scaled(*length))),
            Spec::List { length, item, gaps } => Value::Array(
                (0..sizing.scaled(*length))
                    .map(|index| match gaps {
                        Gaps::Nulls if index % 2 == 1 => Value::Null,
                        Gaps::EmptyLists if index % 2 == 1 => json!([]),
                        _ => item.build_at(sizing.unscaled(), index),
                    })
                    .collect(),
            ),
            Spec::Object {
                type_name,
                fields,
                name_length,
                prefix,
                value,
                gaps,
                grow,
            } => {
                let grown = |dimension: Grow, size: usize| {
                    if *grow == dimension {
                        sizing.scaled(size)
                    } else {
                        sizing.of(size)
                    }
                };
                let mut object = Map::new();
                match type_name {
                    TypeName::None => {}
                    TypeName::Short => {
                        object.insert("$type".into(), json!("T"));
                    }
                    TypeName::Long(length) => {
                        object.insert(
                            "$type".into(),
                            json!("T".repeat(grown(Grow::TypeName, *length).max(1))),
                        );
                    }
                }
                let name_length = grown(Grow::Names, *name_length);
                let inner = if *grow == Grow::Values {
                    sizing
                } else {
                    sizing.unscaled()
                };
                for index in 0..grown(Grow::Fields, *fields) {
                    let mut name = format!("{prefix}{index}");
                    while name.len() < name_length {
                        name.push('n');
                    }
                    let held = match gaps {
                        Gaps::Nulls if index % 2 == 1 => Value::Null,
                        Gaps::EmptyLists if index % 2 == 1 => json!([]),
                        _ => value.build_at(inner, index),
                    };
                    object.insert(name, held);
                }
                Value::Object(object)
            }
            Spec::Nest {
                levels,
                lists,
                leaf,
            } => (0..*levels).fold(leaf.build_at(sizing, index), |held, _| {
                if *lists {
                    json!([held])
                } else {
                    json!({ "v": held })
                }
            }),
            Spec::Reserved {
                form,
                content,
                defined,
            } => {
                let size = sizing.scaled(*content);
                // The member a form defines takes the content when `defined`, and a small value
                // of its own otherwise, with the content beside it under `extra`.
                let member = |small: &str, digit: bool| {
                    if *defined {
                        json!(if digit { "9" } else { "t" }.repeat(size.max(1)))
                    } else {
                        json!(small)
                    }
                };
                let mut object = match form {
                    Reserved::WideInteger => json!({
                        "$type": "nx.int",
                        "value": member("9007199254740993", true),
                    }),
                    Reserved::Function => json!({
                        "$type": "Function",
                        "module": member(PROBE_IDENTITY, false),
                        "name": "keep",
                    }),
                    Reserved::ActionHandler => json!({
                        "$type": "ActionHandler",
                        "action": member("Button.Tapped", false),
                        "token": "h1-1",
                    }),
                    Reserved::Invocation => json!({
                        "$type": "ActionHandlerInvocation",
                        "token": member("h9-9", false),
                        "action": { "$type": "Button.Tapped" },
                    }),
                    Reserved::Update => json!({
                        "$type": "Opt.Update",
                        "held": member("held", false),
                    }),
                    Reserved::HandlerMarking => json!({
                        "$nxKind": "actionHandler",
                        "captured": member("captured", false),
                    }),
                    Reserved::FunctionMarking => json!({
                        "$nxKind": "functionReference",
                        "declaration": member("declaration", false),
                    }),
                };
                if !*defined {
                    object["extra"] = json!("t".repeat(size));
                }
                object
            }
            Spec::Pair { typed, right } => {
                let mut object = json!({ "left": 7, "right": "r".repeat(sizing.scaled(*right)) });
                if *typed {
                    object["$type"] = json!("Pair");
                }
                object
            }
        }
    }

    /// Whether the value holds a `null` inside it, which the two runtimes return differently at
    /// `object` and so are not compared on.
    pub fn holds_null(&self) -> bool {
        match self {
            Spec::Null => true,
            Spec::List { item, gaps, .. } => *gaps == Gaps::Nulls || item.holds_null(),
            Spec::Object { value, gaps, .. } => *gaps == Gaps::Nulls || value.holds_null(),
            Spec::Nest { leaf, .. } => leaf.holds_null(),
            _ => false,
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Probes
// ------------------------------------------------------------------------------------------------

/// The type of the site a probe takes its host value at, which decides what the generator gives
/// it: a value goes to a typed probe only when it fits the type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Site {
    /// `object`: any value but a `null` as the whole argument.
    Untyped,
    /// `object` at a parameter or field marked `?`: any value, a `null` as the whole one too.
    Optional,
    /// `object`, from where the probe splices the value into a list typed `object+`: any value
    /// but a `null`, as the whole argument or as an item of a list that is the whole argument,
    /// since each such item is a whole value at `object` once it is spliced.
    Spliced,
    /// `int+`.
    Ints,
    /// `string`.
    Text,
    /// The record `Pair`.
    Pair,
    /// `object+`.
    Objects,
    /// `int`, inside JavaScript's safe range.
    Int,
    /// `float64`.
    Float,
    /// An entry of a dispatched batch.
    Entry,
}

/// How a probe is called, and so which evaluation API a case exercises.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Calling {
    /// A function by position: the values, then the repeats, or the repeats first.
    Function { repeats_first: bool },
    /// A function by parameter name, through the `Function` record that names it.
    Named,
    /// A function by parameter name, with the members of the value as arguments the function
    /// does not declare, which a call by name drops.
    NamedExtra,
    /// `Shown` initialized with the value as its `data` prop.
    Prop,
    /// `Shown` evaluated with the value as its `data` prop and no state.
    Evaluated,
    /// A descriptor of `Shown` with the value as its `data` prop.
    Descriptor,
    /// A descriptor of `Wrapped` with the value as the content a host passes: its items when it
    /// is a list, and otherwise the value as the one item.
    Content,
    /// `Form` dispatched a `Changed` that carries the value, and then a tap for each repeat.
    Payload,
    /// `Form` dispatched the value as the one entry of a batch.
    Entry,
    /// `Form` dispatched one tap whose invocation record also holds the members of the value,
    /// which an invocation does not define.
    InvocationExtra,
    /// `Form` initialized with a state that holds the value.
    StateInitial,
    /// A state of `Form` that holds the value, normalized.
    StateNormalized,
    /// A state of `Form` that holds the value, patched in another field.
    StatePatched,
    /// A state of `Form` patched with the value.
    PatchApplied,
}

pub struct Probe {
    pub name: &'static str,
    /// The function the probe calls, for the probes that call one.
    pub function: &'static str,
    pub site: Site,
    /// Whether the probe takes two values, to compare or to combine.
    pub pair: bool,
    /// Whether the probe repeats its step.
    pub repeats: bool,
    pub calling: Calling,
}

const fn function(name: &'static str, site: Site, pair: bool) -> Probe {
    Probe {
        name,
        function: name,
        site,
        pair,
        repeats: true,
        calling: Calling::Function {
            repeats_first: matches!(site, Site::Optional),
        },
    }
}

const fn component(name: &'static str, site: Site, repeats: bool, calling: Calling) -> Probe {
    Probe {
        name,
        function: "",
        site,
        pair: false,
        repeats,
        calling,
    }
}

/// Every probe: the operations `probes.nx` holds a declaration for, each with the site its value
/// arrives at. The pairing of shapes with probes follows from the sites, in `Generator::value`.
pub const PROBES: &[Probe] = &[
    function("hold", Site::Untyped, false),
    Probe {
        name: "holdByName",
        function: "hold",
        site: Site::Untyped,
        pair: false,
        repeats: true,
        calling: Calling::Named,
    },
    function("typedInts", Site::Ints, false),
    function("typedText", Site::Text, false),
    function("typedRecord", Site::Pair, false),
    function("typedObjects", Site::Objects, false),
    function("bindContent", Site::Untyped, false),
    function("compareEq", Site::Untyped, true),
    function("comparePattern", Site::Optional, false),
    function("compareDiff", Site::Untyped, true),
    function("compareSelf", Site::Untyped, false),
    function("compareUnequal", Site::Float, false),
    function("place", Site::Untyped, false),
    Probe {
        name: "twice",
        function: "twice",
        site: Site::Untyped,
        pair: false,
        repeats: false,
        calling: Calling::Function {
            repeats_first: false,
        },
    },
    function("concatText", Site::Text, false),
    function("intText", Site::Int, false),
    function("floatText", Site::Float, false),
    function("asProperty", Site::Untyped, false),
    function("asContent", Site::Spliced, false),
    function("asElement", Site::Untyped, false),
    function("captureHandler", Site::Untyped, false),
    function("compareHandlers", Site::Untyped, true),
    function("applyUpdate", Site::Untyped, false),
    function("mergeUpdates", Site::Untyped, true),
    function("present", Site::Optional, false),
    function("fallback", Site::Optional, false),
    function("member", Site::Pair, false),
    function("iterate", Site::Objects, false),
    Probe {
        name: "pass",
        function: "pass",
        site: Site::Untyped,
        pair: false,
        repeats: false,
        calling: Calling::Function {
            repeats_first: false,
        },
    },
    component("prop", Site::Untyped, false, Calling::Prop),
    component("evaluated", Site::Untyped, false, Calling::Evaluated),
    component("descriptor", Site::Untyped, false, Calling::Descriptor),
    component("descriptorContent", Site::Spliced, false, Calling::Content),
    Probe {
        name: "namedExtra",
        function: "hold",
        site: Site::Untyped,
        pair: false,
        repeats: true,
        calling: Calling::NamedExtra,
    },
    component(
        "invocationExtra",
        Site::Untyped,
        false,
        Calling::InvocationExtra,
    ),
    component("payload", Site::Untyped, true, Calling::Payload),
    component("batchEntry", Site::Entry, false, Calling::Entry),
    component("stateInitial", Site::Optional, false, Calling::StateInitial),
    component(
        "stateNormalized",
        Site::Optional,
        false,
        Calling::StateNormalized,
    ),
    component("statePatched", Site::Optional, false, Calling::StatePatched),
    component("patchApplied", Site::Optional, false, Calling::PatchApplied),
];

pub fn probe(name: &str) -> &'static Probe {
    PROBES
        .iter()
        .find(|probe| probe.name == name)
        .unwrap_or_else(|| panic!("no probe is named {name}"))
}

/// How the second value of a pair relates to the first.
#[derive(Clone, Debug, PartialEq)]
pub enum Second {
    /// The same value again.
    Same,
    /// The same value with its first item, field or character changed.
    FirstDiffers,
    /// The same value with its last item, field or character changed.
    LastDiffers,
    /// An object of the same type and width that shares none of its names with the first.
    OtherNames,
    /// The same object with one more field.
    ExtraName,
    /// A value of another shape.
    Unrelated(Spec),
}

// ------------------------------------------------------------------------------------------------
// Cases
// ------------------------------------------------------------------------------------------------

/// One generated case: a probe, the shape of its host values, and how they are sized.
#[derive(Clone)]
pub struct Case {
    pub id: usize,
    pub probe: &'static Probe,
    pub first: Spec,
    pub second: Option<Second>,
    /// What the generator drew to make it, for the histogram that shows what a run covers.
    pub tags: Vec<String>,
    /// How far the sizes are reduced from what was drawn: one, unless a failure is being shrunk.
    pub divisor: usize,
}

impl Case {
    /// The same case with every size divided by `divisor`.
    pub fn shrunk(&self, divisor: usize) -> Case {
        Case {
            divisor,
            ..self.clone()
        }
    }

    fn sizing(&self, scale: usize) -> Sizing {
        Sizing {
            scale,
            divisor: self.divisor,
            least: match self.probe.site {
                Site::Ints | Site::Objects => 1,
                _ => 0,
            },
        }
    }

    /// The host values of the case at `scale`: one, or the two of a pair.
    pub fn values(&self, scale: usize) -> Vec<Value> {
        let sizing = self.sizing(scale);
        let first = self.first.build(sizing);
        let Some(second) = &self.second else {
            return vec![first];
        };
        let second = match second {
            Second::Same => first.clone(),
            Second::FirstDiffers => changed(&first, false),
            Second::LastDiffers => changed(&first, true),
            Second::OtherNames => match &self.first {
                Spec::Object { .. } => {
                    let mut other = self.first.clone();
                    if let Spec::Object { prefix, .. } = &mut other {
                        *prefix = 'z';
                    }
                    other.build(sizing)
                }
                _ => changed(&first, false),
            },
            Second::ExtraName => {
                let mut other = first.clone();
                if let Value::Object(fields) = &mut other {
                    fields.insert("zExtra".into(), json!(1));
                }
                other
            }
            Second::Unrelated(spec) => spec.build(sizing),
        };
        vec![first, second]
    }

    /// The call the case makes at `scale`, as both runtimes read it.
    pub fn call(&self, scale: usize) -> Value {
        self.call_repeating(scale, 1)
    }

    /// The call the case makes at `scale` with its repeats multiplied by `repeats`.
    pub fn call_repeating(&self, scale: usize, repeats: u64) -> Value {
        let mut values = self.values(scale);
        let repeats = BASE_REPEATS * scale as u64 * repeats;
        let tap = |token: &str, action: Value| json!({ "$type": "ActionHandlerInvocation", "token": token, "action": action });
        match self.probe.calling {
            Calling::Function { repeats_first } => {
                let mut arguments = Vec::new();
                if repeats_first && self.probe.repeats {
                    arguments.push(json!(repeats));
                }
                arguments.append(&mut values);
                if !repeats_first && self.probe.repeats {
                    arguments.push(json!(repeats));
                }
                json!({ "kind": "function", "name": self.probe.function, "arguments": arguments })
            }
            Calling::Named => json!({
                "kind": "named",
                "name": self.probe.function,
                "arguments": { "o": values.remove(0), "n": repeats },
            }),
            Calling::NamedExtra => {
                let mut arguments = members(values.remove(0), &[]);
                arguments.insert("o".into(), json!(1));
                arguments.insert("n".into(), json!(repeats));
                json!({ "kind": "named", "name": self.probe.function, "arguments": arguments })
            }
            Calling::Prop => json!({
                "kind": "initialize", "component": "Shown", "props": { "data": values.remove(0) },
            }),
            Calling::Evaluated => json!({
                "kind": "evaluate", "component": "Shown", "props": { "data": values.remove(0) }, "state": {},
            }),
            Calling::Descriptor => json!({
                "kind": "descriptor", "component": "Shown", "props": { "data": values.remove(0) }, "content": [],
            }),
            Calling::Payload => {
                let mut batch = vec![tap(
                    "h1-1",
                    json!({ "$type": "Field.Changed", "value": values.remove(0) }),
                )];
                batch
                    .extend((0..repeats).map(|_| tap("h1-2", json!({ "$type": "Button.Tapped" }))));
                json!({ "kind": "dispatch", "component": "Form", "props": {}, "batch": batch })
            }
            Calling::Content => {
                let content = match values.remove(0) {
                    Value::Array(items) => items,
                    other => vec![other],
                };
                json!({ "kind": "descriptor", "component": "Wrapped", "props": {}, "content": content })
            }
            Calling::Entry => json!({
                "kind": "dispatch", "component": "Form", "props": {}, "batch": [values.remove(0)],
            }),
            Calling::InvocationExtra => {
                let mut entry = members(values.remove(0), &["token", "action"]);
                entry.insert("$type".into(), json!("ActionHandlerInvocation"));
                entry.insert("token".into(), json!("h1-2"));
                entry.insert("action".into(), json!({ "$type": "Button.Tapped" }));
                json!({ "kind": "dispatch", "component": "Form", "props": {}, "batch": [entry] })
            }
            Calling::StateInitial => json!({
                "kind": "initialize", "component": "Form", "props": {},
                "state": { "held": values.remove(0), "count": 0 },
            }),
            Calling::StateNormalized => json!({
                "kind": "normalize", "component": "Form",
                "state": { "held": values.remove(0), "count": 0 },
            }),
            Calling::StatePatched => json!({
                "kind": "patch", "component": "Form",
                "state": { "held": values.remove(0), "count": 0 }, "patch": { "count": 1 },
            }),
            Calling::PatchApplied => json!({
                "kind": "patch", "component": "Form",
                "state": { "count": 0 }, "patch": { "held": values.remove(0) },
            }),
        }
    }

    /// Whether the two runtimes are known to return different values for the case, because it
    /// holds a `null` inside a value at `object`: the Rust runtime reads every `null` as the
    /// empty value and writes `[]`, and the TypeScript runtime keeps it. The count and the
    /// failures of such a case are still compared.
    pub fn results_may_differ(&self) -> bool {
        self.first.holds_null()
            || matches!(&self.second, Some(Second::Unrelated(spec)) if spec.holds_null())
    }

    /// The case as the Node runner reads it.
    pub fn json(&self) -> Value {
        self.json_at(1)
    }

    /// The case with the repeats of both of its scales multiplied by `repeats`: what the time
    /// report runs, so that a step it looks for is repeated often enough to be seen beside the
    /// honest work of a call.
    pub fn json_at(&self, repeats: u64) -> Value {
        json!({
            "id": self.id,
            "probe": self.probe.name,
            "base": self.call_repeating(1, repeats),
            "scaled": self.call_repeating(SCALE, repeats),
        })
    }

    /// What a failure prints to say which case it was, short of the values themselves.
    pub fn describe(&self) -> String {
        format!(
            "case {} of probe `{}`, sizes divided by {}: {:?}{}",
            self.id,
            self.probe.name,
            self.divisor,
            self.first,
            match &self.second {
                Some(second) => format!(", second {second:?}"),
                None => String::new(),
            }
        )
    }
}

/// The members of `value` as the members of a map a probe builds around them: those of an object,
/// without its type name and without the names in `taken`, which the map defines itself; and any
/// other value as the one member `extra`.
fn members(value: Value, taken: &[&str]) -> Map<String, Value> {
    match value {
        Value::Object(mut fields) => {
            fields.remove("$type");
            for name in taken {
                fields.remove(*name);
            }
            fields
        }
        other => Map::from_iter([("extra".to_string(), other)]),
    }
}

/// `value` with its first or its last item, field or character changed, so that an equality of
/// the two differs there.
fn changed(value: &Value, last: bool) -> Value {
    let mut other = value.clone();
    match &mut other {
        Value::Array(items) if !items.is_empty() => {
            let index = if last { items.len() - 1 } else { 0 };
            items[index] = json!("changed");
        }
        Value::Object(fields) => {
            let name = if last {
                fields.keys().rfind(|name| *name != "$type").cloned()
            } else {
                fields.keys().find(|name| *name != "$type").cloned()
            };
            match name {
                Some(name) => {
                    fields.insert(name, json!("changed"));
                }
                None => {
                    fields.insert("changed".into(), json!(true));
                }
            }
        }
        Value::String(text) if !text.is_empty() => {
            let mut characters: Vec<char> = text.chars().collect();
            let index = if last { characters.len() - 1 } else { 0 };
            characters[index] = if characters[index] == 'x' { 'y' } else { 'x' };
            *text = characters.into_iter().collect();
        }
        _ => other = json!("changed"),
    }
    other
}

// ------------------------------------------------------------------------------------------------
// The generator
// ------------------------------------------------------------------------------------------------

/// The sizes the generator draws, weighted toward the boundaries the cost model has: nothing,
/// one, and one under, at and over the 64 code units that cost an operation; then a few hundred
/// and a few thousand.
pub struct Sizes {
    pub hundreds: (usize, usize),
    pub thousands: (usize, usize),
}

impl Sizes {
    /// The sizes every test draws from: a few hundred is 200 to 400, and a few thousand 1,000 to
    /// 2,000, which the larger scale makes 8,000 to 16,000. The time report multiplies both of
    /// its scales where a step it looks for needs more than this to be seen.
    pub const STANDARD: Sizes = Sizes {
        hundreds: (200, 400),
        thousands: (1000, 2000),
    };
}

/// The boundary sizes every run is expected to draw.
pub const BOUNDARY_SIZES: &[usize] = &[0, 1, 63, 64, 65];

pub struct Generator {
    rng: Rng,
    sizes: Sizes,
    tags: Vec<String>,
}

impl Generator {
    pub fn new(seed: u64, sizes: Sizes) -> Self {
        Self {
            rng: Rng::new(seed),
            sizes,
            tags: Vec::new(),
        }
    }

    fn tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// A size: a boundary three times in four, and otherwise a few hundred or a few thousand.
    fn size(&mut self) -> usize {
        let size = match self.rng.below(16) {
            0..=11 => {
                let size = self.rng.pick(BOUNDARY_SIZES);
                self.tag(format!("size:{size}"));
                return size;
            }
            12..=14 => {
                self.tag("size:hundreds");
                self.sizes.hundreds
            }
            _ => {
                self.tag("size:thousands");
                self.sizes.thousands
            }
        };
        size.0 + self.rng.below(size.1 - size.0 + 1)
    }

    /// A size that is small whatever is drawn: for the parts of a value that are not the one
    /// being made large.
    fn small(&mut self) -> usize {
        self.rng.pick(&[0, 1, 2, 3])
    }

    fn alphabet(&mut self) -> Alphabet {
        let alphabet = self
            .rng
            .pick(&[Alphabet::Ascii, Alphabet::TwoByte, Alphabet::Astral]);
        self.tag(alphabet.tag());
        alphabet
    }

    fn text(&mut self) -> Spec {
        self.tag("shape:string");
        Spec::Text {
            alphabet: self.alphabet(),
            length: self.size(),
        }
    }

    fn scalar(&mut self) -> Spec {
        self.tag("shape:scalar");
        match self.rng.below(3) {
            0 => Spec::Bool(self.rng.chance(50)),
            1 => Spec::Int(self.rng.below(2_000_001) as i64 - 1_000_000),
            _ => Spec::Float(self.rng.below(2_000_001) as f64 / 8.0 + 0.5),
        }
    }

    /// A small value to hold in a list or an object: a scalar, a short string or a small record.
    fn item(&mut self) -> Spec {
        match self.rng.below(4) {
            0 => Spec::Index,
            1 => Spec::Text {
                alphabet: self.alphabet(),
                length: self.small(),
            },
            2 => Spec::Object {
                type_name: TypeName::Short,
                fields: 2,
                name_length: 0,
                prefix: 'k',
                value: Box::new(Spec::Index),
                gaps: Gaps::None,
                grow: Grow::Fields,
            },
            _ => self.scalar(),
        }
    }

    fn gaps(&mut self, allowed: bool) -> Gaps {
        if !allowed {
            return Gaps::None;
        }
        match self.rng.below(4) {
            0 => {
                self.tag("gaps:null");
                Gaps::Nulls
            }
            1 => {
                self.tag("gaps:empty-list");
                Gaps::EmptyLists
            }
            _ => Gaps::None,
        }
    }

    fn list(&mut self, gaps: bool) -> Spec {
        self.tag("shape:list");
        Spec::List {
            length: self.size(),
            item: Box::new(self.item()),
            gaps: self.gaps(gaps),
        }
    }

    fn object(&mut self, gaps: bool) -> Spec {
        self.tag("shape:object");
        let grow = self.rng.pick(&[
            Grow::Fields,
            Grow::Fields,
            Grow::Names,
            Grow::Values,
            Grow::TypeName,
        ]);
        let type_name = match (grow, self.rng.below(3)) {
            (Grow::TypeName, _) => {
                self.tag("type-name:long");
                TypeName::Long(self.size())
            }
            (_, 0) => TypeName::None,
            (_, 1) => {
                self.tag("type-name:short");
                TypeName::Short
            }
            _ => {
                self.tag("type-name:long");
                TypeName::Long(self.rng.pick(&[63, 64, 65]))
            }
        };
        let (fields, name_length) = match grow {
            Grow::Fields => (self.size(), self.rng.pick(&[0, 0, 63, 64, 65])),
            Grow::Names => {
                self.tag("names:long");
                (1 + self.small(), self.size())
            }
            _ => (self.small(), 0),
        };
        let value = match grow {
            Grow::Values => self.text(),
            _ => self.item(),
        };
        Spec::Object {
            type_name,
            fields,
            name_length,
            prefix: 'k',
            value: Box::new(value),
            gaps: self.gaps(gaps),
            grow,
        }
    }

    fn reserved(&mut self) -> Spec {
        self.tag("shape:reserved");
        let form = self.rng.pick(RESERVED);
        self.tag(form.tag());
        let large = self.rng.chance(50);
        self.tag(if large {
            "content:large"
        } else {
            "content:small"
        });
        let defined = self.rng.chance(50);
        self.tag(if defined {
            "member:defined"
        } else {
            "member:extra"
        });
        Spec::Reserved {
            form,
            content: if large {
                self.size().max(64)
            } else {
                self.small()
            },
            defined,
        }
    }

    /// Any value a host could pass at an untyped site. Nesting is drawn once and is not scaled:
    /// at most 40 levels, well inside the 256 the Rust runtime accepts in a host value.
    fn any(&mut self, gaps: bool) -> Spec {
        match self.rng.below(12) {
            0 => self.scalar(),
            1 | 2 => self.text(),
            3..=5 => self.list(gaps),
            6..=8 => self.object(gaps),
            9 => {
                self.tag("shape:nested");
                let levels = self.rng.pick(&[1, 2, 5, 40]);
                let lists = self.rng.chance(50);
                let leaf = match self.rng.below(3) {
                    0 => self.text(),
                    1 => self.list(gaps),
                    _ => self.object(gaps),
                };
                Spec::Nest {
                    levels,
                    lists,
                    leaf: Box::new(leaf),
                }
            }
            _ => self.reserved(),
        }
    }

    /// A value for the site a probe takes it at. This is the pairing of shapes with probes: a
    /// shape reaches a probe only when both runtimes accept it there.
    ///
    /// <para>A `null` is never the whole argument at `object`, where the TypeScript runtime
    /// refuses it and the Rust runtime reads the empty value; nor an item of a list a probe
    /// splices into a list typed `object+`, where each item is such a whole value. The `nx.int`
    /// record, whatever its digits, reaches untyped sites only: at a typed integer the Rust
    /// runtime reads it as the integer and the TypeScript runtime refuses it. A typed site is
    /// given a value that fits its type, and no `null` or empty list in it, which a typed item
    /// is not.</para>
    fn value(&mut self, site: Site) -> Spec {
        match site {
            Site::Untyped | Site::Entry => self.any(true),
            Site::Spliced => match self.any(true) {
                Spec::List {
                    length,
                    item,
                    gaps: Gaps::Nulls,
                } => Spec::List {
                    length,
                    item,
                    gaps: Gaps::None,
                },
                other => other,
            },
            Site::Optional => {
                if self.rng.chance(10) {
                    self.tag("shape:null");
                    Spec::Null
                } else {
                    self.any(true)
                }
            }
            Site::Ints => {
                self.tag("shape:list");
                Spec::List {
                    length: self.size(),
                    item: Box::new(Spec::Index),
                    gaps: Gaps::None,
                }
            }
            Site::Objects => {
                self.tag("shape:list");
                Spec::List {
                    length: self.size(),
                    item: Box::new(self.item()),
                    gaps: Gaps::None,
                }
            }
            Site::Text => self.text(),
            Site::Pair => {
                self.tag("shape:object");
                Spec::Pair {
                    typed: self.rng.chance(50),
                    right: self.size(),
                }
            }
            Site::Int => {
                self.tag("shape:scalar");
                Spec::Int(self.rng.below(2_000_001) as i64 - 1_000_000)
            }
            // A float, small or so large that its square is past what a float holds.
            Site::Float => {
                self.tag("shape:scalar");
                let small = self.rng.below(2_000_001) as f64 / 8.0 + 0.5;
                Spec::Float(if self.rng.chance(50) {
                    small
                } else {
                    small * 1e200
                })
            }
        }
    }

    /// The next case. Probes are taken in turn, so that every run covers every probe evenly.
    pub fn case(&mut self, id: usize) -> Case {
        let probe = &PROBES[id % PROBES.len()];
        self.tags.clear();
        self.tag(format!("probe:{}", probe.name));
        let first = self.value(probe.site);
        let second = probe.pair.then(|| match self.rng.below(8) {
            0 | 1 => {
                self.tag("second:same");
                Second::Same
            }
            2 => {
                self.tag("second:first-differs");
                Second::FirstDiffers
            }
            3 => {
                self.tag("second:last-differs");
                Second::LastDiffers
            }
            4 | 5 => {
                self.tag("second:other-names");
                Second::OtherNames
            }
            6 => {
                self.tag("second:extra-name");
                Second::ExtraName
            }
            _ => {
                self.tag("second:unrelated");
                Second::Unrelated(self.value(probe.site))
            }
        });
        Case {
            id,
            probe,
            first,
            second,
            tags: std::mem::take(&mut self.tags),
            divisor: 1,
        }
    }
}

/// The cases of a run: `count` of them from `seed`.
pub fn generate(seed: u64, count: usize, sizes: Sizes) -> Vec<Case> {
    let mut generator = Generator::new(seed, sizes);
    (0..count).map(|id| generator.case(id)).collect()
}

// ------------------------------------------------------------------------------------------------
// Running a call in the Rust runtime
// ------------------------------------------------------------------------------------------------

fn nx_value(value: &Value) -> NxValue {
    serde_json::from_value(value.clone())
        .unwrap_or_else(|error| panic!("{value} is no canonical value: {error}"))
}

fn fields(value: &Value) -> BTreeMap<String, NxValue> {
    match nx_value(value) {
        NxValue::Record { properties, .. } => properties,
        other => panic!("expected an object, got {other:?}"),
    }
}

/// A value the runtime returned, as canonical JSON: an integer outside JavaScript's safe range
/// is the `nx.int` record, the one spelling JSON has for it.
fn canonical(value: &NxValue) -> Value {
    fn spell(value: Value) -> Value {
        const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
        match value {
            Value::Number(number) => match number.as_i64() {
                Some(int) if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&int) => {
                    json!({ "$type": "nx.int", "value": int.to_string() })
                }
                _ => Value::Number(number),
            },
            Value::Array(items) => Value::Array(items.into_iter().map(spell).collect()),
            Value::Object(fields) => Value::Object(
                fields
                    .into_iter()
                    .map(|(key, value)| (key, spell(value)))
                    .collect(),
            ),
            other => other,
        }
    }
    spell(serde_json::from_str(&value.to_json_string().expect("json")).expect("json value"))
}

fn canonical_fields(fields: &BTreeMap<String, NxValue>) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(name, value)| (name.clone(), canonical(value)))
            .collect(),
    )
}

/// A call as the Rust runtime takes it: the JSON both runtimes read, converted once, so that a
/// measurement of the call does not measure the conversion.
pub enum Call {
    Function {
        name: String,
        arguments: Vec<NxValue>,
    },
    Named {
        function: NxValue,
        arguments: BTreeMap<String, NxValue>,
    },
    Initialize {
        component: String,
        props: BTreeMap<String, NxValue>,
        state: Option<BTreeMap<String, NxValue>>,
    },
    Evaluate {
        component: String,
        props: BTreeMap<String, NxValue>,
        state: BTreeMap<String, NxValue>,
    },
    Descriptor {
        component: String,
        props: BTreeMap<String, NxValue>,
        content: Vec<NxValue>,
    },
    Dispatch {
        component: String,
        props: BTreeMap<String, NxValue>,
        batch: Vec<NxValue>,
    },
    Normalize {
        component: String,
        state: BTreeMap<String, NxValue>,
    },
    Patch {
        component: String,
        state: BTreeMap<String, NxValue>,
        patch: NxValue,
    },
}

impl Call {
    pub fn read(call: &Value) -> Call {
        let text = |key: &str| call[key].as_str().expect(key).to_string();
        let list = |key: &str| -> Vec<NxValue> {
            call[key]
                .as_array()
                .expect(key)
                .iter()
                .map(nx_value)
                .collect()
        };
        match call["kind"].as_str().expect("kind") {
            "function" => Call::Function {
                name: text("name"),
                arguments: list("arguments"),
            },
            "named" => Call::Named {
                function: nx_value(
                    &json!({ "$type": "Function", "module": PROBE_IDENTITY, "name": text("name") }),
                ),
                arguments: fields(&call["arguments"]),
            },
            "initialize" => Call::Initialize {
                component: text("component"),
                props: fields(&call["props"]),
                state: call.get("state").map(fields),
            },
            "evaluate" => Call::Evaluate {
                component: text("component"),
                props: fields(&call["props"]),
                state: fields(&call["state"]),
            },
            "descriptor" => Call::Descriptor {
                component: text("component"),
                props: fields(&call["props"]),
                content: list("content"),
            },
            "dispatch" => Call::Dispatch {
                component: text("component"),
                props: fields(&call["props"]),
                batch: list("batch"),
            },
            "normalize" => Call::Normalize {
                component: text("component"),
                state: fields(&call["state"]),
            },
            "patch" => Call::Patch {
                component: text("component"),
                state: fields(&call["state"]),
                patch: nx_value(&call["patch"]),
            },
            other => panic!("no call is of kind {other}"),
        }
    }

    /// What a dispatch runs against: the instance its component initializes to, under no limit.
    /// It is made once, outside what is measured, as a host that holds an instance would have.
    pub fn prepare(&self, program: &Program) -> Option<nx_ir_runtime::ComponentInstance> {
        match self {
            Call::Dispatch {
                component, props, ..
            } => Some(
                program
                    .initialize_component(
                        component,
                        props,
                        &ComponentInit::default(),
                        &RuntimeOptions::default(),
                    )
                    .unwrap_or_else(|error| panic!("{component} does not initialize: {error}"))
                    .instance,
            ),
            _ => None,
        }
    }

    /// Makes the call under `options` and returns what the runtime returned, unconverted.
    pub fn run(
        &self,
        program: &Program,
        instance: Option<&nx_ir_runtime::ComponentInstance>,
        options: &RuntimeOptions,
    ) -> Result<Returned, NxIrRuntimeError> {
        Ok(match self {
            Call::Function { name, arguments } => {
                Returned::Value(program.evaluate_function(name, arguments, options)?)
            }
            Call::Named {
                function,
                arguments,
            } => Returned::Value(program.call_function(function, arguments, options)?),
            Call::Initialize {
                component,
                props,
                state,
            } => {
                let init = ComponentInit {
                    state: state.as_ref(),
                    ..ComponentInit::default()
                };
                let result = program.initialize_component(component, props, &init, options)?;
                Returned::Rendered {
                    rendered: result.rendered,
                    effects: Vec::new(),
                    state: result.state,
                }
            }
            Call::Evaluate {
                component,
                props,
                state,
            } => Returned::Value(program.evaluate_component(component, props, state, options)?),
            Call::Descriptor {
                component,
                props,
                content,
            } => Returned::Value(
                program.construct_component_descriptor(component, props, content, options)?,
            ),
            Call::Dispatch { batch, .. } => {
                let instance = instance.expect("a dispatch is prepared before it is run");
                let result = program.dispatch_component_actions(instance, batch, options)?;
                Returned::Rendered {
                    rendered: result.rendered,
                    effects: result.effects,
                    state: result.state,
                }
            }
            Call::Normalize { component, state } => {
                Returned::State(program.normalize_component_state(component, state, options)?)
            }
            Call::Patch {
                component,
                state,
                patch,
            } => Returned::State(
                program.apply_component_state_patch(component, state, patch, options)?,
            ),
        })
    }
}

/// What a call returned.
pub enum Returned {
    Value(NxValue),
    State(BTreeMap<String, NxValue>),
    Rendered {
        rendered: NxValue,
        effects: Vec<NxValue>,
        state: BTreeMap<String, NxValue>,
    },
}

impl Returned {
    /// The result as canonical JSON, in the form the Node runner writes its own.
    pub fn json(&self) -> Value {
        match self {
            Returned::Value(value) => canonical(value),
            Returned::State(state) => canonical_fields(state),
            Returned::Rendered {
                rendered,
                effects,
                state,
            } => json!({
                "rendered": canonical(rendered),
                "effects": effects.iter().map(canonical).collect::<Vec<_>>(),
                "state": canonical_fields(state),
            }),
        }
    }
}

/// The options that read what a call used and limit nothing, and the report they fill.
pub fn measuring() -> (RuntimeOptions, Arc<Usage>) {
    let usage = Arc::new(Usage::new());
    let options = RuntimeOptions {
        max_operations: Some(AMPLE),
        max_input_size: Some(AMPLE),
        usage: Some(Arc::clone(&usage)),
        ..RuntimeOptions::default()
    };
    (options, usage)
}

/// The budgets below a count that a case's failures are recorded at: every one when the count is
/// at most 200, and otherwise a quarter, a half and one less.
pub fn budgets_below(operations: u64) -> Vec<u64> {
    if operations <= 200 {
        (0..operations).collect()
    } else {
        vec![operations / 4, operations / 2, operations - 1]
    }
}

/// The code a call was refused with and the argument the diagnostic names, `null` when it names
/// none, under the member `refused` names the code with.
fn refusal(refused: &str, error: &NxIrRuntimeError) -> Value {
    json!({ refused: error.code(), "argument": error.diagnostics[0].argument })
}

/// The Rust runtime's answer for one scale of a case: the code it refuses the call with and the
/// argument that names, or the operations and the input size the usage report gives.
fn measured(program: &Program, call: &Call) -> Value {
    let instance = call.prepare(program);
    let (options, usage) = measuring();
    match call.run(program, instance.as_ref(), &options) {
        Err(error) => refusal("refused", &error),
        Ok(_) => json!({ "operations": usage.operations(), "inputSize": usage.input_size() }),
    }
}

/// The Rust runtime's answer for a case, in the form the Node runner writes the TypeScript
/// runtime's: at the base scale its result with no budget, its count and input size, whether the
/// result under a budget equal to the count is the result under none, and where budgets below
/// the count stop it; at the larger scale its count and input size. A case the runtime refuses
/// with no budget has only the diagnostic's code and the argument it names, which is recorded
/// wherever a diagnostic is.
pub fn answer(program: &Program, case: &Case) -> Value {
    let base = Call::read(&case.call(1));
    let instance = base.prepare(program);
    let unlimited = match base.run(program, instance.as_ref(), &RuntimeOptions::default()) {
        Err(error) => return json!({ "base": refusal("refused", &error) }),
        Ok(returned) => returned.json(),
    };
    let (options, usage) = measuring();
    if let Err(error) = base.run(program, instance.as_ref(), &options) {
        return json!({ "base": refusal("refusedUnderAmpleLimits", &error) });
    }
    let (operations, input_size) = (usage.operations(), usage.input_size());
    let operations_used = operations.unwrap_or(0);
    let under = |budget: u64| RuntimeOptions {
        max_operations: Some(budget),
        usage: Some(Arc::clone(&usage)),
        ..RuntimeOptions::default()
    };
    let under_count = match base.run(program, instance.as_ref(), &under(operations_used)) {
        Ok(returned) => returned.json() == unlimited,
        Err(_) => false,
    };
    let failures: Vec<Value> = budgets_below(operations_used)
        .into_iter()
        .map(
            |budget| match base.run(program, instance.as_ref(), &under(budget)) {
                Ok(_) => json!({ "budget": budget, "succeeded": true }),
                Err(error) => {
                    let diagnostic = &error.diagnostics[0];
                    json!({
                        "budget": budget,
                        "code": diagnostic.code,
                        "argument": diagnostic.argument,
                        "limit": diagnostic.limit.map(|limit| limit.name),
                        "declaration": diagnostic.declaration,
                        "source": diagnostic.source.as_ref().map(|source| json!({
                            "identity": source.identity,
                            "start": source.start,
                            "end": source.end,
                        })),
                        "operations": usage.operations(),
                    })
                }
            },
        )
        .collect();
    json!({
        "base": {
            "operations": operations,
            "inputSize": input_size,
            "result": unlimited,
            "resultUnderCount": under_count,
            "failures": failures,
        },
        "scaled": measured(program, &Call::read(&case.call(SCALE))),
    })
}

// ------------------------------------------------------------------------------------------------
// Known findings
// ------------------------------------------------------------------------------------------------

/// One of the two runtimes the cases are run in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Runtime {
    Rust,
    TypeScript,
}

/// The check a known finding fails.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Check {
    /// The two runtimes disagree on the case.
    Differential,
    /// The Rust runtime's allocation is out of proportion to the case's count.
    Allocation,
    /// A runtime's time grows faster than the case's count. Only the time report shows it, and
    /// time cannot be required to reproduce, so such an entry is not run by the blocking tests.
    Time,
}

/// A step the cost tests found that is not charged, or is charged differently by the two
/// runtimes. The fix belongs to the cost model and to a change of its own; until it lands the
/// finding is listed here, so that the suite stays green and the list stays true.
///
/// <para>The blocking tests skip the generated cases a finding `covers`, run the finding's own
/// `case`, and fail when that case passes its check, with a message saying to remove the entry:
/// the change that fixes a finding is the one that deletes it. The case is written out in full
/// and does not depend on the seed, so a search under another seed neither trips the rule nor
/// hides behind it.</para>
pub struct Finding {
    /// What was found.
    pub description: &'static str,
    /// Where it is tracked: an OpenSpec change or an issue.
    pub tracked: &'static str,
    pub check: Check,
    /// The runtime a finding of the time report was seen in. The report takes a case the finding
    /// covers as known for that runtime only, so that the other runtime outgrowing its units on
    /// the same case is still reported. `None` for a finding of the blocking tests.
    pub runtime: Option<Runtime>,
    /// One concrete case that shows the step.
    pub case: fn() -> Case,
    /// Whether a generated case is one the finding covers.
    pub covers: fn(&Case) -> bool,
}

/// The probes that compare the value they are given, by name where it is a record, or write it
/// for the host: where the first finding has been seen. A probe that only holds the value, or
/// binds or checks it, is not among them, so a wide object that outgrows its units there is
/// reported.
const COMPARES_OR_WRITES: &[&str] = &[
    "compareEq",
    "compareSelf",
    "compareDiff",
    "compareHandlers",
    "mergeUpdates",
    "applyUpdate",
    "asProperty",
    "asContent",
    "asElement",
    "place",
    "twice",
    "pass",
    "prop",
    "evaluated",
    "descriptor",
    "payload",
    "stateInitial",
    "stateNormalized",
    "statePatched",
    "patchApplied",
];

/// The probes that write the value they are given once for each repeat, so that their result
/// grows with the repeats and the value together: where the large-result findings have been seen.
const WRITES_ON_EVERY_REPEAT: &[&str] = &["place", "asProperty", "asContent", "asElement"];

/// Whether `case` gives one of those probes a list of 64 items or more: a result of half a
/// million values and more at the larger scale of the time report.
fn writes_a_long_list_on_every_repeat(case: &Case) -> bool {
    WRITES_ON_EVERY_REPEAT.contains(&case.probe.name)
        && matches!(&case.first, Spec::List { length, .. } if *length >= 64)
}

/// The findings that are open. The differential test and the allocation test have found none:
/// every entry is of the time report alone, and none is a step that is not charged. Each is
/// charged in proportion to the values it touches and still takes longer for each unit at the
/// larger scale, which is all the report can see.
pub const KNOWN_FINDINGS: &[Finding] = &[
    Finding {
        description: "the TypeScript runtime's time for each field of a wide object rises about two and a half times between a few hundred fields and a few thousand, wherever it compares one by name or writes one, whose names it sorts",
        tracked: "openspec/changes/investigate-cost-time-report-findings",
        check: Check::Time,
        runtime: Some(Runtime::TypeScript),
        case: wide_objects_compared,
        covers: |case| {
            COMPARES_OR_WRITES.contains(&case.probe.name)
                && matches!(
                    &case.first,
                    Spec::Object { fields, grow: Grow::Fields, .. } if *fields >= 200
                )
        },
    },
    Finding {
        description: "the Rust runtime's time for each value written rises two to four times when a probe that writes its value on every repeat grows its result from tens of thousands of values to a million and more, each an allocation, and leaves the processor's caches",
        tracked: "openspec/changes/investigate-cost-time-report-findings",
        check: Check::Time,
        runtime: Some(Runtime::Rust),
        case: long_list_placed,
        covers: writes_a_long_list_on_every_repeat,
    },
    Finding {
        description: "the TypeScript runtime's time for each value written rises about twofold, by a little more in some runs and a little less in others, when a probe that writes its value on every repeat grows its result from thousands of values to half a million and more",
        tracked: "openspec/changes/investigate-cost-time-report-findings",
        check: Check::Time,
        runtime: Some(Runtime::TypeScript),
        case: floats_placed,
        covers: writes_a_long_list_on_every_repeat,
    },
];

/// The case of the first finding: two objects of 333 fields, each name 63 code units long, that
/// share a type name of 64 code units and none of their field names, compared.
fn wide_objects_compared() -> Case {
    written_case(
        "compareEq",
        Spec::Object {
            type_name: TypeName::Long(64),
            fields: 333,
            name_length: 63,
            prefix: 'k',
            value: Box::new(Spec::Int(149_052)),
            gaps: Gaps::Nulls,
            grow: Grow::Fields,
        },
        Some(Second::OtherNames),
    )
}

/// The case of the second finding: a list of 374 items, short astral strings and empty lists by
/// turns, placed once for each repeat.
fn long_list_placed() -> Case {
    written_case(
        "place",
        Spec::List {
            length: 374,
            item: Box::new(Spec::Text {
                alphabet: Alphabet::Astral,
                length: 3,
            }),
            gaps: Gaps::EmptyLists,
        },
        None,
    )
}

/// The case of the third finding: a list of 65 floats, placed once for each repeat.
fn floats_placed() -> Case {
    written_case(
        "place",
        Spec::List {
            length: 65,
            item: Box::new(Spec::Float(78_031.75)),
            gaps: Gaps::None,
        },
        None,
    )
}

/// A case written by hand, for a known finding or for a check of the harness itself.
pub fn written_case(probe_name: &str, first: Spec, second: Option<Second>) -> Case {
    Case {
        id: 0,
        probe: probe(probe_name),
        first,
        second,
        tags: Vec::new(),
        divisor: 1,
    }
}

/// Whether a finding of `check` covers `case`.
pub fn covered_by<'a>(findings: &'a [Finding], check: Check, case: &Case) -> Option<&'a Finding> {
    findings
        .iter()
        .find(|finding| finding.check == check && (finding.covers)(case))
}

/// Whether a finding of the time report covers `case` in `runtime`.
pub fn known_to_the_time_report<'a>(
    findings: &'a [Finding],
    case: &Case,
    runtime: Runtime,
) -> Option<&'a Finding> {
    findings.iter().find(|finding| {
        finding.check == Check::Time && finding.runtime == Some(runtime) && (finding.covers)(case)
    })
}

/// What to say when a finding's own case passes its check.
pub fn stale_finding(finding: &Finding) -> String {
    format!(
        "the known finding \"{}\" (tracked in {}) no longer reproduces: its case passes the {:?} check. Remove the entry from KNOWN_FINDINGS in crates/nx-codegen/tests/cost/mod.rs.",
        finding.description, finding.tracked, finding.check
    )
}

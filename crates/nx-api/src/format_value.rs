//! The value model owned by the NX text formatter, independent of either evaluator.
use nx_hir::Name;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Int32(i32),
    Int(i64),
    Float32(f32),
    Float(f64),
    String(SmolStr),
    Boolean(bool),
    Array(Vec<Value>),
    UnionCase {
        union: Name,
        case: SmolStr,
    },
    Record {
        type_name: Name,
        fields: FxHashMap<SmolStr, Value>,
    },
    Function {
        module: SmolStr,
        name: SmolStr,
    },
    ActionHandler,
}
impl Value {
    #[cfg(test)]
    pub(crate) fn is_empty_value(&self) -> bool {
        matches!(self, Self::Array(v) if v.is_empty())
    }
    pub(crate) fn empty() -> Self {
        Self::Array(Vec::new())
    }
}

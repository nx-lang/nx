//! The size of what a host hands one call, and the limit a host may set on it.
//!
//! <para>The measure is the one `docs/nx-ir-format.md` defines as the input size of a call: one for
//! each value, a sequence, the empty value and a `null` included, and one more for every 64 UTF-16
//! code units of a string, of a record's type name and of each field name. It is what the value
//! would cost to write for the host, so a value costs the same to hand in as to get back.</para>
//!
//! <para>The pass counts and builds nothing. It does not recurse, so no nesting of a host value
//! reaches the native stack, and it stops as soon as the count passes the limit, so what it reads
//! is bounded by the limit and not by what arrived.</para>

use crate::error::{fail_limit, Limit, Result};
use crate::eval::{utf16_len, TEXT_UNITS};
use nx_value::NxValue;
use std::collections::{btree_map, BTreeMap};

/// The name a diagnostic gives the input limit, which is the name of the TypeScript runtime's
/// option.
const INPUT_LIMIT: &str = "maxInputSize";

/// The member that holds a record's type name where a host spells a record as a map.
const TYPE_KEY: &str = "$type";

/// The most bytes of UTF-8 that 64 UTF-16 code units take: a code unit is at most three bytes.
/// Text of `n` bytes therefore costs at least `n / TEXT_BYTES`, which is known without reading it.
const TEXT_BYTES: usize = TEXT_UNITS.saturating_mul(3);

/// What is left to visit of a list or of a record the walk has entered.
enum Pending<'a> {
    Items(std::slice::Iter<'a, NxValue>),
    Fields(btree_map::Iter<'a, String, NxValue>),
}

/// The running size of a call's input, against the limit it must stay within.
pub(crate) struct Measure {
    size: u64,
    /// `u64::MAX` when the host set none.
    limit: u64,
}

impl Measure {
    pub(crate) fn new(limit: Option<u64>) -> Self {
        Self {
            size: 0,
            limit: limit.unwrap_or(u64::MAX),
        }
    }

    /// The size counted so far: the size of everything measured when the limit was not passed,
    /// and otherwise some number greater than the limit.
    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    fn exceeded(&self) -> bool {
        self.size > self.limit
    }

    /// Counts `amount` more and says whether the size is still within the limit.
    fn add(&mut self, amount: u64) -> bool {
        self.size = self.size.saturating_add(amount);
        !self.exceeded()
    }

    /// Counts the length of a string, a type name or a field name.
    fn text(&mut self, text: &str) -> bool {
        self.text_scanned_by(text, utf16_len)
    }

    /// Counts the length of `text`, which `scan` gives in UTF-16 code units.
    ///
    /// <para>Text whose byte length alone passes what is left of the limit is refused from that
    /// length and `scan` is not called, so the text scanned in a call is under `TEXT_BYTES` bytes
    /// for each unit of the limit.</para>
    fn text_scanned_by(&mut self, text: &str, scan: impl FnOnce(&str) -> usize) -> bool {
        let left = self.limit.saturating_sub(self.size);
        let least = (text.len().checked_div(TEXT_BYTES).unwrap_or(0)) as u64;
        if least > left {
            return self.add(least);
        }
        self.add(scan(text).checked_div(TEXT_UNITS).unwrap_or(0) as u64)
    }

    /// Counts one host value and everything it holds.
    pub(crate) fn value(&mut self, value: &NxValue) -> &mut Self {
        if !self.exceeded() {
            let mut pending = Vec::new();
            if self.enter(value, &mut pending) {
                self.drain(&mut pending);
            }
        }
        self
    }

    /// Counts each of `values`: the positional arguments of a call, its content, or the entries of
    /// a batch. The list itself is not a value the host supplied and is not counted.
    pub(crate) fn values(&mut self, values: &[NxValue]) -> &mut Self {
        for value in values {
            if self.exceeded() {
                break;
            }
            self.value(value);
        }
        self
    }

    /// Counts a map of named values as the one record it is passed as: props, a state, a patch or
    /// arguments by name.
    pub(crate) fn record(&mut self, fields: &BTreeMap<String, NxValue>) -> &mut Self {
        if !self.exceeded() && self.add(1) {
            self.drain(&mut vec![Pending::Fields(fields.iter())]);
        }
        self
    }

    /// Counts `value` itself and, when it holds others, leaves them on `pending`.
    fn enter<'a>(&mut self, value: &'a NxValue, pending: &mut Vec<Pending<'a>>) -> bool {
        if !self.add(1) {
            return false;
        }
        match value {
            NxValue::String(text) => self.text(text),
            NxValue::Array(items) => {
                pending.push(Pending::Items(items.iter()));
                true
            }
            NxValue::Record {
                type_name,
                properties,
            } => {
                if let Some(type_name) = type_name {
                    if !self.text(type_name) {
                        return false;
                    }
                }
                pending.push(Pending::Fields(properties.iter()));
                true
            }
            _ => true,
        }
    }

    /// Visits everything `pending` holds, depth first, until it is empty or the limit is passed.
    /// Every entry of `pending` was counted as a value before it was pushed, so the stack is no
    /// deeper than the limit.
    fn drain<'a>(&mut self, pending: &mut Vec<Pending<'a>>) {
        while let Some(top) = pending.last_mut() {
            let next = match top {
                Pending::Items(items) => items.next(),
                Pending::Fields(fields) => match fields.next() {
                    // A `$type` member that holds a string is the record's type name and counts
                    // its length alone; holding anything else it is a field like any other.
                    Some((name, NxValue::String(type_name))) if name == TYPE_KEY => {
                        if !self.text(type_name) {
                            return;
                        }
                        continue;
                    }
                    Some((name, value)) => {
                        if !self.text(name) {
                            return;
                        }
                        Some(value)
                    }
                    None => None,
                },
            };
            match next {
                Some(value) => {
                    if !self.enter(value, pending) {
                        return;
                    }
                }
                None => {
                    pending.pop();
                }
            }
        }
    }

    /// The size of the input, or the failure of a call whose input is larger than its limit.
    pub(crate) fn finish(&self) -> Result<u64> {
        if self.exceeded() {
            let limit = self.limit;
            return fail_limit(
                Limit {
                    name: INPUT_LIMIT,
                    value: Some(limit),
                },
                format!("The call's input is larger than its limit of {limit}."),
            );
        }
        Ok(self.size)
    }
}

/// The size of one host value, as `docs/nx-ir-format.md` defines the input size of a call and as
/// [`RuntimeOptions::max_input_size`](crate::RuntimeOptions::max_input_size) measures it.
///
/// <para>With a `limit`, measuring stops as soon as the size passes it and the result is some
/// number greater than the limit: the value is too large, and how large is not found out. A host
/// uses this to hold one part of what it passes to a number of its own before it calls.</para>
///
/// <para>A value measured alone has the size it adds to a call that is given it as an argument, a
/// content item, a batch entry or a patch. A list of arguments, of content or of batch entries
/// measured as one value is one more than its entries add to a call, since a call counts the
/// entries and not the list.</para>
pub fn input_size(value: &NxValue, limit: Option<u64>) -> u64 {
    Measure::new(limit).value(value).size()
}

/// The size of a map of named values measured as the one record a call measures it as: the form
/// props, a state and arguments by name are passed in. See [`input_size`].
pub fn record_input_size(fields: &BTreeMap<String, NxValue>, limit: Option<u64>) -> u64 {
    Measure::new(limit).record(fields).size()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(count: i64) -> NxValue {
        NxValue::Array((0..count).map(NxValue::Int).collect())
    }

    fn object(fields: &[(&str, NxValue)]) -> NxValue {
        NxValue::Record {
            type_name: None,
            properties: map(fields),
        }
    }

    fn map(fields: &[(&str, NxValue)]) -> BTreeMap<String, NxValue> {
        fields
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect()
    }

    fn text(length: usize) -> NxValue {
        NxValue::String("a".repeat(length))
    }

    /// Asserts that `value` has the size `size`: with no limit, under a limit equal to it, and
    /// refused under one less.
    #[track_caller]
    fn assert_size(value: &NxValue, size: u64) {
        assert_eq!(input_size(value, None), size);
        assert_eq!(Measure::new(Some(size)).value(value).finish(), Ok(size));
        let error = Measure::new(Some(size - 1))
            .value(value)
            .finish()
            .unwrap_err();
        let diagnostic = &error.diagnostics[0];
        assert_eq!(diagnostic.code, "nx-ir-resource-limit");
        assert_eq!(
            diagnostic.limit,
            Some(Limit {
                name: "maxInputSize",
                value: Some(size - 1),
            })
        );
        assert_eq!(
            (&diagnostic.declaration, &diagnostic.source),
            (&None, &None)
        );
        assert!(input_size(value, Some(size - 1)) > size - 1);
    }

    #[test]
    fn a_value_is_one_and_text_costs_its_length() {
        for scalar in [
            NxValue::Null,
            NxValue::Bool(true),
            NxValue::Int32(7),
            NxValue::Int(i64::MAX),
            NxValue::Float32(0.5),
            NxValue::Float(0.5),
            NxValue::Array(Vec::new()),
            text(63),
        ] {
            assert_size(&scalar, 1);
        }
        assert_size(&text(64), 2);
        assert_size(&text(6400), 101);
        // Length is in UTF-16 code units: `é` is two bytes and one unit, and an astral character
        // four bytes and two.
        assert_size(&NxValue::String("é".repeat(64)), 2);
        assert_size(&NxValue::String("é".repeat(63)), 1);
        assert_size(&NxValue::String("😀".repeat(32)), 2);
        assert_size(&NxValue::String("😀".repeat(31)), 1);
    }

    #[test]
    fn a_list_and_a_record_are_their_values_and_themselves() {
        assert_size(&ints(1000), 1001);
        assert_size(
            &object(&[("title", text(4)), ("count", NxValue::Int(3))]),
            3,
        );
        // Nested values, empty ones and `null`s each take a place.
        assert_size(
            &NxValue::Array(vec![
                NxValue::Null,
                NxValue::Array(Vec::new()),
                object(&[]),
                NxValue::Array(vec![NxValue::Null, NxValue::Null]),
            ]),
            7,
        );
        // A name costs its length: the object, 16,384 for a megabyte name, and the number.
        let named = object(&[(&"k".repeat(1 << 20), NxValue::Int(1))]);
        assert_size(&named, 16_386);
        // And so does a type name.
        let typed = NxValue::Record {
            type_name: Some("T".repeat(128)),
            properties: BTreeMap::new(),
        };
        assert_size(&typed, 3);
    }

    #[test]
    fn a_map_is_one_record_and_a_type_key_holding_a_string_is_its_type_name() {
        let props = map(&[("title", text(4)), ("count", NxValue::Int(3))]);
        assert_eq!(record_input_size(&props, None), 3);
        assert_eq!(record_input_size(&BTreeMap::new(), None), 1);
        assert_eq!(Measure::new(Some(3)).record(&props).finish(), Ok(3));
        assert!(Measure::new(Some(2)).record(&props).finish().is_err());
        assert!(record_input_size(&props, Some(2)) > 2);

        // The type name counts its length alone, in a map and in a record's properties.
        let typed = map(&[("$type", text(4)), ("title", text(4))]);
        assert_eq!(record_input_size(&typed, None), 2);
        let long = map(&[("$type", text(640)), ("title", text(4))]);
        assert_eq!(record_input_size(&long, None), 12);
        assert_size(&object(&[("$type", text(640)), ("title", text(4))]), 12);
        // Holding anything else, `$type` is a field.
        let odd = map(&[("$type", NxValue::Int(1)), ("title", text(4))]);
        assert_eq!(record_input_size(&odd, None), 3);
    }

    #[test]
    fn the_json_form_of_a_wide_integer_is_the_record_it_is() {
        let wide = |digits: &str| NxValue::Record {
            type_name: Some("nx.int".to_string()),
            properties: map(&[("value", NxValue::String(digits.to_string()))]),
        };
        assert_size(&wide("9007199254740993"), 2);
        assert_size(
            &NxValue::Array(vec![wide("9007199254740993"), wide("9007199254740993")]),
            5,
        );
        assert_size(&NxValue::Int(9_007_199_254_740_993), 1);
        assert_size(&wide(&"9".repeat(1 << 20)), 16_386);
    }

    #[test]
    fn several_values_are_measured_together_and_a_list_of_them_is_not_one() {
        let args = [ints(10), text(64), NxValue::Null];
        assert_eq!(Measure::new(None).values(&args).size(), 14);
        assert_eq!(input_size(&NxValue::Array(args.to_vec()), None), 15);
        assert_eq!(Measure::new(Some(14)).values(&args).finish(), Ok(14));
        // The limit is passed in the second value and the third is not reached.
        assert!(Measure::new(Some(12)).values(&args).finish().is_err());
        // What follows a value that passed the limit changes nothing.
        let mut measure = Measure::new(Some(5));
        measure.value(&ints(10));
        let refused = measure.size();
        measure.value(&ints(10)).record(&BTreeMap::new());
        assert_eq!(measure.size(), refused);
    }

    #[test]
    fn a_deeply_nested_value_is_measured_without_recursion() {
        // No deeper than this: dropping a much deeper `NxValue` overflows the stack by itself.
        let mut value = NxValue::Int(1);
        for _ in 0..2000 {
            value = NxValue::Array(vec![value]);
        }
        assert_eq!(input_size(&value, None), 2001);
        assert!(Measure::new(Some(1000)).value(&value).finish().is_err());
        assert_eq!(input_size(&value, Some(1000)), 1001);
    }

    #[test]
    fn measuring_stops_at_the_limit() {
        // A million items under a limit of ten: the eleventh value read is the last.
        assert_eq!(input_size(&ints(1_000_000), Some(10)), 11);
    }

    #[test]
    fn text_far_longer_than_the_limit_allows_is_refused_without_being_read() {
        let long = "a".repeat(64 << 20);
        let mut measure = Measure::new(Some(1000));
        assert!(!measure.text_scanned_by(&long, |_| panic!("the text was read")));
        assert!(measure.size() > 1000);

        // Text the limit might cover is read, and only then: 192 bytes for each unit left.
        let mut scanned = 0;
        let mut measure = Measure::new(Some(1000));
        let short = "a".repeat(1000 * 192 + 191);
        assert!(!measure.text_scanned_by(&short, |text| {
            scanned += 1;
            utf16_len(text)
        }));
        assert_eq!(scanned, 1);
        assert!(!Measure::new(Some(1000))
            .text_scanned_by(&"a".repeat(1001 * 192), |_| panic!("the text was read")));
    }
}

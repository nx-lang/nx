use crate::{color, NxValue};

#[test]
fn standard_color_reference_corpus() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("color-cases.json")).unwrap();
    for (index, case) in corpus["cases"].as_array().unwrap().iter().enumerate() {
        let name = case["op"].as_str().unwrap();
        let op = color::NAMES
            .iter()
            .position(|entry| *entry == name)
            .unwrap() as u32;
        let args: Vec<NxValue> = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(
                |value| match value.get("color").and_then(serde_json::Value::as_str) {
                    Some(text) => color::parse(text).unwrap(),
                    None => serde_json::from_value(value.clone()).unwrap(),
                },
            )
            .collect();
        let result = color::evaluate(op, &args);
        if let Some(code) = case["error"].as_str() {
            assert_eq!(result.unwrap_err().code, code, "case {index}: {name}");
            continue;
        }
        let actual = result.unwrap_or_else(|error| panic!("case {index}: {name}: {error:?}"));
        let expected = &case["expected"];
        if case["exact"] == true {
            let channels = color::channels(&actual).unwrap();
            for (channel, expected) in channels.iter().zip(expected.as_array().unwrap()) {
                assert_eq!(
                    (*channel as f32).to_bits(),
                    (expected.as_f64().unwrap() as f32).to_bits(),
                    "case {index}: {name}"
                );
            }
            continue;
        }
        let actual = if expected.is_array() {
            serde_json::json!(color::channels(&actual).unwrap())
        } else {
            serde_json::to_value(actual).unwrap()
        };
        fn compare(actual: &serde_json::Value, expected: &serde_json::Value, context: &str) {
            match expected {
                serde_json::Value::Number(number) => assert!(
                    (actual.as_f64().unwrap() - number.as_f64().unwrap()).abs() <= 2e-6,
                    "{context}: {actual} vs {expected}"
                ),
                serde_json::Value::Array(items) => {
                    for (i, item) in items.iter().enumerate() {
                        compare(&actual[i], item, context);
                    }
                }
                serde_json::Value::Object(fields) => {
                    for (key, value) in fields {
                        compare(&actual[key], value, context);
                    }
                }
                _ => assert_eq!(actual, expected, "{context}"),
            }
        }
        compare(&actual, expected, &format!("case {index}: {name}"));
    }
}

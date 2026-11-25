use super::*;

fn example() -> (Temp, Plugin) {
    let dir = Temp::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/sort-json-keys");
    install(&dir.0, &source).unwrap();
    let mut catalog = load(&dir.0);
    assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
    (dir, catalog.plugins.remove(0))
}

fn run(plugin: &Plugin, text: &str, ending: &str) -> Result<String, Error> {
    plugin.run(input(text, ending, "sort-keys"), Arc::default())
}

#[test]
fn sorts_nested_objects_without_reordering_arrays() {
    let (_dir, plugin) = example();
    let before = r#"{"z":[3,{"b":false,"a":null},1],"a":{"z":[],"a":{}},"A":true}"#;
    let after = "{\n  \"A\": true,\n  \"a\": {\n    \"a\": {},\n    \"z\": []\n  },\n  \"z\": [\n    3,\n    {\n      \"a\": null,\n      \"b\": false\n    },\n    1\n  ]\n}";
    assert_eq!(run(&plugin, before, "\n").unwrap(), after);
    assert_eq!(run(&plugin, after, "\n").unwrap(), after);
}

#[test]
fn sorts_decoded_unicode_keys_and_preserves_string_and_number_tokens() {
    let (_dir, plugin) = example();
    // The escaped key sorts by "a"; escaped supplementary characters must also round-trip.
    let before = r#"{"\uD83D\uDE00":"😀","中":"\"{}[],\\\/\b\f\n\r\t","z":123456789012345678901234567890,"\u0061":1.2300000000000000000001e+1000,"A":-0,"é":-3.0E-99}"#;
    let after = concat!(
        "{\n",
        "  \"A\": -0,\n",
        "  \"\\u0061\": 1.2300000000000000000001e+1000,\n",
        "  \"z\": 123456789012345678901234567890,\n",
        "  \"é\": -3.0E-99,\n",
        "  \"中\": \"\\\"{}[],\\\\\\/\\b\\f\\n\\r\\t\",\n",
        "  \"\\uD83D\\uDE00\": \"😀\"\n",
        "}"
    );
    assert_eq!(run(&plugin, before, "\n").unwrap(), after);
    for value in [
        "null",
        "false",
        "true",
        "-0",
        "1e+999",
        "[]",
        "{}",
        r#""你好""#,
    ] {
        assert_eq!(run(&plugin, value, "\n").unwrap(), value);
    }
    assert_eq!(run(&plugin, "[2,1]", "\n").unwrap(), "[\n  2,\n  1\n]");
}

#[test]
fn preserves_line_endings_and_supports_custom_indentation() {
    let (_dir, mut plugin) = example();
    for ending in ["\n", "\r\n", "\r"] {
        for trailing in ["", ending] {
            let before = format!("{{\"b\":2,\"a\":1}}{trailing}");
            let after = format!("{{{ending}  \"a\": 1,{ending}  \"b\": 2{ending}}}{trailing}");
            assert_eq!(run(&plugin, &before, ending).unwrap(), after);
        }
    }
    plugin.manifest.commands[0]
        .options
        .insert("indent".into(), 4.into());
    assert_eq!(
        run(&plugin, "{\"a\":1}", "\n").unwrap(),
        "{\n    \"a\": 1\n}"
    );
    for bad in [0.into(), 9.into(), true.into(), "2".into()] {
        plugin.manifest.commands[0]
            .options
            .insert("indent".into(), bad);
        assert!(run(&plugin, "{}", "\n").is_err());
    }
}

#[test]
fn rejects_invalid_json_and_duplicate_decoded_keys() {
    let (_dir, plugin) = example();
    for text in [
        "",
        " ",
        "{",
        "[",
        "{a:1}",
        "{\"a\" 1}",
        "{\"a\":}",
        "[1,]",
        "{\"a\":1,}",
        "[1 2]",
        "[1,,2]",
        "true false",
        "nullx",
        "NaN",
        "Infinity",
        "+1",
        "01",
        "-01",
        "-",
        "1.",
        ".1",
        "1e",
        "1e+",
        "// comment\n{}",
        "{ /* comment */ }",
        "\"unterminated",
        "\"line\nbreak\"",
        r#""\x20""#,
        r#""\uZZZZ""#,
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uD800\u0041""#,
        r#"{"a":1,"a":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"😀":1,"\uD83D\uDE00":2}"#,
    ] {
        assert!(
            run(&plugin, text, "\n").is_err(),
            "accepted invalid JSON: {text:?}"
        );
    }
}

#[test]
fn handles_moderate_documents_and_bounds_recursion() {
    let (_dir, plugin) = example();
    let members: Vec<_> = (0..1000)
        .rev()
        .map(|i| format!("\"key{i:04}\":{i}"))
        .collect();
    let result = run(&plugin, &format!("{{{}}}", members.join(",")), "\n").unwrap();
    assert!(result.starts_with("{\n  \"key0000\": 0,"));
    assert!(result.ends_with("\"key0999\": 999\n}"));
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 1000);
    let nested = format!("{}0{}", "[".repeat(24), "]".repeat(24));
    assert!(run(&plugin, &nested, "\n").is_ok());
    let too_deep = format!("{}0{}", "[".repeat(26), "]".repeat(26));
    assert!(run(&plugin, &too_deep, "\n").is_err());
}

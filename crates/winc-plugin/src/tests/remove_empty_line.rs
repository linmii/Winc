use super::*;

#[test]
fn removes_blank_lines_and_preserves_text_and_line_endings() {
    let dir = Temp::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-empty-line");
    install(&dir.0, &source).unwrap();
    let catalog = load(&dir.0);
    assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
    let plugin = &catalog.plugins[0];
    for ending in ["\n", "\r\n", "\r"] {
        for (before, after) in [
            ("", ""),
            ("\n\n", ""),
            (" \t\n\t\n\u{3000}\n\u{a0}", ""),
            ("\n  甲 \n\t\n\t乙\t\n\n", "  甲 \n\t乙\t\n"),
            ("\n甲\n \n乙", "甲\n乙"),
            ("甲\n \t", "甲"),
            ("甲\n甲\n", "甲\n甲\n"),
            ("  甲 \n\t乙\t", "  甲 \n\t乙\t"),
        ] {
            let before = before.replace('\n', ending);
            let after = after.replace('\n', ending);
            let run = |text: &str| {
                plugin
                    .run(input(text, ending, "remove-empty-line"), Arc::default())
                    .unwrap()
            };
            assert_eq!(run(&before), after, "input: {before:?}");
            assert_eq!(
                run(&after),
                after,
                "a second run must leave the result unchanged"
            );
        }
    }
    // Mixed input uses the document's configured line ending.
    assert_eq!(
        plugin
            .run(
                input("\r\n甲\r\n \n乙\r\t\r", "\r\n", "remove-empty-line"),
                Arc::default()
            )
            .unwrap(),
        "甲\r\n乙\r\n"
    );
}

#[test]
fn remove_empty_line_zip_can_be_imported_and_run() {
    let dir = Temp::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-empty-line");
    let archive = dir.0.join("remove-empty-line.zip");
    zip_file(
        &archive,
        &[
            (
                "plugin.json",
                &fs::read_to_string(source.join("plugin.json")).unwrap(),
            ),
            (
                "main.rhai",
                &fs::read_to_string(source.join("main.rhai")).unwrap(),
            ),
        ],
    );
    let root = dir.0.join("installed");
    assert_eq!(install(&root, &archive).unwrap(), "winc.remove-empty-line");
    assert_eq!(
        load(&root).plugins[0]
            .run(
                input("\n甲\n \n", "\n", "remove-empty-line"),
                Arc::default()
            )
            .unwrap(),
        "甲\n"
    );
}

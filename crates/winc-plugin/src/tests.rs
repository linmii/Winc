use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

mod json_sort;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "winc-plugin-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn package(&self, script: &str) -> PathBuf {
        let path = self.0.join("source");
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("plugin.json"),
            r#"{
          "id":"test.transform", "name":"Test", "version":"1", "api_version":1,
          "commands":[{"id":"run", "title":"Transform", "script":"main.rhai"}]
        }"#,
        )
        .unwrap();
        fs::write(path.join("main.rhai"), script).unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn input(text: &str, ending: &str, command: &str) -> Input {
    Input {
        text: text.into(),
        line_ending: ending.into(),
        command: command.into(),
    }
}

#[test]
fn example_deduplicates_unicode_and_preserves_line_endings_and_empty_lines() {
    let dir = Temp::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-duplicate-lines");
    install(&dir.0, &source).unwrap();
    let catalog = load(&dir.0);
    assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
    let p = &catalog.plugins[0];
    for (before, after, ending) in [
        ("甲\n乙\n甲\n", "甲\n乙\n", "\n"),
        ("a\r\nb\r\na", "a\r\nb", "\r\n"),
        ("a\rb\ra\r", "a\rb\r", "\r"),
        ("\n\na\na\n", "\na\n", "\n"),
        ("", "", "\n"),
        // 只有空行时返回空文本，否则一个末尾换行仍会显示成两行。
        ("\n", "", "\n"),
        ("\n\n\n", "", "\n"),
        ("\r\n\r\n\r\n", "", "\r\n"),
        ("\r\r\r", "", "\r"),
        ("\r\n\n\r", "", "\r\n"),
        // 普通文本和含空格的行继续保留原来的末尾换行语义。
        ("a\n\n\n", "a\n\n", "\n"),
        (" \n \n", " \n", "\n"),
        ("a\nA\n a ", "a\nA\n a ", "\n"),
    ] {
        assert_eq!(
            p.run(input(before, ending, "remove-duplicates"), Arc::default())
                .unwrap(),
            after
        );
    }
    let mut p = p.clone();
    p.manifest.commands[0]
        .options
        .insert("ignore_case".into(), true.into());
    p.manifest.commands[0]
        .options
        .insert("trim_whitespace".into(), true.into());
    assert_eq!(
        p.run(
            input(" A \na\nB\nb", "\n", "remove-duplicates"),
            Arc::default()
        )
        .unwrap(),
        " A \nB"
    );
}

#[test]
fn installing_reloading_disabling_and_uninstalling_are_persistent() {
    let dir = Temp::new();
    let source = dir.package("text.to_upper()");
    let root = dir.0.join("installed");
    let id = install(&root, &source.join("plugin.json")).unwrap();
    assert!(install(&root, &source).is_err());
    assert_eq!(
        load(&root).plugins[0]
            .run(input("abc", "\n", "run"), Arc::default())
            .unwrap(),
        "ABC"
    );
    set_enabled(&root, &id, false).unwrap();
    let p = &load(&root).plugins[0];
    assert!(!p.enabled);
    assert!(p.run(input("abc", "\n", "run"), Arc::default()).is_err());
    set_enabled(&root, &id, true).unwrap();
    assert!(load(&root).plugins[0].enabled);
    fs::write(root.join(&id).join("main.rhai"), "text + \"!\"").unwrap();
    assert_eq!(
        load(&root).plugins[0]
            .run(input("a", "\n", "run"), Arc::default())
            .unwrap(),
        "a!"
    );
    uninstall(&root, &id).unwrap();
    assert!(load(&root).plugins.is_empty());
    assert!(source.is_dir());
}

#[test]
fn invalid_packages_never_leave_a_partial_installation() {
    let dir = Temp::new();
    let source = dir.package("let = ;");
    let root = dir.0.join("installed");
    assert!(install(&root, &source).is_err());
    assert!(!root.join("test.transform").exists());
    fs::write(source.join("main.rhai"), "text").unwrap();
    let manifest = fs::read_to_string(source.join("plugin.json")).unwrap();
    for bad in [
        manifest.replace("main.rhai", "../outside.rhai"),
        manifest.replace("api_version\":1", "api_version\":2"),
        manifest.replace("test.transform", "../escape"),
    ] {
        fs::write(source.join("plugin.json"), bad).unwrap();
        assert!(install(&root, &source).is_err());
    }
}

#[test]
fn a_broken_installed_script_remains_manageable_and_can_be_repaired() {
    let dir = Temp::new();
    let source = dir.package("text");
    let root = dir.0.join("installed");
    let id = install(&root, &source).unwrap();
    let script = root.join(&id).join("main.rhai");
    fs::write(&script, "let = ;").unwrap();
    let catalog = load(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0].error.is_some());
    assert_eq!(catalog.errors.len(), 1);
    set_enabled(&root, &id, false).unwrap();
    assert!(!load(&root).plugins[0].enabled);
    fs::write(script, "text").unwrap();
    assert!(load(&root).plugins[0].error.is_none());
    uninstall(&root, &id).unwrap();
    assert!(load(&root).plugins.is_empty());
}

#[test]
fn grain_limits_loops_recursion_output_and_invalid_return_values() {
    for source in [
        "loop {}",
        "fn recurse() { recurse() } recurse()",
        "42",
        "let x = \"a\"; loop { x += x; }",
        "eval(\"text\")",
        "import \"file\" as x;",
    ] {
        let dir = Temp::new();
        let source = dir.package(source);
        let root = dir.0.join("installed");
        if install(&root, &source).is_ok() {
            assert!(
                load(&root).plugins[0]
                    .run(input("a", "\n", "run"), Arc::default())
                    .is_err()
            );
        }
    }
    let dir = Temp::new();
    let source = dir.package("loop {}");
    let root = dir.0.join("installed");
    install(&root, &source).unwrap();
    let p = &load(&root).plugins[0];
    assert!(
        p.run(input("a", "\n", "run"), Arc::new(AtomicBool::new(true)))
            .is_err()
    );
    assert!(
        p.run(
            input(&"a".repeat(MAX_TEXT_BYTES + 1), "\n", "run"),
            Arc::default()
        )
        .is_err()
    );
}

fn zip_file(path: &Path, entries: &[(&str, &str)]) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, contents) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn zip_import_validates_paths_and_copies_only_declared_files() {
    let dir = Temp::new();
    let source = dir.package("text");
    let manifest = fs::read_to_string(source.join("plugin.json")).unwrap();
    let archive = dir.0.join("plugin.zip");
    zip_file(
        &archive,
        &[
            ("plugin.json", &manifest),
            ("main.rhai", "text"),
            ("unused.txt", "ignored"),
        ],
    );
    let root = dir.0.join("installed");
    install(&root, &archive).unwrap();
    assert!(!root.join("test.transform/unused.txt").exists());
    for bad in [
        "../escaped.txt",
        "C:/escaped.txt",
        "nested/../../escaped.txt",
        "file:stream",
    ] {
        zip_file(
            &archive,
            &[
                ("plugin.json", &manifest),
                ("main.rhai", "text"),
                (bad, "bad"),
            ],
        );
        assert!(install(&dir.0.join("other"), &archive).is_err());
    }
    assert!(!dir.0.join("escaped.txt").exists());
}

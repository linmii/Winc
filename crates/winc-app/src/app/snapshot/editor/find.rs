//! 文件内查找/替换的正则偏好：写入状态后，在新窗口与下次启动时恢复。

use super::*;

#[test]
fn search_regex_survives_reopening_and_restores_both_choices() {
    let dir = sample_folder("find-regex-state");
    let path = dir.join("numbers.txt");
    let state_path = dir.join("state.json");
    std::fs::write(&path, "one1 two22\n").unwrap();
    let mut app = app_with(&path, false);
    app.store.state_path = Some(state_path.clone());
    app.open_search(false);
    assert!(!app.bars.search.regex);

    // 另一个窗口先保存的状态也应保留；当前窗口只写这次切换的正则选项。
    let other = State::parse(r#"{"searchTree": true}"#);
    other.save_to(&state_path).unwrap();

    for regex in [true, false] {
        app.press_search_button(SearchButton::Regex);
        assert_eq!(app.bars.search.regex, regex);
        let saved = State::load_from(&state_path);
        assert_eq!(saved.search_regex, regex);
        assert!(saved.search_tree);

        app.close_search();
        app.open_search(true);
        assert!(app.bars.find.replace_open);
        assert_eq!(app.bars.search.regex, regex);

        // 与启动/新建窗口一样，从磁盘加载偏好，而不沿用旧 App 的内存。
        let mut restored = blank_app(false);
        restored.state = State::load_from(&state_path);
        restored.restore_search_state();
        restored.store.state_path = Some(state_path.clone());
        restored.open_path(&path);
        restored.open_search(false);
        assert_eq!(restored.bars.search.regex, regex);
        restored.edit_field(|field| {
            field.set_text(r"\d+");
            true
        });
        restored.advance_search();
        assert_eq!(
            restored.bars.search.matches().len(),
            if regex { 2 } else { 0 }
        );

        // 切换到替换后仍使用恢复的选项，正则匹配才会改动这两个数字。
        restored.press_search_button(SearchButton::Expand);
        assert!(restored.bars.find.replace_open);
        restored.focus = Focus::Replace;
        restored.edit_field(|field| field.insert("#"));
        restored.replace_all();
        let doc = restored.editor.active().unwrap();
        assert_eq!(
            doc.text_range(0, doc.text().end_pos()).unwrap(),
            if regex { "one# two#\n" } else { "one1 two22\n" }
        );
        app = restored;
    }
    drop(app);
    std::fs::remove_dir_all(&dir).unwrap();
}

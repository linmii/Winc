//! End-to-end command registration, key bindings and editor transactions without a window.

use super::*;
use crate::keymap::{Action, Context, Custom, Lookup, Os, Preset};
use std::sync::atomic::AtomicU64;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "winc-plugin-app-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn test_app(text: &str) -> (App, Temp, Action) {
    let temp = Temp::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-duplicate-lines");
    winc_plugin::install(&temp.0, &source).unwrap();
    let mut editor = Editor::default();
    editor.new_untitled();
    editor
        .active_mut()
        .unwrap()
        .apply(winc_core::Transaction::new(
            vec![Change {
                start: 0,
                removed: String::new(),
                inserted: text.into(),
            }],
            Selection::point(0),
            Selection::point(0),
            winc_core::EditKind::Other,
        ));
    let mut app = App::new(editor, None, Bench::from_env(), Config::default());
    app.plugins.root = Some(temp.0.clone());
    keymap::plugins::install(Vec::new());
    app.reload_plugins();
    let action = Action::from_id("plugin:winc.remove-duplicate-lines/remove-duplicates").unwrap();
    (app, temp, action)
}

fn text(app: &App) -> String {
    app.editor
        .active()
        .unwrap()
        .text_range(0, usize::MAX)
        .unwrap()
}

#[test]
fn imports_and_reloads_plugins_in_the_portable_or_user_directory() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-duplicate-lines");
    for portable in [true, false] {
        let temp = Temp::new();
        // 配置目录恰好叫 data 不代表便携版，必须使用实际的便携模式判断结果。
        let data = temp.0.join("data");
        std::fs::create_dir(&data).unwrap();
        let config = data.join("config.json");
        let root = plugin_directory(Some(&config), portable.then_some(data.as_path())).unwrap();
        let expected = if portable { &temp.0 } else { &data }.join("plugins");
        let unused = if portable { &data } else { &temp.0 }.join("plugins");
        let mut app = App::new(
            Editor::default(),
            None,
            Bench::from_env(),
            Config::default(),
        );
        app.plugins.root = Some(root);
        keymap::plugins::install(Vec::new());
        app.import_plugin(source.clone());
        assert!(
            expected
                .join("winc.remove-duplicate-lines/plugin.json")
                .is_file()
        );
        assert!(!unused.exists());

        // 清空命令后重新加载，确认发现、禁用和卸载都使用同一个目录。
        keymap::plugins::install(Vec::new());
        app.reload_plugins();
        let action =
            Action::from_id("plugin:winc.remove-duplicate-lines/remove-duplicates").unwrap();
        assert!(app.action_available(action));
        app.manage_plugins();
        app.run_menu_command(MenuCommand::PluginEnable(0, false));
        assert!(
            expected
                .join("winc.remove-duplicate-lines/.disabled")
                .is_file()
        );
        app.manage_plugins();
        app.run_menu_command(MenuCommand::PluginRemove(0));
        assert!(!expected.join("winc.remove-duplicate-lines").exists());
    }
}

#[test]
fn plugin_command_transforms_and_undoes_the_document() {
    let (mut app, _dir, action) = test_app("甲\n乙\n甲\n");
    // 上游将静态命令列表拆到单独模块，管理命令和动态脚本都必须继续注册。
    for management in [
        Action::ImportPluginFolder,
        Action::ImportPluginPackage,
        Action::ManagePlugins,
        Action::ReloadPlugins,
    ] {
        assert!(palette().contains(&management));
        assert_eq!(Action::from_id(&management.id()), Some(management));
    }
    assert!(palette().contains(&action));
    assert!(app.run_action(action));
    assert_eq!(text(&app), "甲\n乙\n");
    app.run(Command::Undo);
    assert_eq!(text(&app), "甲\n乙\n甲\n");
    app.run(Command::Redo);
    assert_eq!(text(&app), "甲\n乙\n");
}

#[test]
fn empty_lines_collapse_to_one_editor_line_and_can_be_undone() {
    for ending in ["\n", "\r\n", "\r"] {
        for select_all in [false, true] {
            let original = ending.repeat(3);
            let (mut app, _dir, action) = test_app(&original);
            if select_all {
                app.editor
                    .active_mut()
                    .unwrap()
                    .set_selections(vec![Selection::new(0, original.chars().count())], 0);
            }
            assert!(app.run_action(action));
            assert_eq!(text(&app), "");
            assert_eq!(app.editor.active().unwrap().text().line_count(), 1);
            // 对空文档再次运行不增加撤销步骤，一次撤销就能恢复原来的空行。
            assert!(app.run_action(action));
            assert_eq!(text(&app), "");
            app.run(Command::Undo);
            assert_eq!(text(&app), original);
            app.run(Command::Redo);
            assert_eq!(text(&app), "");
        }
    }
}

#[test]
fn only_selected_text_changes_and_multiselections_undo_as_one_step() {
    let original = "前\na\na\n中\nb\nb\n后";
    let (mut app, _dir, action) = test_app(original);
    app.editor
        .active_mut()
        .unwrap()
        .set_selections(vec![Selection::new(2, 6), Selection::new(8, 12)], 0);
    let selections = app.editor.active().unwrap().selections();
    app.run_action(action);
    assert_eq!(text(&app), "前\na\n中\nb\n后");
    app.run(Command::Undo);
    assert_eq!(text(&app), original);
    assert_eq!(app.editor.active().unwrap().selections(), selections);
}

#[test]
fn json_sort_command_handles_selections_atomically_and_supports_undo() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/sort-json-keys");
    let left = r#"{"b":2,"a":1}"#;
    let right = r#"{"d":4,"c":3}"#;
    let sorted_left = "{\n  \"a\": 1,\n  \"b\": 2\n}";
    let sorted_right = "{\n  \"c\": 3,\n  \"d\": 4\n}";
    for selected in [false, true] {
        for valid in [false, true] {
            let right = if valid { right } else { r#"{"c":}"# };
            let original = if selected {
                format!("前\n{left}\n中\n{right}\n后")
            } else {
                right.to_string()
            };
            let (mut app, _dir, _) = test_app(&original);
            app.import_plugin(source.clone());
            let action = Action::from_id("plugin:winc.sort-json-keys/sort-keys").unwrap();
            assert!(palette().contains(&action));
            if selected {
                let left_end = 2 + left.chars().count();
                let right_start = left_end + 3;
                app.editor.active_mut().unwrap().set_selections(
                    vec![
                        Selection::new(2, left_end),
                        Selection::new(right_start, right_start + right.chars().count()),
                    ],
                    0,
                );
            }
            let selections = app.editor.active().unwrap().selections();
            assert!(app.run_action(action));
            if valid {
                let expected = if selected {
                    format!("前\n{sorted_left}\n中\n{sorted_right}\n后")
                } else {
                    sorted_right.to_string()
                };
                assert_eq!(text(&app), expected);
                app.run(Command::Undo);
                assert_eq!(text(&app), original);
                assert_eq!(app.editor.active().unwrap().selections(), selections);
                app.run(Command::Redo);
                assert_eq!(text(&app), expected);
            } else {
                // A malformed second selection must not apply the successful first result.
                assert_eq!(text(&app), original);
                assert_eq!(app.editor.active().unwrap().selections(), selections);
            }
        }
    }
}

#[test]
fn remove_empty_line_supports_document_selections_and_undo() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-empty-line");
    for (original, selections, expected) in [
        ("\n  甲\n \t\n乙\n", vec![], "  甲\n乙\n"),
        ("\t\n \n\u{3000}", vec![], ""),
        (
            "外\n\n甲\n \n乙\n后\n\n",
            vec![Selection::new(3, 9)],
            "外\n\n甲\n乙\n后\n\n",
        ),
        (
            "前\n甲\n \n中\n\t\n乙\n后",
            vec![Selection::new(2, 6), Selection::new(8, 12)],
            "前\n甲\n中\n乙\n后",
        ),
    ] {
        let (mut app, _dir, _) = test_app(original);
        app.import_plugin(source.clone());
        let action = Action::from_id("plugin:winc.remove-empty-line/remove-empty-line").unwrap();
        assert!(palette().contains(&action));
        assert!(app.action_available(action));
        if !selections.is_empty() {
            app.editor
                .active_mut()
                .unwrap()
                .set_selections(selections, 0);
        }
        let selections = app.editor.active().unwrap().selections();
        assert!(app.run_action(action));
        assert_eq!(text(&app), expected);
        if expected.is_empty() {
            assert_eq!(app.editor.active().unwrap().text().line_count(), 1);
            // 再次处理空文档不会额外创建撤销步骤。
            assert!(app.run_action(action));
        }
        app.run(Command::Undo);
        assert_eq!(text(&app), original);
        assert_eq!(app.editor.active().unwrap().selections(), selections);
        app.run(Command::Redo);
        assert_eq!(text(&app), expected);
    }
}

#[test]
fn jsonc_sort_preserves_field_comments_and_can_be_undone() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/sort-json-keys");
    let original = "{\n// z docs\n\"z\":1, // z tail\n// a docs\n\"a\":2,\n}";
    let expected = "{\n  // a docs\n  \"a\": 2,\n  // z docs\n  \"z\": 1 // z tail\n}";
    let (mut app, _dir, _) = test_app(original);
    app.import_plugin(source);
    let action = Action::from_id("plugin:winc.sort-json-keys/sort-keys").unwrap();
    assert!(app.run_action(action));
    assert_eq!(text(&app), expected);
    app.run(Command::Undo);
    assert_eq!(text(&app), original);
    app.run(Command::Redo);
    assert_eq!(text(&app), expected);
}

#[test]
fn shortcuts_persist_while_plugins_are_disabled_or_missing() {
    let (mut app, dir, action) = test_app("a\na\n");
    let id = action.id();
    app.config
        .shortcuts
        .insert(id.clone(), vec!["ctrl+alt+u".into()]);
    keymap::install_from(&app.config);
    let key = keymap::parse_spec("ctrl+alt+u").unwrap().0;
    assert_eq!(
        keymap::lookup(&keymap::current(), key, None, Context::Editor),
        Lookup::Run(vec![action])
    );
    assert!(app.on_shortcut(key));
    assert_eq!(text(&app), "a\n");
    winc_plugin::set_enabled(&dir.0, "winc.remove-duplicate-lines", false).unwrap();
    app.reload_plugins();
    assert!(!app.action_available(action));
    assert_eq!(
        keymap::lookup(&keymap::current(), key, None, Context::Editor),
        Lookup::None
    );
    let (custom, errors) = Custom::from_config(&app.config.shortcuts);
    assert!(errors.is_empty());
    assert!(custom.to_config(Os::CURRENT).contains_key(&id));
    keymap::plugins::install(Vec::new());
    keymap::install(Preset::VsCode, &custom);
    assert!(
        Custom::from_config(&app.config.shortcuts)
            .0
            .to_config(Os::CURRENT)
            .contains_key(&id)
    );
    let unknown = std::collections::BTreeMap::from([(
        "plugin:missing/run".into(),
        vec!["ctrl+alt+j".into()],
    )]);
    assert_eq!(
        Custom::from_config(&unknown).0.to_config(Os::CURRENT),
        unknown
    );
    winc_plugin::set_enabled(&dir.0, "winc.remove-duplicate-lines", true).unwrap();
    app.reload_plugins();
    assert_eq!(Action::from_id(&id), Some(action));
    assert_eq!(
        keymap::lookup(&keymap::current(), key, None, Context::Editor),
        Lookup::Run(vec![action])
    );
}

#[test]
fn readonly_and_stale_results_do_not_change_text() {
    let (mut app, _dir, action) = test_app("a\na\n");
    app.editor.active_mut().unwrap().set_locked(true);
    app.run_action(action);
    assert_eq!(text(&app), "a\na\n");
    app.editor.active_mut().unwrap().set_locked(false);
    let doc = app.editor.active().unwrap();
    let token = Arc::new(AtomicBool::new(false));
    app.plugins.running = Some(Pending {
        document: doc.id,
        sent: doc.format_source().unwrap(),
        cancel: token.clone(),
        title: "Test".into(),
    });
    let doc = app.editor.active_mut().unwrap();
    doc.apply(winc_core::Transaction::new(
        vec![Change {
            start: 0,
            removed: String::new(),
            inserted: "new\n".into(),
        }],
        doc.selection,
        Selection::point(0),
        winc_core::EditKind::Other,
    ));
    app.plugins
        .channel
        .0
        .send(Outcome::Edited(
            token,
            Ok(vec![Change {
                start: 0,
                removed: "a\na\n".into(),
                inserted: "a\n".into(),
            }]),
        ))
        .unwrap();
    app.poll_plugins();
    assert_eq!(text(&app), "new\na\na\n");
}

#[test]
fn manager_menu_disables_enables_and_uninstalls_the_chosen_plugin() {
    let (mut app, _dir, action) = test_app("a\na\n");
    app.manage_plugins();
    assert!(app.overlays.context_menu.is_some());
    app.run_menu_command(MenuCommand::PluginEnable(0, false));
    assert!(!app.action_available(action));
    app.manage_plugins();
    app.run_menu_command(MenuCommand::PluginEnable(0, true));
    assert!(app.action_available(action));
    app.manage_plugins();
    app.run_menu_command(MenuCommand::PluginRemove(0));
    assert!(!app.action_available(action));
    assert!(!palette().contains(&action));
}

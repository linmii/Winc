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
fn plugin_command_transforms_and_undoes_the_document() {
    let (mut app, _dir, action) = test_app("甲\n乙\n甲\n");
    assert!(palette().contains(&action));
    assert!(app.run_action(action));
    assert_eq!(text(&app), "甲\n乙\n");
    app.run(Command::Undo);
    assert_eq!(text(&app), "甲\n乙\n甲\n");
    app.run(Command::Redo);
    assert_eq!(text(&app), "甲\n乙\n");
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

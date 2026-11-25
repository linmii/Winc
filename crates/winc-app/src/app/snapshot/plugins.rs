//! Render plugin commands, the shortcut editor and plugin management using the normal UI.

use super::*;
use crate::keymap::{self, Action};

#[test]
fn plugin_commands_appear_in_palette_shortcuts_and_management() {
    let dir = sample_folder("plugin-ui");
    let root = dir.join("plugins");
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/remove-duplicate-lines");
    winc_plugin::install(&root, &source).unwrap();
    keymap::plugins::install(winc_plugin::load(&root).plugins);
    let mut app = blank_app(false);
    app.sync_plugin_commands();
    let action = Action::from_id("plugin:winc.remove-duplicate-lines/remove-duplicates").unwrap();
    app.open_palette();
    app.overlays.picker.input.set_text(">去除重复行");
    app.palette_refresh();
    assert_eq!(app.overlays.picker.hits.len(), 1);
    assert_eq!(palette()[app.overlays.picker.hits[0]], action);
    paint(&mut app);
    if std::env::var_os("WINC_SNAPSHOT_DIR").is_some() {
        shoot(&mut app, "plugin-palette");
    }

    app.close_picker();
    app.open_settings();
    app.settings_show_page(SettingsPage::Keys);
    app.settings.keys_query.set_text("去除重复行");
    app.start_key_edit(action, true);
    paint(&mut app);
    assert!(
        app.hits
            .keys_parts
            .iter()
            .any(|(_, part)| matches!(part, SettingsPart::KeySave))
    );
    if std::env::var_os("WINC_SNAPSHOT_DIR").is_some() {
        shoot(&mut app, "plugin-shortcuts");
    }
    app.close_settings();

    app.manage_plugins();
    paint(&mut app);
    assert!(menu_commands(&app).contains(&(MenuCommand::PluginRemove(0), true)));
    if std::env::var_os("WINC_SNAPSHOT_DIR").is_some() {
        shoot(&mut app, "plugin-manager");
    }
    let _ = std::fs::remove_dir_all(dir);
}

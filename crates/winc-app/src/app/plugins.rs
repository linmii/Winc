//! Plugin UI, package jobs and transactional text edits. Scripts run outside the UI thread.

use std::sync::atomic::Ordering;

use ropey::Rope;
use winc_core::Change;
use winc_i18n::t;

use super::input::context_menu::{MenuEntry, MenuTarget};
use super::*;
use crate::keymap;

/// Per-window jobs; package metadata and command handles are shared between windows.
pub(crate) struct Plugins {
    channel: (mpsc::Sender<Outcome>, mpsc::Receiver<Outcome>),
    loading: bool,
    revision: u64,
    running: Option<Pending>,
    #[cfg(test)]
    root: Option<PathBuf>,
}

impl Default for Plugins {
    fn default() -> Self {
        Self {
            channel: mpsc::channel(),
            loading: false,
            revision: 0,
            running: None,
            #[cfg(test)]
            root: None,
        }
    }
}

struct Pending {
    document: DocumentId,
    sent: Rope,
    cancel: Arc<AtomicBool>,
    title: String,
}

enum Outcome {
    Catalog(u64, winc_plugin::Catalog, Option<String>),
    Failed(String),
    Edited(Arc<AtomicBool>, Result<Vec<Change>, String>),
}

impl Drop for Plugins {
    fn drop(&mut self) {
        if let Some(job) = &self.running {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl App {
    fn plugin_root(&self) -> Option<PathBuf> {
        #[cfg(test)]
        {
            self.plugins.root.clone()
        }
        #[cfg(not(test))]
        self.store
            .config_path
            .clone()
            .or_else(winc_config::config_path)
            .and_then(|p| p.parent().map(|p| p.join("plugins")))
    }

    /// Reload scripts after a package change or an explicit developer reload command.
    pub(in crate::app) fn reload_plugins(&mut self) {
        self.plugin_job(|_| Ok(None));
    }

    /// Called by the local file/folder chooser; the worker validates before installing.
    pub(in crate::app) fn import_plugin(&mut self, source: PathBuf) {
        self.plugin_job(move |root| {
            winc_plugin::install(root, &source)
                .map(|id| Some(t!("plugin-installed", name = id)))
                .map_err(|e| e.to_string())
        });
    }

    fn plugin_job(
        &mut self,
        job: impl FnOnce(&Path) -> Result<Option<String>, String> + Send + 'static,
    ) {
        if self.plugins.loading {
            self.flash(t!("plugin-busy"));
            return;
        }
        let Some(root) = self.plugin_root() else {
            self.notify(NoticeKind::Error, t!("plugin-no-directory"));
            return;
        };
        self.plugins.loading = true;
        self.cancel_plugin();
        let sender = self.plugins.channel.0.clone();
        self.spawn_blocking_then(UserEvent::PluginsReady, move || {
            // Serialize filesystem changes across windows. A later result cannot be replaced
            // by a stale catalog whose event arrives after it.
            static JOBS: Mutex<u64> = Mutex::new(0);
            let mut generation = JOBS.lock().unwrap_or_else(|e| e.into_inner());
            *generation += 1;
            let outcome = match job(&root) {
                Ok(message) => Outcome::Catalog(*generation, winc_plugin::load(&root), message),
                Err(error) => Outcome::Failed(error),
            };
            let _ = sender.send(outcome);
        });
    }

    /// Manage installed packages through the existing native/in-app menu infrastructure.
    pub(in crate::app) fn manage_plugins(&mut self) {
        if self.plugins.loading {
            self.flash(t!("plugin-busy"));
            return;
        }
        let packages = keymap::plugins::packages();
        if packages.is_empty() {
            self.flash(t!("plugin-none"));
            return;
        }
        let ids = packages.iter().map(|p| p.manifest.id.clone()).collect();
        self.open_context_menu(
            MenuTarget::Plugins(ids),
            (self.layout.editor.x + 24.0, self.layout.editor.y + 24.0),
        );
    }

    pub(in crate::app) fn plugin_menu(&self, ids: &[String]) -> Vec<MenuEntry> {
        let packages = keymap::plugins::packages();
        ids.iter()
            .enumerate()
            .filter_map(|(i, id)| {
                let p = packages.iter().find(|p| p.manifest.id == *id)?;
                let state = if p.error.is_some() {
                    t!("plugin-invalid")
                } else if p.enabled {
                    t!("plugin-enabled")
                } else {
                    t!("plugin-disabled")
                };
                Some(MenuEntry::Submenu(
                    format!("{}  {} · {state}", p.manifest.name, p.manifest.version),
                    vec![
                        MenuEntry::Item(
                            MenuCommand::PluginEnable(i, !p.enabled),
                            !self.plugins.loading,
                        ),
                        MenuEntry::Item(MenuCommand::PluginRemove(i), !self.plugins.loading),
                    ],
                ))
            })
            .collect()
    }

    pub(in crate::app) fn plugin_menu_command(&mut self, ids: &[String], command: MenuCommand) {
        let (index, enabled) = match command {
            MenuCommand::PluginEnable(i, on) => (i, Some(on)),
            MenuCommand::PluginRemove(i) => (i, None),
            _ => return,
        };
        let Some(id) = ids.get(index).cloned() else {
            return;
        };
        self.plugin_job(move |root| {
            match enabled {
                Some(on) => winc_plugin::set_enabled(root, &id, on),
                None => winc_plugin::uninstall(root, &id),
            }
            .map_err(|e| e.to_string())?;
            Ok(Some(t!("plugin-updated", name = id)))
        });
    }

    /// Refresh local picker indices when another window has changed the shared catalog.
    pub(in crate::app) fn sync_plugin_commands(&mut self) {
        let revision = keymap::plugins::revision();
        if self.plugins.revision == revision {
            return;
        }
        self.plugins.revision = revision;
        self.cancel_plugin();
        self.cancel_key_edit();
        self.pending_chord = None;
        keymap::install_from(&self.config);
        if self.picker_lists_commands() {
            self.palette_refresh();
        }
        self.keymap_changed();
    }

    pub(in crate::app) fn plugin_available(&self, slot: usize) -> bool {
        keymap::plugins::command(slot).is_some()
    }

    /// Transform all nonempty selections, or the document when no text is selected.
    pub(in crate::app) fn run_plugin(&mut self, slot: usize) {
        if self.plugins.loading || self.plugins.running.is_some() {
            self.flash(t!("plugin-busy"));
            return;
        }
        let Some((plugin, command)) = keymap::plugins::command(slot) else {
            return;
        };
        let Some(doc) = self.editor.active() else {
            self.flash(t!("plugin-no-document"));
            return;
        };
        let sent = match doc.format_source() {
            Ok(sent) => sent,
            Err(error) => {
                self.notify(NoticeKind::Warning, error);
                return;
            }
        };
        let mut ranges: Vec<_> = doc
            .selections()
            .0
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.from()..s.to())
            .collect();
        if ranges.is_empty() {
            ranges.push(0..sent.len_chars());
        }
        ranges.sort_by_key(|r| r.start);
        // Adjacent/overlapping selections are one input. Never apply overlapping replacements.
        let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(previous) = merged.last_mut()
                && range.start <= previous.end
            {
                previous.end = previous.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        let bytes: usize = merged
            .iter()
            .map(|r| sent.slice(r.clone()).len_bytes())
            .sum();
        if bytes > winc_plugin::MAX_TEXT_BYTES || merged.len() > 256 {
            self.notify(NoticeKind::Warning, t!("plugin-input-limit"));
            return;
        }
        let title = keymap::Action::Plugin(slot).title().into_owned();
        let ending = doc.line_ending.as_str().to_owned();
        let cancel = Arc::new(AtomicBool::new(false));
        self.plugins.running = Some(Pending {
            document: doc.id,
            sent: sent.clone(),
            cancel: cancel.clone(),
            title,
        });
        let sender = self.plugins.channel.0.clone();
        self.flash(t!("plugin-running"));
        self.spawn_blocking_then(UserEvent::PluginsReady, move || {
            let result = (|| {
                let mut changes = Vec::new();
                let mut total = 0;
                let started = Instant::now();
                // Apply right to left so every offset still addresses the original snapshot.
                for range in merged.into_iter().rev() {
                    if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(5)
                    {
                        return Err("Plugin cancelled".into());
                    }
                    let removed = sent.slice(range.clone()).to_string();
                    let inserted = plugin
                        .run(
                            winc_plugin::Input {
                                text: removed.clone(),
                                line_ending: ending.clone(),
                                command: command.clone(),
                            },
                            cancel.clone(),
                        )
                        .map_err(|e| e.to_string())?;
                    total += inserted.len();
                    if total > winc_plugin::MAX_TEXT_BYTES {
                        return Err("Plugin output exceeds 8 MiB".into());
                    }
                    if inserted != removed {
                        changes.push(Change {
                            start: range.start,
                            removed,
                            inserted,
                        });
                    }
                }
                Ok(changes)
            })();
            let _ = sender.send(Outcome::Edited(cancel, result));
        });
    }

    pub(in crate::app) fn cancel_plugin(&mut self) {
        if let Some(job) = self.plugins.running.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub(in crate::app) fn poll_plugins(&mut self) {
        while let Ok(outcome) = self.plugins.channel.1.try_recv() {
            match outcome {
                Outcome::Catalog(generation, catalog, message) => {
                    self.plugins.loading = false;
                    keymap::plugins::publish(generation, catalog.plugins);
                    self.sync_plugin_commands();
                    if let Some(message) = message {
                        self.flash(message);
                    }
                    if !catalog.errors.is_empty() {
                        self.notify(
                            NoticeKind::Warning,
                            t!("plugin-load-error", error = catalog.errors.join("\n")),
                        );
                    }
                }
                Outcome::Failed(error) => {
                    self.plugins.loading = false;
                    self.notify(NoticeKind::Error, t!("plugin-error", error = error));
                }
                Outcome::Edited(token, result) => {
                    if !self
                        .plugins
                        .running
                        .as_ref()
                        .is_some_and(|p| Arc::ptr_eq(&p.cancel, &token))
                    {
                        continue;
                    }
                    let pending = self.plugins.running.take().unwrap();
                    if token.load(Ordering::Relaxed) {
                        continue;
                    }
                    match result {
                        Ok(changes) => {
                            if self.editor.document(pending.document).is_none() {
                                continue;
                            }
                            let clean = changes.is_empty();
                            if self.apply_formatted(pending.document, &pending.sent, changes) {
                                self.flash(if clean {
                                    t!("plugin-unchanged").to_owned()
                                } else {
                                    t!("plugin-applied", name = pending.title)
                                });
                            } else {
                                self.notify(NoticeKind::Warning, t!("plugin-document-changed"));
                            }
                        }
                        Err(error) => {
                            self.notify(NoticeKind::Error, t!("plugin-error", error = error))
                        }
                    }
                }
            }
        }
        self.request_redraw();
    }
}

#[cfg(test)]
mod tests;

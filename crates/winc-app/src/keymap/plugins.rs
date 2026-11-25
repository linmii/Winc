//! Stable runtime handles for plugin commands. Persist string ids, never these process-local slots.

use std::sync::Arc;

use super::Action;

#[derive(Default)]
struct Registry {
    slots: Vec<(String, String)>,
    packages: Vec<Arc<winc_plugin::Plugin>>,
    actions: Vec<Action>,
    revision: u64,
    loaded: u64,
}

#[cfg(not(test))]
static REGISTRY: std::sync::RwLock<Registry> = std::sync::RwLock::new(Registry {
    slots: Vec::new(),
    packages: Vec::new(),
    actions: Vec::new(),
    revision: 0,
    loaded: 0,
});

#[cfg(test)]
thread_local! {
    static REGISTRY: std::cell::RefCell<Registry> = Default::default();
}

fn with<R>(f: impl FnOnce(&Registry) -> R) -> R {
    #[cfg(not(test))]
    {
        f(&REGISTRY.read().unwrap_or_else(|e| e.into_inner()))
    }
    #[cfg(test)]
    {
        REGISTRY.with_borrow(f)
    }
}

/// Publish a newly loaded catalog, retaining handles so stale menus never target another command.
pub(crate) fn publish(generation: u64, packages: Vec<winc_plugin::Plugin>) {
    let update = |r: &mut Registry| {
        if generation < r.loaded {
            return;
        }
        r.loaded = generation;
        r.actions.clear();
        r.packages = packages.into_iter().map(Arc::new).collect();
        for plugin in &r.packages {
            for command in &plugin.manifest.commands {
                let id = plugin.command_id(command);
                let slot = match r.slots.iter().position(|s| s.0 == id) {
                    Some(slot) => {
                        r.slots[slot].1 = command.title.clone();
                        slot
                    }
                    None => {
                        r.slots.push((id, command.title.clone()));
                        r.slots.len() - 1
                    }
                };
                r.actions.push(Action::Plugin(slot));
            }
        }
        r.revision += 1;
    };
    #[cfg(not(test))]
    {
        update(&mut REGISTRY.write().unwrap_or_else(|e| e.into_inner()));
    }
    #[cfg(test)]
    {
        REGISTRY.with_borrow_mut(update);
    }
}

#[cfg(test)]
pub(crate) fn install(packages: Vec<winc_plugin::Plugin>) {
    REGISTRY.with_borrow_mut(|r| r.loaded = 0);
    publish(0, packages);
}

pub(crate) fn revision() -> u64 {
    with(|r| r.revision)
}
pub(crate) fn packages() -> Vec<Arc<winc_plugin::Plugin>> {
    with(|r| r.packages.clone())
}
pub(super) fn title(slot: usize) -> String {
    with(|r| r.slots.get(slot).map(|s| s.1.clone()).unwrap_or_default())
}
pub(super) fn id(slot: usize) -> String {
    with(|r| r.slots.get(slot).map(|s| s.0.clone()).unwrap_or_default())
}
pub(super) fn from_id(id: &str) -> Option<Action> {
    with(|r| r.slots.iter().position(|s| s.0 == id).map(Action::Plugin))
}

/// Find the current package for an enabled command; removed/disabled commands cannot execute.
pub(crate) fn command(slot: usize) -> Option<(Arc<winc_plugin::Plugin>, String)> {
    with(|r| {
        let id = &r.slots.get(slot)?.0;
        for plugin in &r.packages {
            if !plugin.enabled || plugin.error.is_some() {
                continue;
            }
            for command in &plugin.manifest.commands {
                if plugin.command_id(command) == *id {
                    return Some((Arc::clone(plugin), command.id.clone()));
                }
            }
        }
        None
    })
}

/// Built-in and currently installed plugin commands, in display order.
pub fn actions() -> Vec<Action> {
    super::ACTIONS
        .iter()
        .copied()
        .chain(with(|r| r.actions.clone()))
        .collect()
}

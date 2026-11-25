//! Package storage. Only the manifest and explicitly declared scripts are installed.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rhai::grain::Program;
use serde::{Deserialize, Serialize};

use crate::{Error, Input, runtime};

const MAX_MANIFEST: usize = 64 * 1024;
const MAX_SCRIPT: usize = 256 * 1024;
const MAX_COMMANDS: usize = 32;
const MAX_PLUGINS: usize = 64;

/// Versioned text-plugin manifest, stored as `plugin.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Stable lower-case identifier (letters, digits, dots and hyphens).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin author's version label.
    pub version: String,
    /// Host API version. This implementation supports version 1.
    pub api_version: u32,
    /// Commands exposed to the command palette and keyboard settings.
    pub commands: Vec<Command>,
}

/// One text transformation. Each invocation evaluates the named script.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    /// Identifier within the plugin.
    pub id: String,
    /// Command palette label.
    pub title: String,
    /// Relative `.rhai` file path within the package.
    pub script: String,
    /// Default scalar options visible to the script as `options`.
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

/// A validated installed plugin with cached Grain programs.
#[derive(Clone)]
pub struct Plugin {
    /// Public package metadata.
    pub manifest: Manifest,
    /// Whether commands may be executed.
    pub enabled: bool,
    /// Script loading failure; metadata remains available for management and repair.
    pub error: Option<String>,
    programs: BTreeMap<String, Arc<Program<'static>>>,
}

impl Plugin {
    /// Execute a declared command using its cached program.
    ///
    /// # Errors
    /// Returns an error for disabled/unknown commands, invalid output or exceeded script limits.
    pub fn run(&self, input: Input, cancel: Arc<AtomicBool>) -> Result<String, Error> {
        if let Some(error) = &self.error {
            return Err(Error(error.clone()));
        }
        if !self.enabled {
            return Err(Error("Plugin is disabled".into()));
        }
        let command = self
            .manifest
            .commands
            .iter()
            .find(|c| c.id == input.command)
            .ok_or_else(|| Error("Unknown plugin command".into()))?;
        runtime::execute(
            &self.programs[&command.script],
            input,
            &command.options,
            cancel,
        )
    }

    /// Stable command identifier persisted in Winc's shortcut configuration.
    pub fn command_id(&self, command: &Command) -> String {
        format!("plugin:{}/{}", self.manifest.id, command.id)
    }
}

/// Installed plugins and diagnostics; one broken package does not hide the other plugins.
#[derive(Default)]
pub struct Catalog {
    /// Packages ordered by identifier.
    pub plugins: Vec<Plugin>,
    /// Unreadable or invalid packages.
    pub errors: Vec<String>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && value.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && !value.ends_with('.')
        && !value.contains("..")
        && !matches!(
            value.split('.').next(),
            Some(
                "con"
                    | "prn"
                    | "aux"
                    | "nul"
                    | "com1"
                    | "com2"
                    | "com3"
                    | "com4"
                    | "com5"
                    | "com6"
                    | "com7"
                    | "com8"
                    | "com9"
                    | "lpt1"
                    | "lpt2"
                    | "lpt3"
                    | "lpt4"
                    | "lpt5"
                    | "lpt6"
                    | "lpt7"
                    | "lpt8"
                    | "lpt9"
            )
        )
}

fn relative(value: &str) -> Result<PathBuf, Error> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains(['\\', ':'])
        || path.is_absolute()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
        || value
            .split('/')
            .any(|p| p.is_empty() || p.ends_with(['.', ' ']))
    {
        return Err(Error(format!("Invalid package path: {value}")));
    }
    Ok(path.to_path_buf())
}

fn bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    reader.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error("Plugin file is too large".into()));
    }
    Ok(bytes)
}

fn local_file(root: &Path, name: &str, limit: usize) -> Result<Vec<u8>, Error> {
    let rel = relative(name)?;
    let mut path = root.to_path_buf();
    // Reject symlinks/reparse points in every component, not just in the final script.
    for part in rel.components() {
        path.push(part.as_os_str());
        reject_link(&path)?;
    }
    bounded(fs::File::open(path)?, limit)
}

fn reject_link(path: &Path) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path)?;
    let linked = meta.file_type().is_symlink();
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        linked || meta.file_attributes() & 0x400 != 0
    };
    if linked {
        return Err(Error("Plugin packages cannot contain links".into()));
    }
    Ok(())
}

fn parse(bytes: &[u8]) -> Result<Manifest, Error> {
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
    if !valid_id(&manifest.id)
        || manifest.api_version != 1
        || manifest.name.trim().is_empty()
        || manifest.name.len() > 256
        || manifest.version.is_empty()
        || manifest.version.len() > 64
        || manifest.commands.is_empty()
        || manifest.commands.len() > MAX_COMMANDS
    {
        return Err(Error(
            "Invalid plugin metadata or unsupported API version (expected 1)".into(),
        ));
    }
    let mut ids = HashSet::new();
    for command in &manifest.commands {
        if !valid_id(&command.id)
            || !ids.insert(&command.id)
            || command.title.trim().is_empty()
            || command.title.len() > 256
            || command.options.len() > 32
            || command
                .options
                .values()
                .any(|v| !(v.is_boolean() || v.is_string() || v.as_i64().is_some()))
        {
            return Err(Error("Invalid or duplicate plugin command".into()));
        }
        let path = relative(&command.script)?;
        if path.extension().is_none_or(|e| e != "rhai") {
            return Err(Error(
                "Plugin entry points must be .rhai source files".into(),
            ));
        }
    }
    Ok(manifest)
}

fn prepare(
    manifest: Manifest,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, Error>,
) -> Result<(Plugin, BTreeMap<String, Vec<u8>>), Error> {
    let mut programs = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for command in &manifest.commands {
        if programs.contains_key(&command.script) {
            continue;
        }
        let bytes = read(&command.script)?;
        let source = std::str::from_utf8(&bytes).map_err(|e| Error(e.to_string()))?;
        programs.insert(command.script.clone(), Arc::new(runtime::compile(source)?));
        sources.insert(command.script.clone(), bytes);
    }
    Ok((
        Plugin {
            manifest,
            enabled: true,
            error: None,
            programs,
        },
        sources,
    ))
}

/// Read all installed packages without executing their scripts.
pub fn load(root: &Path) -> Catalog {
    let mut catalog = Catalog::default();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return catalog,
        Err(e) => {
            catalog.errors.push(e.to_string());
            return catalog;
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if paths.len() >= MAX_PLUGINS {
                    catalog
                        .errors
                        .push("At most 64 plugins may be installed".into());
                    break;
                }
                paths.push(entry.path());
            }
            Err(e) => catalog.errors.push(e.to_string()),
        }
    }
    paths.sort();
    for path in paths {
        let result = (|| {
            reject_link(&path)?;
            let manifest = parse(&local_file(&path, "plugin.json", MAX_MANIFEST)?)?;
            if path.file_name().and_then(|s| s.to_str()) != Some(&manifest.id) {
                return Err(Error("Installed directory must match plugin id".into()));
            }
            let mut plugin =
                match prepare(manifest.clone(), |name| local_file(&path, name, MAX_SCRIPT)) {
                    Ok((plugin, _)) => plugin,
                    Err(error) => Plugin {
                        manifest,
                        enabled: true,
                        error: Some(error.to_string()),
                        programs: BTreeMap::new(),
                    },
                };
            plugin.enabled = !path.join(".disabled").try_exists()?;
            Ok::<_, Error>(plugin)
        })();
        match result {
            Ok(plugin) => {
                if let Some(error) = &plugin.error {
                    catalog.errors.push(format!("{}: {error}", path.display()));
                }
                catalog.plugins.push(plugin);
            }
            Err(error) => catalog.errors.push(format!("{}: {error}", path.display())),
        }
    }
    catalog
}

/// Install a directory, its `plugin.json`, or a ZIP with `plugin.json` at its root.
/// Existing identifiers are never overwritten. Only validated manifest/script files are copied.
///
/// # Errors
/// Returns an error for invalid packages, duplicate ids, exceeded limits or filesystem failures.
pub fn install(root: &Path, source: &Path) -> Result<String, Error> {
    let (plugin, sources) = if source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        let file = fs::File::open(source)?;
        if file.metadata()?.len() > 16 * 1024 * 1024 {
            return Err(Error("Plugin ZIP exceeds 16 MiB".into()));
        }
        let mut archive = zip::ZipArchive::new(file).map_err(|e| Error(e.to_string()))?;
        if archive.len() > 256 {
            return Err(Error("Too many ZIP entries".into()));
        }
        let mut names = HashSet::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| Error(e.to_string()))?;
            let name = entry.name().map_err(|e| Error(e.to_string()))?;
            relative(name.trim_end_matches('/'))?;
            if !names.insert(name.to_lowercase()) || entry.is_symlink() {
                return Err(Error(
                    "Duplicate ZIP paths or symbolic links are not allowed".into(),
                ));
            }
        }
        let bytes = bounded(
            archive
                .by_name("plugin.json")
                .map_err(|e| Error(e.to_string()))?,
            MAX_MANIFEST,
        )?;
        let manifest = parse(&bytes)?;
        prepare(manifest, |name| {
            let entry = archive.by_name(name).map_err(|e| Error(e.to_string()))?;
            bounded(entry, MAX_SCRIPT)
        })?
    } else {
        let dir = if source.is_dir() {
            source
        } else if source.file_name().is_some_and(|n| n == "plugin.json") {
            source
                .parent()
                .ok_or_else(|| Error("Missing package directory".into()))?
        } else {
            return Err(Error(
                "Choose plugin.json, a plugin directory or a ZIP package".into(),
            ));
        };
        reject_link(dir)?;
        let manifest = parse(&local_file(dir, "plugin.json", MAX_MANIFEST)?)?;
        prepare(manifest, |name| local_file(dir, name, MAX_SCRIPT))?
    };
    fs::create_dir_all(root)?;
    reject_link(root)?;
    if fs::read_dir(root)?.count() >= MAX_PLUGINS {
        return Err(Error("At most 64 plugins may be installed".into()));
    }
    let id = &plugin.manifest.id;
    let destination = root.join(id);
    if destination.try_exists()? {
        return Err(Error(format!(
            "Plugin {id} is already installed; uninstall it first"
        )));
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let staging = root.join(format!(
        ".install-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&staging)?;
    let result = (|| {
        for (name, bytes) in sources {
            let path = staging.join(relative(&name)?);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            file.write_all(&bytes)?;
        }
        let bytes =
            serde_json::to_vec_pretty(&plugin.manifest).map_err(|e| Error(e.to_string()))?;
        fs::write(staging.join("plugin.json"), bytes)?;
        fs::rename(&staging, &destination)?;
        Ok(id.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn installed(root: &Path, id: &str) -> Result<PathBuf, Error> {
    if !valid_id(id) {
        return Err(Error("Invalid plugin id".into()));
    }
    reject_link(root)?;
    let path = root.join(id);
    reject_link(&path)?;
    let manifest = parse(&local_file(&path, "plugin.json", MAX_MANIFEST)?)?;
    if manifest.id != id {
        return Err(Error("Plugin directory mismatch".into()));
    }
    Ok(path)
}

/// Persist enabled state independently of plugin code and metadata.
///
/// # Errors
/// Returns an error if the package is invalid or its state cannot be written.
pub fn set_enabled(root: &Path, id: &str, enabled: bool) -> Result<(), Error> {
    let marker = installed(root, id)?.join(".disabled");
    if enabled {
        match fs::remove_file(marker) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    } else {
        if marker.try_exists()? {
            reject_link(&marker)?;
        }
        fs::write(marker, b"").map_err(Into::into)
    }
}

/// Remove a validated package from the plugin storage directory.
///
/// # Errors
/// Returns an error if the id/path is invalid or the package cannot be removed.
pub fn uninstall(root: &Path, id: &str) -> Result<(), Error> {
    let path = installed(root, id)?;
    fs::remove_dir_all(path).map_err(Into::into)
}

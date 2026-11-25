//! Compile once on loading, execute on a worker with a fresh scope for every invocation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rhai::grain::{Compiler, Program, Vm};
use rhai::{Dynamic, Engine, ImmutableString, Map, Scope};

use crate::Error;

/// Maximum UTF-8 bytes accepted or returned by one text transformation (8 MiB).
pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;

/// Immutable invocation data. Positions and document identities stay in the application.
#[derive(Debug, Clone)]
pub struct Input {
    /// Selected text, or the whole document when there is no selection.
    pub text: String,
    /// Document line ending: `\n`, `\r\n` or `\r`.
    pub line_ending: String,
    /// Command name declared in the plugin manifest.
    pub command: String,
}

fn engine(cancel: Arc<AtomicBool>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(5_000_000);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(MAX_TEXT_BYTES);
    engine.set_max_array_size(200_000);
    engine.set_max_map_size(200_000);
    engine.set_max_variables(256);
    engine.disable_symbol("eval");
    // No filesystem, network, process or module resolver is exposed to scripts.
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    let started = Instant::now();
    engine.on_progress(move |_| {
        (cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(5))
            .then(|| Dynamic::from("Plugin cancelled or timed out"))
    });
    engine
}

pub(crate) fn compile(source: &str) -> Result<Program<'static>, Error> {
    let engine = engine(Arc::default());
    // Avoid constant-folding user expressions during loading; runtime limits govern execution.
    let mut engine = engine;
    engine.set_optimization_level(rhai::OptimizationLevel::None);
    let ast = engine.compile(source).map_err(|e| Error(e.to_string()))?;
    Ok(Compiler::new().compile(&ast))
}

pub(crate) fn execute(
    program: &Program,
    input: Input,
    options: &std::collections::BTreeMap<String, serde_json::Value>,
    cancel: Arc<AtomicBool>,
) -> Result<String, Error> {
    if input.text.len() > MAX_TEXT_BYTES {
        return Err(Error("Plugin input exceeds 8 MiB".into()));
    }
    let mut settings = Map::new();
    for (key, value) in options {
        let value = match value {
            serde_json::Value::Bool(value) => Dynamic::from(*value),
            serde_json::Value::String(value) => Dynamic::from(value.clone()),
            serde_json::Value::Number(value) => {
                value.as_i64().map(Dynamic::from).ok_or_else(|| {
                    Error("Plugin options must contain integers, strings or booleans".into())
                })?
            }
            _ => {
                return Err(Error(
                    "Plugin options must contain integers, strings or booleans".into(),
                ));
            }
        };
        settings.insert(key.into(), value);
    }
    let mut scope = Scope::new();
    scope.push("text", input.text);
    scope.push_constant("line_ending", input.line_ending);
    scope.push_constant("command", input.command);
    scope.push_constant("options", settings);
    let engine = engine(cancel);
    let value = Vm::new(&engine)
        .eval_with_scope(&mut scope, program)
        .map_err(|e| Error(e.to_string()))?;
    let result = value
        .try_cast::<ImmutableString>()
        .ok_or_else(|| Error("A text plugin must return a string".into()))?;
    if result.len() > MAX_TEXT_BYTES {
        return Err(Error("Plugin output exceeds 8 MiB".into()));
    }
    Ok(result.to_string())
}

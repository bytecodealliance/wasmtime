use clap::Parser;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use wasmtime::component::{Component, Linker, bindgen};
use wasmtime::{Engine, Store, format_err};

// Generates bindings for the plugin world defined in the wit/calculator.wit file.
bindgen!("plugin");

/// A CLI that implements a plugin-based calculator whose
/// operations are implemented by independent WebAssembly components.

#[derive(Parser)]
struct BinaryOperation {
    /// The name of the operation
    op: String,
    /// The first operand
    x: i32,
    /// The second operand
    y: i32,
}

fn lookup_plugin<'a>(
    state: &'a mut CalculatorState,
    op: &str,
) -> wasmtime::Result<&'a mut PluginDesc> {
    state
        .plugin_descs
        .get_mut(op)
        .ok_or(format_err!("unknown operation {}", op))
}

impl BinaryOperation {
    fn run(self, state: &mut CalculatorState) -> wasmtime::Result<()> {
        let desc = lookup_plugin(state, self.op.as_ref())?;
        let result = desc.plugin.call_evaluate(&mut desc.store, self.x, self.y)?;
        println!("{}({}, {}) = {}", self.op, self.x, self.y, result);
        Ok(())
    }
}

pub struct CalculatorState {
    // Mapping from operation names to descriptors for the plugins
    pub plugin_descs: HashMap<String, PluginDesc>,
}

impl Default for CalculatorState {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorState {
    pub fn new() -> Self {
        CalculatorState {
            plugin_descs: HashMap::new(),
        }
    }
}

pub struct PluginDesc {
    pub plugin: Plugin,
    pub store: wasmtime::Store<()>,
}

fn load_plugin(
    state: &mut CalculatorState,
    engine: &Engine,
    linker: &Linker<()>,
    path: PathBuf,
) -> wasmtime::Result<()> {
    println!("Loading plugin from file {:?}", path);

    // Creates a component from a .wasm file
    let component = Component::from_file(engine, &path)?;

    let (plugin_name, plugin, store) = {
        // Creates a Store for each plugin.
        // A Store represents dynamic state, so each plugin
        // has its own Store.
        let mut store = Store::new(engine, ());
        // Instantiate the plugin to the store we just created.
        let plugin = Plugin::instantiate(&mut store, &component, linker)?;
        (plugin.call_get_plugin_name(&mut store)?, plugin, store)
    };

    state
        .plugin_descs
        .insert(plugin_name, PluginDesc { plugin, store });

    Ok(())
}

fn load_plugins(state: &mut CalculatorState, plugins_dir: &Path) -> wasmtime::Result<()> {
    let engine = Engine::default();
    let linker: Linker<()> = Linker::new(&engine);

    if !plugins_dir.is_dir() {
        wasmtime::bail!("plugins directory does not exist");
    }

    for entry in fs::read_dir(plugins_dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("wasm") {
            load_plugin(state, &engine, &linker, path)?;
        }
    }
    Ok(())
}

#[derive(Parser)]
#[command(name = "calculator-host", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A calculator with plugin support")]
struct Args {
    #[command(flatten)]
    op: BinaryOperation,

    #[arg(long, help = "Plugin directory")]
    plugins: PathBuf,
}

fn main() -> wasmtime::Result<()> {
    // Get plugins directory
    let args = Args::parse();

    // Initialize mapping from plugin names to plugins
    let mut state = CalculatorState::new();

    // Load plugins from plugins directory
    load_plugins(&mut state, args.plugins.as_path())?;

    // Evaluate the expression given on the command line
    args.op.run(&mut state)
}

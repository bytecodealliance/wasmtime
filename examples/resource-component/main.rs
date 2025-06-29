//! Example of instantiating a WASIp2 component with the use of resource

/*
You can execute this example with:
    cmake examples/
    cargo run --example resource-component
*/

use std::collections::HashMap;

use wasmtime::component::bindgen;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::component::{HasSelf, Resource};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_wasi::p2::add_to_linker_async;
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

pub struct ComponentRunStates {
    // These two are required basically as a standard way to enable the impl of IoView and
    // WasiView.
    // impl of WasiView is required by [`wasmtime_wasi::p2::add_to_linker_sync`]
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    // You can add other custom host states if needed
}

impl IoView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}
impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl ComponentRunStates {
    pub fn new() -> Self {
        // Create a WASI context and put it in a Store; all instances in the store
        // share this context. `WasiCtxBuilder` provides a number of ways to
        // configure what the target program will have access to.
        ComponentRunStates {
            wasi_ctx: WasiCtxBuilder::new().build(),
            resource_table: ResourceTable::new(),
        }
    }
}

bindgen!({
    path: "./examples/resource-component/kv-store.wit",
    world: "kv-database",
    async: true,
    with: {
        "example:kv-store/kvdb/connection": Connection
    },
    // Interactions with `ResourceTable` can possibly trap so enable the ability
    // to return traps from generated functions.
    trappable_imports: true,
});

pub struct Connection {
    pub storage: HashMap<String, String>,
}

impl KvDatabaseImports for ComponentRunStates {
    async fn log(&mut self, msg: String) -> Result<(), wasmtime::Error> {
        // provide host function to the component
        println!("Log: {msg}");
        Ok(())
    }
}

impl example::kv_store::kvdb::Host for ComponentRunStates {}

impl example::kv_store::kvdb::HostConnection for ComponentRunStates {
    async fn new(&mut self) -> Result<Resource<Connection>, wasmtime::Error> {
        Ok(self.resource_table.push(Connection {
            storage: HashMap::new(),
        })?)
    }

    async fn get(
        &mut self,
        resource: Resource<Connection>,
        key: String,
    ) -> Result<Option<String>, wasmtime::Error> {
        let connection = self.resource_table.get(&resource)?;
        Ok(connection.storage.get(&key).cloned())
    }

    async fn set(
        &mut self,
        resource: Resource<Connection>,
        key: String,
        value: String,
    ) -> Result<()> {
        let connection = self.resource_table.get_mut(&resource)?;
        connection.storage.insert(key, value);
        Ok(())
    }

    async fn remove(
        &mut self,
        resource: Resource<Connection>,
        key: String,
    ) -> Result<Option<String>> {
        let connection = self.resource_table.get_mut(&resource)?;
        Ok(connection.storage.remove(&key))
    }

    async fn clear(&mut self, resource: Resource<Connection>) -> Result<(), wasmtime::Error> {
        let large_string = self.resource_table.get_mut(&resource)?;
        large_string.storage.clear();
        Ok(())
    }

    async fn drop(&mut self, resource: Resource<Connection>) -> Result<()> {
        let _ = self.resource_table.delete(resource)?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Construct the wasm engine with async support enabled.
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);
    let state = ComponentRunStates::new();
    let mut store = Store::new(&engine, state);

    KvDatabase::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    add_to_linker_async(&mut linker)?;

    // Instantiate our component with the imports we've created, and run its function
    let component = Component::from_file(&engine, "target/wasm32-wasip2/debug/guest_kvdb.wasm")?;
    let bindings = KvDatabase::instantiate_async(&mut store, &component, &linker).await?;
    let result = bindings
        .call_replace_value(&mut store, "hello", "world")
        .await?;
    assert_eq!(result, None);
    let result = bindings
        .call_replace_value(&mut store, "hello", "wasmtime")
        .await?;
    assert_eq!(result, Some("world".to_string()));
    Ok(())
}

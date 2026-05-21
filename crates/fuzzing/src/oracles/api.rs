use crate::generators::api::{ApiCall, ApiCalls};
use crate::oracles::{StoreLimits, instantiate_with_dummy, log_wasm};
use std::collections::HashMap;
use wasmtime::*;

/// Invoke the given API calls.
pub fn make_api_calls(api: ApiCalls) {
    // The generator always starts with StoreNew; use its config to build the
    // shared engine that all stores in this sequence will use.
    let engine = match api.calls.first() {
        Some(ApiCall::StoreNew { config, .. }) => Engine::new(&config.to_wasmtime()).unwrap(),
        _ => return,
    };

    let mut stores: HashMap<usize, Store<StoreLimits>> = Default::default();
    let mut modules: HashMap<usize, Module> = Default::default();
    let mut instances: HashMap<usize, (Instance, usize)> = Default::default();
    let mut global_types: HashMap<usize, GlobalType> = Default::default();
    let mut globals: HashMap<usize, (Global, usize)> = Default::default();
    let mut table_types: HashMap<usize, TableType> = Default::default();
    let mut tables: HashMap<usize, (Table, usize)> = Default::default();

    for call in api.calls {
        match call {
            ApiCall::StoreNew { id, config } => {
                log::trace!("creating store {id}");
                let mut store = Store::new(&engine, StoreLimits::new());
                config.configure_store(&mut store);
                let old = stores.insert(id, store);
                assert!(old.is_none());
            }

            ApiCall::StoreDrop { id } => {
                log::trace!("dropping store {id}");
                instances.retain(|_, (_, store_id)| *store_id != id);
                globals.retain(|_, (_, store_id)| *store_id != id);
                tables.retain(|_, (_, store_id)| *store_id != id);
                stores.remove(&id);
            }

            ApiCall::ModuleNew { id, wasm } => {
                log::debug!("creating module: {id}");
                log_wasm(&wasm);
                let module = match Module::new(&engine, &wasm) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let old = modules.insert(id, module);
                assert!(old.is_none());
            }

            ApiCall::ModuleDrop { id } => {
                log::trace!("dropping module: {id}");
                drop(modules.remove(&id));
            }

            ApiCall::InstanceNew { id, module, store } => {
                log::trace!("instantiating module {module} as {id} in store {store}");
                let module = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                if let Some(instance) = instantiate_with_dummy(st, module) {
                    instances.insert(id, (instance, store));
                }
            }

            ApiCall::InstanceDrop { id } => {
                log::trace!("dropping instance {id}");
                instances.remove(&id);
            }

            ApiCall::CallExportedFunc { instance, nth } => {
                log::trace!("calling instance export {instance} / {nth}");
                let (inst, store_id) = match instances.get(&instance) {
                    Some(&x) => x,
                    None => {
                        // Note that we aren't guaranteed to instantiate valid
                        // modules, see comments in `InstanceNew` for details on
                        // that. But the API call generator can't know if
                        // instantiation failed, so we might not actually have
                        // this instance. When that's the case, just skip the
                        // API call and keep going.
                        continue;
                    }
                };
                let store = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };

                let funcs = inst
                    .exports(&mut *store)
                    .filter_map(|e| match e.into_extern() {
                        Extern::Func(f) => Some(f),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if funcs.is_empty() {
                    continue;
                }

                let nth = nth % funcs.len();
                let f = &funcs[nth];
                let ty = f.ty(&*store);
                if let Some(params) = ty
                    .params()
                    .map(|p| p.default_value())
                    .collect::<Option<Vec<_>>>()
                {
                    let mut results = vec![Val::I32(0); ty.results().len()];
                    let _ = f.call(store, &params, &mut results);
                }
            }

            ApiCall::GlobalTypeNew { id, ty, mutable } => {
                log::trace!("creating global type {id}");
                let mutability = if mutable {
                    Mutability::Var
                } else {
                    Mutability::Const
                };
                let old = global_types.insert(id, GlobalType::new(ty.to_wasmtime(), mutability));
                assert!(old.is_none());
            }

            ApiCall::GlobalTypeDrop { id } => {
                log::trace!("dropping global type {id}");
                global_types.remove(&id);
            }

            ApiCall::GlobalNew {
                id,
                global_ty,
                store,
            } => {
                log::trace!("creating global {id} with type {global_ty} in store {store}");
                let gt = match global_types.get(&global_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                match gt.default_value(&mut *st) {
                    Ok(g) => {
                        globals.insert(id, (g, store));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::GlobalGet { global } => {
                log::trace!("getting global {global}");
                let (g, store_id) = match globals.get(&global) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = g.get(&mut *st);
            }

            ApiCall::GlobalSet { global } => {
                log::trace!("setting global {global}");
                let (g, store_id) = match globals.get(&global) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                // ty() takes &Store (immutable reborrow); returns owned GlobalType.
                let gt = g.ty(&*st);
                if gt.mutability().is_const() {
                    continue;
                }
                if let Some(val) = gt.content().default_value() {
                    let _ = g.set(&mut *st, val);
                }
            }

            ApiCall::GlobalTy { global } => {
                log::trace!("checking type of global {global}");
                let (g, store_id) = match globals.get(&global) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = g.ty(st);
            }

            ApiCall::GlobalDrop { id } => {
                log::trace!("dropping global {id}");
                globals.remove(&id);
            }

            ApiCall::GetGlobalExport { id, instance, nth } => {
                log::trace!("getting {nth}th global export of instance {instance} as {id}");
                let (inst, store_id) = match instances.get(&instance) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let gs = inst
                    .exports(&mut *st)
                    .filter_map(|e| e.into_global())
                    .collect::<Vec<_>>();
                if gs.is_empty() {
                    continue;
                }
                globals.insert(id, (gs[nth % gs.len()], store_id));
            }

            ApiCall::TableTypeNew { id, nullable } => {
                log::trace!("creating table type {id}");
                let element = RefType::new(nullable, HeapType::Func);
                let old = table_types.insert(id, TableType::new(element, 0, None));
                assert!(old.is_none());
            }

            ApiCall::TableTypeDrop { id } => {
                log::trace!("dropping table type {id}");
                table_types.remove(&id);
            }

            ApiCall::TableNew {
                id,
                table_ty,
                store,
            } => {
                log::trace!("creating table {id} with type {table_ty} in store {store}");
                let tt = match table_types.get(&table_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                match tt.default_value(&mut *st) {
                    Ok(t) => {
                        tables.insert(id, (t, store));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::TableGet { table, idx } => {
                log::trace!("getting table {table} at index {idx}");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = t.get(&mut *st, idx);
            }

            ApiCall::TableSet { table, idx } => {
                log::trace!("setting table {table} at index {idx}");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ty = t.ty(&*st);
                let val: ValType = ty.element().clone().into();
                if let Some(init) = val.default_value() {
                    let _ = t.set(&mut *st, idx, init.ref_().unwrap());
                }
            }

            ApiCall::TableGrow { table, delta } => {
                log::trace!("growing table {table} by {delta}");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ty = t.ty(&*st);
                let val: ValType = ty.element().clone().into();
                if let Some(init) = val.default_value() {
                    let _ = t.grow(&mut *st, delta.into(), init.ref_().unwrap());
                }
            }

            ApiCall::TableSize { table } => {
                log::trace!("getting size of table {table}");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = t.size(st);
            }

            ApiCall::TableTy { table } => {
                log::trace!("checking type of table {table}");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = t.ty(st);
            }

            ApiCall::TableDrop { id } => {
                log::trace!("dropping table {id}");
                tables.remove(&id);
            }

            ApiCall::GetTableExport { id, instance, nth } => {
                log::trace!("getting {nth}th table export of instance {instance} as {id}");
                let (inst, store_id) = match instances.get(&instance) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ts = inst
                    .exports(&mut *st)
                    .filter_map(|e| e.into_table())
                    .collect::<Vec<_>>();
                if ts.is_empty() {
                    continue;
                }
                tables.insert(id, (ts[nth % ts.len()], store_id));
            }
        }
    }
}

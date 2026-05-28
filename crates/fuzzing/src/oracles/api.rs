use crate::generators::api::{ApiCall, ApiCalls};
use crate::oracles::{StoreLimits, instantiate_with_dummy, log_wasm};
use std::collections::HashMap;
use wasmtime::*;

/// Invoke the given API calls.
pub fn make_api_calls(api: ApiCalls) {
    // The generator always starts with StoreNew; use its config to build the
    // shared engine that all stores in this sequence will use.
    let (engine, gc_enabled) = match api.calls.first() {
        Some(ApiCall::StoreNew { config, .. }) => (
            Engine::new(&config.to_wasmtime()).unwrap(),
            config.module_config.config.gc_enabled,
        ),
        _ => return,
    };

    let mut stores: HashMap<usize, Store<StoreLimits>> = Default::default();
    let mut modules: HashMap<usize, Module> = Default::default();
    let mut instances: HashMap<usize, (Instance, usize)> = Default::default();
    let mut global_types: HashMap<usize, GlobalType> = Default::default();
    let mut globals: HashMap<usize, (Global, usize)> = Default::default();
    let mut table_types: HashMap<usize, TableType> = Default::default();
    let mut tables: HashMap<usize, (Table, usize)> = Default::default();
    let mut memory_types: HashMap<usize, MemoryType> = Default::default();
    let mut memories: HashMap<usize, (Memory, usize)> = Default::default();
    let mut func_types: HashMap<usize, FuncType> = Default::default();
    let mut funcs: HashMap<usize, (Func, usize)> = Default::default();
    let mut tag_types: HashMap<usize, TagType> = Default::default();
    let mut tags: HashMap<usize, (Tag, usize)> = Default::default();
    let mut struct_types: HashMap<usize, StructType> = Default::default();
    let mut struct_ref_pres: HashMap<usize, (StructRefPre, StructType, usize)> = Default::default();
    let mut struct_refs: HashMap<usize, (OwnedRooted<StructRef>, usize)> = Default::default();
    let mut array_types: HashMap<usize, ArrayType> = Default::default();
    let mut array_ref_pres: HashMap<usize, (ArrayRefPre, ArrayType, usize)> = Default::default();
    let mut array_refs: HashMap<usize, (OwnedRooted<ArrayRef>, usize)> = Default::default();
    let mut exn_types: HashMap<usize, ExnType> = Default::default();
    let mut exn_ref_pres: HashMap<usize, (ExnRefPre, ExnType, usize)> = Default::default();
    let mut exn_refs: HashMap<usize, (OwnedRooted<ExnRef>, usize)> = Default::default();

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
                memories.retain(|_, (_, store_id)| *store_id != id);
                funcs.retain(|_, (_, store_id)| *store_id != id);
                tags.retain(|_, (_, store_id)| *store_id != id);
                struct_ref_pres.retain(|_, (_, _, store_id)| *store_id != id);
                struct_refs.retain(|_, (_, store_id)| *store_id != id);
                array_ref_pres.retain(|_, (_, _, store_id)| *store_id != id);
                array_refs.retain(|_, (_, store_id)| *store_id != id);
                exn_ref_pres.retain(|_, (_, _, store_id)| *store_id != id);
                exn_refs.retain(|_, (_, store_id)| *store_id != id);
                stores.remove(&id);
            }

            ApiCall::StoreGc { id } => {
                log::trace!("collecting garbage in store {id}");
                let st = match stores.get_mut(&id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = st.gc(None);
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

            ApiCall::MemoryTypeNew {
                id,
                minimum,
                maximum,
            } => {
                log::trace!("creating memory type {id}");
                let old = memory_types.insert(id, MemoryType::new(minimum, maximum));
                assert!(old.is_none());
            }

            ApiCall::MemoryTypeDrop { id } => {
                log::trace!("dropping memory type {id}");
                memory_types.remove(&id);
            }

            ApiCall::MemoryNew {
                id,
                memory_ty,
                store,
            } => {
                log::trace!("creating memory {id} with type {memory_ty} in store {store}");
                let mt = match memory_types.get(&memory_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                match Memory::new(&mut *st, mt) {
                    Ok(m) => {
                        memories.insert(id, (m, store));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::MemoryRead {
                memory,
                offset,
                len,
            } => {
                log::trace!("reading {len} bytes from memory {memory} at offset {offset}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let mut buf = vec![0u8; len];
                let _ = m.read(st, offset, &mut buf);
            }

            ApiCall::MemoryWrite {
                memory,
                offset,
                ref data,
            } => {
                log::trace!(
                    "writing {} bytes to memory {memory} at offset {offset}",
                    data.len()
                );
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.write(&mut *st, offset, data);
            }

            ApiCall::MemoryData { memory } => {
                log::trace!("getting data slice of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.data(st);
            }

            ApiCall::MemoryDataMut { memory } => {
                log::trace!("getting mutable data slice of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.data_mut(&mut *st);
            }

            ApiCall::MemoryGrow { memory, delta } => {
                log::trace!("growing memory {memory} by {delta} pages");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.grow(&mut *st, delta.into());
            }

            ApiCall::MemoryDataSize { memory } => {
                log::trace!("getting data size of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.data_size(st);
            }

            ApiCall::MemorySize { memory } => {
                log::trace!("getting size of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.size(st);
            }

            ApiCall::MemoryPageSize { memory } => {
                log::trace!("getting page size of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.page_size(st);
            }

            ApiCall::MemoryPageSizeLog2 { memory } => {
                log::trace!("getting page size log2 of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.page_size_log2(st);
            }

            ApiCall::MemoryTy { memory } => {
                log::trace!("checking type of memory {memory}");
                let (m, store_id) = match memories.get(&memory) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = m.ty(st);
            }

            ApiCall::MemoryDrop { id } => {
                log::trace!("dropping memory {id}");
                memories.remove(&id);
            }

            ApiCall::GetMemoryExport { id, instance, nth } => {
                log::trace!("getting {nth}th memory export of instance {instance} as {id}");
                let (inst, store_id) = match instances.get(&instance) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ms = inst
                    .exports(&mut *st)
                    .filter_map(|e| e.into_memory())
                    .collect::<Vec<_>>();
                if ms.is_empty() {
                    continue;
                }
                memories.insert(id, (ms[nth % ms.len()], store_id));
            }

            ApiCall::FuncTypeNew {
                id,
                ref params,
                ref results,
            } => {
                log::trace!("creating func type {id}");
                let param_tys = params.iter().map(|p| p.to_wasmtime());
                let result_tys = results.iter().map(|r| r.to_wasmtime());
                let ft = FuncType::new(&engine, param_tys, result_tys);
                let old = func_types.insert(id, ft);
                assert!(old.is_none());
            }

            ApiCall::FuncTypeDrop { id } => {
                log::trace!("dropping func type {id}");
                func_types.remove(&id);
            }

            ApiCall::FuncNew { id, func_ty, store } => {
                log::trace!("creating func {id} with type {func_ty} in store {store}");
                let ft = match func_types.get(&func_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                let f = Func::new(&mut *st, ft, |_caller, _params, _results| Ok(()));
                funcs.insert(id, (f, store));
            }

            ApiCall::FuncTy { func } => {
                log::trace!("checking type of func {func}");
                let (f, store_id) = match funcs.get(&func) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = f.ty(&*st);
            }

            ApiCall::FuncCall { func } => {
                log::trace!("calling func {func}");
                let (f, store_id) = match funcs.get(&func) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ty = f.ty(&*st);
                if let Some(params) = ty
                    .params()
                    .map(|p| p.default_value())
                    .collect::<Option<Vec<_>>>()
                {
                    let mut results = vec![Val::I32(0); ty.results().len()];
                    let _ = f.call(st, &params, &mut results);
                }
            }

            ApiCall::GetFuncExport { id, instance, nth } => {
                log::trace!("getting {nth}th func export of instance {instance} as {id}");
                let (inst, store_id) = match instances.get(&instance) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let fs = inst
                    .exports(&mut *st)
                    .filter_map(|e| match e.into_extern() {
                        Extern::Func(f) => Some(f),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if fs.is_empty() {
                    continue;
                }
                funcs.insert(id, (fs[nth % fs.len()], store_id));
            }

            ApiCall::FuncDrop { id } => {
                log::trace!("dropping func {id}");
                funcs.remove(&id);
            }

            ApiCall::TagTypeNew { id, func_ty } => {
                log::trace!("creating tag type {id} from func type {func_ty}");
                let ft = match func_types.get(&func_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let old = tag_types.insert(id, TagType::new(ft));
                assert!(old.is_none());
            }

            ApiCall::TagTypeDrop { id } => {
                log::trace!("dropping tag type {id}");
                tag_types.remove(&id);
            }

            ApiCall::TagNew { id, tag_ty, store } => {
                log::trace!("creating tag {id} with type {tag_ty} in store {store}");
                let tt = match tag_types.get(&tag_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                if !gc_enabled {
                    continue;
                }
                match Tag::new(&mut *st, &tt) {
                    Ok(t) => {
                        tags.insert(id, (t, store));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::TagTy { tag } => {
                log::trace!("checking type of tag {tag}");
                let (t, store_id) = match tags.get(&tag) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = t.ty(st);
            }

            ApiCall::TagEq { a, b } => {
                log::trace!("comparing tags {a} and {b}");
                let (ta, store_id) = match tags.get(&a) {
                    Some(&x) => x,
                    None => continue,
                };
                let (tb, store_id_b) = match tags.get(&b) {
                    Some(&x) => x,
                    None => continue,
                };
                if store_id != store_id_b {
                    continue;
                }
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = Tag::eq(&ta, &tb, st);
            }

            ApiCall::GetTagExport { id, instance, nth } => {
                log::trace!("getting {nth}th tag export of instance {instance} as {id}");
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
                    .filter_map(|e| e.into_tag())
                    .collect::<Vec<_>>();
                if ts.is_empty() {
                    continue;
                }
                tags.insert(id, (ts[nth % ts.len()], store_id));
            }

            ApiCall::TagDrop { id } => {
                log::trace!("dropping tag {id}");
                tags.remove(&id);
            }

            ApiCall::ModuleImports { module } => {
                log::trace!("iterating imports of module {module}");
                let m = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };
                for _ in m.imports() {}
            }

            ApiCall::ModuleExports { module } => {
                log::trace!("iterating exports of module {module}");
                let m = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };
                for _ in m.exports() {}
            }

            ApiCall::ModuleGetExport { module, ref name } => {
                log::trace!("getting export {name:?} of module {module}");
                let m = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };
                let _ = m.get_export(name);
            }

            ApiCall::ModuleName { module } => {
                log::trace!("getting name of module {module}");
                let m = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };
                let _ = m.name();
            }

            ApiCall::ModuleValidate { ref wasm } => {
                log::trace!("validating {} bytes of wasm", wasm.len());
                let _ = Module::validate(&engine, wasm);
            }

            ApiCall::ModuleSerializeDeserialize { src_id, dst_id } => {
                log::trace!("serializing module {src_id} and deserializing as {dst_id}");
                let src = match modules.get(&src_id) {
                    Some(m) => m,
                    None => continue,
                };
                let bytes = match src.serialize() {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                match unsafe { Module::deserialize(&engine, &bytes) } {
                    Ok(m) => {
                        modules.insert(dst_id, m);
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::TableCopy {
                dst_table,
                dst_index,
                src_table,
                src_index,
                len,
            } => {
                log::trace!(
                    "copying table {src_table}[{src_index}..+{len}] to {dst_table}[{dst_index}]"
                );
                let (dt, dst_store_id) = match tables.get(&dst_table) {
                    Some(&x) => x,
                    None => continue,
                };
                let (st_tbl, src_store_id) = match tables.get(&src_table) {
                    Some(&x) => x,
                    None => continue,
                };
                if dst_store_id != src_store_id {
                    continue;
                }
                let st = match stores.get_mut(&dst_store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = Table::copy(&mut *st, &dt, dst_index, &st_tbl, src_index, len);
            }

            ApiCall::TableFill { table, dst, len } => {
                log::trace!("filling table {table}[{dst}..+{len}]");
                let (t, store_id) = match tables.get(&table) {
                    Some(&x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let ty = t.ty(&*st);
                let elem_ty: ValType = ty.element().clone().into();
                if let Some(val) = elem_ty.default_value() {
                    let _ = t.fill(&mut *st, dst, val.ref_().unwrap(), len);
                }
            }

            ApiCall::StructTypeNew { id, ref fields } => {
                log::trace!("creating struct type {id}");
                if !gc_enabled {
                    continue;
                }
                let field_types = fields.iter().map(|(fvt, mutable)| {
                    let mutability = if *mutable {
                        Mutability::Var
                    } else {
                        Mutability::Const
                    };
                    FieldType::new(mutability, StorageType::ValType(fvt.to_wasmtime()))
                });
                match StructType::new(&engine, field_types) {
                    Ok(st) => {
                        struct_types.insert(id, st);
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::StructTypeDrop { id } => {
                log::trace!("dropping struct type {id}");
                struct_types.remove(&id);
            }

            ApiCall::StructRefPreNew {
                id,
                struct_ty,
                store,
            } => {
                log::trace!("creating struct ref pre {id} with type {struct_ty} in store {store}");
                let sty = match struct_types.get(&struct_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                if !gc_enabled {
                    continue;
                }
                let pre = StructRefPre::new(&mut *st, sty.clone());
                struct_ref_pres.insert(id, (pre, sty, store));
            }

            ApiCall::StructRefPreDrop { id } => {
                log::trace!("dropping struct ref pre {id}");
                struct_ref_pres.remove(&id);
            }

            ApiCall::StructRefNew { id, pre } => {
                log::trace!("creating struct ref {id} from pre {pre}");
                let (allocator, sty, store_id) = match struct_ref_pres.get(&pre) {
                    Some(x) => x,
                    None => continue,
                };
                let store_id = *store_id;
                let fields: Option<Vec<Val>> = sty
                    .fields()
                    .map(|f| f.element_type().unpack().default_value())
                    .collect();
                let fields = match fields {
                    Some(f) => f,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let mut st = RootScope::new(st);
                match StructRef::new(&mut st, allocator, &fields) {
                    Ok(r) => {
                        struct_refs.insert(id, (r.to_owned_rooted(st).unwrap(), store_id));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::StructRefTy { struct_ref } => {
                log::trace!("getting type of struct ref {struct_ref}");
                let (r, store_id) = match struct_refs.get(&struct_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.ty(st);
            }

            ApiCall::StructRefField { struct_ref, index } => {
                log::trace!("getting field {index} of struct ref {struct_ref}");
                let (r, store_id) = match struct_refs.get(&struct_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.field(&mut *st, index);
            }

            ApiCall::StructRefSetField { struct_ref, index } => {
                log::trace!("setting field {index} of struct ref {struct_ref}");
                let (r, store_id) = match struct_refs.get(&struct_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let sty = match r.ty(&*st) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let field_tys: Vec<_> = sty.fields().collect();
                if index >= field_tys.len() {
                    continue;
                }
                let val = match field_tys[index].element_type().unpack().default_value() {
                    Some(v) => v,
                    None => continue,
                };
                let _ = r.set_field(&mut *st, index, val);
            }

            ApiCall::StructRefDrop { id } => {
                log::trace!("dropping struct ref {id}");
                struct_refs.remove(&id);
            }

            ApiCall::ArrayTypeNew {
                id,
                elem_ty,
                mutable,
            } => {
                log::trace!("creating array type {id}");
                if !gc_enabled {
                    continue;
                }
                let mutability = if mutable {
                    Mutability::Var
                } else {
                    Mutability::Const
                };
                let ft = FieldType::new(mutability, StorageType::ValType(elem_ty.to_wasmtime()));
                let at = ArrayType::new(&engine, ft);
                array_types.insert(id, at);
            }

            ApiCall::ArrayTypeDrop { id } => {
                log::trace!("dropping array type {id}");
                array_types.remove(&id);
            }

            ApiCall::ArrayRefPreNew {
                id,
                array_ty,
                store,
            } => {
                log::trace!("creating array ref pre {id} with type {array_ty} in store {store}");
                let aty = match array_types.get(&array_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                if !gc_enabled {
                    continue;
                }
                let pre = ArrayRefPre::new(&mut *st, aty.clone());
                array_ref_pres.insert(id, (pre, aty, store));
            }

            ApiCall::ArrayRefPreDrop { id } => {
                log::trace!("dropping array ref pre {id}");
                array_ref_pres.remove(&id);
            }

            ApiCall::ArrayRefNew { id, pre, len } => {
                log::trace!("creating array ref {id} from pre {pre} with len {len}");
                let (allocator, aty, store_id) = match array_ref_pres.get(&pre) {
                    Some(x) => x,
                    None => continue,
                };
                let store_id = *store_id;
                let len = len % 17;
                let elem_val = match aty.field_type().element_type().unpack().default_value() {
                    Some(v) => v,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let mut st = RootScope::new(st);
                match ArrayRef::new(&mut st, allocator, &elem_val, len) {
                    Ok(r) => {
                        array_refs.insert(id, (r.to_owned_rooted(st).unwrap(), store_id));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::ArrayRefNewFixed { id, pre, count } => {
                log::trace!("creating array ref {id} from pre {pre} with {count} elements");
                let (allocator, aty, store_id) = match array_ref_pres.get(&pre) {
                    Some(x) => x,
                    None => continue,
                };
                let store_id = *store_id;
                let count = count % 9;
                let elem_val = match aty.field_type().element_type().unpack().default_value() {
                    Some(v) => v,
                    None => continue,
                };
                let elems: Vec<Val> = (0..count).map(|_| elem_val).collect();
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let mut st = RootScope::new(st);
                match ArrayRef::new_fixed(&mut st, allocator, &elems) {
                    Ok(r) => {
                        array_refs.insert(id, (r.to_owned_rooted(st).unwrap(), store_id));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::ArrayRefTy { array_ref } => {
                log::trace!("getting type of array ref {array_ref}");
                let (r, store_id) = match array_refs.get(&array_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.ty(st);
            }

            ApiCall::ArrayRefLen { array_ref } => {
                log::trace!("getting length of array ref {array_ref}");
                let (r, store_id) = match array_refs.get(&array_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.len(st);
            }

            ApiCall::ArrayRefGet { array_ref, index } => {
                log::trace!("getting index {index} of array ref {array_ref}");
                let (r, store_id) = match array_refs.get(&array_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.get(&mut *st, index);
            }

            ApiCall::ArrayRefSet { array_ref, index } => {
                log::trace!("setting index {index} of array ref {array_ref}");
                let (r, store_id) = match array_refs.get(&array_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let aty = match r.ty(&*st) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let val = match aty.field_type().element_type().unpack().default_value() {
                    Some(v) => v,
                    None => continue,
                };
                let _ = r.set(&mut *st, index, val);
            }

            ApiCall::ArrayRefDrop { id } => {
                log::trace!("dropping array ref {id}");
                array_refs.remove(&id);
            }

            ApiCall::ExnTypeNew { id, ref fields } => {
                log::trace!("creating exn type {id}");
                if !gc_enabled {
                    continue;
                }
                match ExnType::new(&engine, fields.iter().map(|f| f.to_wasmtime())) {
                    Ok(et) => {
                        exn_types.insert(id, et);
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::ExnTypeFromTagType { id, tag_ty } => {
                log::trace!("creating exn type {id} from tag type {tag_ty}");
                if !gc_enabled {
                    continue;
                }
                let tt = match tag_types.get(&tag_ty) {
                    Some(t) => t,
                    None => continue,
                };
                match ExnType::from_tag_type(tt) {
                    Ok(et) => {
                        exn_types.insert(id, et);
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::ExnTypeDrop { id } => {
                log::trace!("dropping exn type {id}");
                exn_types.remove(&id);
            }

            ApiCall::ExnRefPreNew { id, exn_ty, store } => {
                log::trace!("creating exn ref pre {id} with type {exn_ty} in store {store}");
                let ety = match exn_types.get(&exn_ty) {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let st = match stores.get_mut(&store) {
                    Some(s) => s,
                    None => continue,
                };
                if !gc_enabled {
                    continue;
                }
                let pre = ExnRefPre::new(&mut *st, ety.clone());
                exn_ref_pres.insert(id, (pre, ety, store));
            }

            ApiCall::ExnRefPreDrop { id } => {
                log::trace!("dropping exn ref pre {id}");
                exn_ref_pres.remove(&id);
            }

            ApiCall::ExnRefNew { id, pre, tag } => {
                log::trace!("creating exn ref {id} from pre {pre} with tag {tag}");
                let (allocator, ety, store_id) = match exn_ref_pres.get(&pre) {
                    Some(x) => x,
                    None => continue,
                };
                let store_id = *store_id;
                let (t, tag_store_id) = match tags.get(&tag) {
                    Some(&x) => x,
                    None => continue,
                };
                if store_id != tag_store_id {
                    continue;
                }
                let fields: Option<Vec<Val>> = ety
                    .fields()
                    .map(|f| f.element_type().unpack().default_value())
                    .collect();
                let fields = match fields {
                    Some(f) => f,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let mut st = RootScope::new(st);
                match ExnRef::new(&mut st, allocator, &t, &fields) {
                    Ok(r) => {
                        exn_refs.insert(id, (r.to_owned_rooted(st).unwrap(), store_id));
                    }
                    Err(_) => continue,
                }
            }

            ApiCall::ExnRefTy { exn_ref } => {
                log::trace!("getting type of exn ref {exn_ref}");
                let (r, store_id) = match exn_refs.get(&exn_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.ty(st);
            }

            ApiCall::ExnRefTag { exn_ref } => {
                log::trace!("getting tag of exn ref {exn_ref}");
                let (r, store_id) = match exn_refs.get(&exn_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.tag(&mut *st);
            }

            ApiCall::ExnRefField { exn_ref, index } => {
                log::trace!("getting field {index} of exn ref {exn_ref}");
                let (r, store_id) = match exn_refs.get(&exn_ref) {
                    Some(x) => x,
                    None => continue,
                };
                let st = match stores.get_mut(&store_id) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = r.field(&mut *st, index);
            }

            ApiCall::ExnRefDrop { id } => {
                log::trace!("dropping exn ref {id}");
                exn_refs.remove(&id);
            }
        }
    }
}

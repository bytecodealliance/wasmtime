#![cfg(all(not(target_os = "windows"), not(miri)))]
use anyhow::{Context, bail};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ops::Range,
    ptr::NonNull,
    sync::Arc,
};
use wasmtime::*;

fn align_up(v: usize, align: usize) -> usize {
    return (v + (align - 1)) & (!(align - 1));
}

struct CustomStack {
    base: NonNull<u8>,
    len: usize,
}
unsafe impl Send for CustomStack {}
unsafe impl Sync for CustomStack {}
impl CustomStack {
    fn new(base: NonNull<u8>, len: usize) -> Self {
        CustomStack { base, len }
    }
}
unsafe impl StackMemory for CustomStack {
    fn top(&self) -> *mut u8 {
        unsafe { self.base.as_ptr().add(self.len) }
    }
    fn range(&self) -> Range<usize> {
        let base = self.base.as_ptr() as usize;
        base..base + self.len
    }
    fn guard_range(&self) -> Range<*mut u8> {
        std::ptr::null_mut()..std::ptr::null_mut()
    }
}

// A creator that allocates stacks on the heap instead of mmap'ing.
struct CustomStackCreator {
    memory: NonNull<u8>,
    size: usize,
    layout: Layout,
}

unsafe impl Send for CustomStackCreator {}
unsafe impl Sync for CustomStackCreator {}
impl CustomStackCreator {
    fn new() -> Result<Self> {
        // 1MB
        const MINIMUM_STACK_SIZE: usize = 1 * 1_024 * 1_024;
        let page_size = rustix::param::page_size();
        let size = align_up(MINIMUM_STACK_SIZE, page_size);
        // Add an extra page for the guard page
        let layout = Layout::from_size_align(size + page_size, page_size)
            .context("unable to compute stack layout")?;
        let memory = unsafe {
            let mem = System.alloc(layout);
            let notnull = NonNull::new(mem);
            if let Some(mem) = notnull {
                // It's required that stack memory is zeroed for wasmtime
                libc::memset(mem.as_ptr().cast(), 0, layout.size());
                // Mark guard page as protected
                rustix::mm::mprotect(
                    mem.as_ptr().cast(),
                    page_size,
                    rustix::mm::MprotectFlags::empty(),
                )?;
            }
            notnull
        }
        .context("unable to allocate stack memory")?;
        Ok(CustomStackCreator {
            memory,
            size,
            layout,
        })
    }
    fn range(&self) -> Range<usize> {
        let page_size = rustix::param::page_size();
        let base = unsafe { self.memory.as_ptr().add(page_size) as usize };
        base..base + self.size
    }
}
impl Drop for CustomStackCreator {
    fn drop(&mut self) {
        let page_size = rustix::param::page_size();
        unsafe {
            // Unprotect the guard page as the allocator could reuse it.
            rustix::mm::mprotect(
                self.memory.as_ptr().cast(),
                page_size,
                rustix::mm::MprotectFlags::READ | rustix::mm::MprotectFlags::WRITE,
            )
            .unwrap();
            System.dealloc(self.memory.as_ptr(), self.layout);
        }
    }
}
unsafe impl StackCreator for CustomStackCreator {
    fn new_stack(&self, size: usize) -> Result<Box<dyn StackMemory>> {
        if size != self.size {
            bail!("must use the size we allocated for this stack memory creator");
        }
        let page_size = rustix::param::page_size();
        // skip over the page size
        let base_ptr = unsafe { self.memory.as_ptr().add(page_size) };
        let base = NonNull::new(base_ptr).context("unable to compute stack base")?;
        Ok(Box::new(CustomStack::new(base, self.size)))
    }
}

fn config() -> (Store<()>, Arc<CustomStackCreator>) {
    let stack_creator = Arc::new(CustomStackCreator::new().unwrap());
    let mut config = Config::new();
    config
        .async_support(true)
        .max_wasm_stack(stack_creator.size / 2)
        .async_stack_size(stack_creator.size)
        .with_host_stack(stack_creator.clone());
    (
        Store::new(&Engine::new(&config).unwrap(), ()),
        stack_creator,
    )
}

#[tokio::test]
async fn called_on_custom_heap_stack() -> Result<()> {
    let (mut store, stack_creator) = config();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "host" "callback" (func $callback (result i64)))
                (func $f (result i64) (call $callback))
                (export "f" (func $f))
            )
        "#,
    )?;

    let ty = FuncType::new(store.engine(), [], [ValType::I64]);
    let host_func = Func::new(&mut store, ty, move |_caller, _params, results| {
        let foo = 42;
        // output an address on the stack
        results[0] = Val::I64((&foo as *const i32) as usize as i64);
        Ok(())
    });
    let export = wasmtime::Extern::Func(host_func);
    let instance = Instance::new_async(&mut store, &module, &[export]).await?;
    let mut results = [Val::I64(0)];
    instance
        .get_func(&mut store, "f")
        .context("missing function export")?
        .call_async(&mut store, &[], &mut results)
        .await?;
    // Make sure the stack address we wrote was within our custom stack range
    let stack_address = results[0].i64().unwrap() as usize;
    assert!(stack_creator.range().contains(&stack_address));
    Ok(())
}

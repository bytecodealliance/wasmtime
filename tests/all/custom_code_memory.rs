#[cfg(all(not(target_os = "windows"), not(miri)))]
mod not_for_windows {
    use rustix::mm::{MprotectFlags, mprotect};
    use rustix::param::page_size;
    use std::sync::Arc;
    use wasmtime::*;

    struct CustomCodePublish;
    impl CustomCodeMemory for CustomCodePublish {
        fn required_alignment(&self) -> usize {
            page_size()
        }

        fn publish_executable(&self, ptr: *const u8, len: usize) -> anyhow::Result<()> {
            unsafe {
                mprotect(
                    ptr as *mut _,
                    len,
                    MprotectFlags::READ | MprotectFlags::EXEC,
                )?;
            }
            Ok(())
        }

        fn unpublish_executable(&self, ptr: *const u8, len: usize) -> anyhow::Result<()> {
            unsafe {
                mprotect(
                    ptr as *mut _,
                    len,
                    MprotectFlags::READ | MprotectFlags::WRITE,
                )?;
            }
            Ok(())
        }
    }

    #[test]
    fn custom_code_publish() {
        let mut config = Config::default();
        config.with_custom_code_memory(Some(Arc::new(CustomCodePublish)));
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(
            &engine,
            "(module (func (export \"main\") (result i32) i32.const 42))",
        )
        .unwrap();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let func: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "main").unwrap();
        let result = func.call(&mut store, ()).unwrap();
        assert_eq!(result, 42);
    }
}

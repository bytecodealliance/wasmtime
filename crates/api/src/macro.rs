#[macro_export]
macro_rules! wrap_wasmtime_func {
    ($store:expr; module( $module_name:path )) => {{
        use $module_name as m;
        let f = m::metadata();
        ::wasmtime_api::Func::from_raw($store, f.address, f.signature)
    }};
}

#[macro_export]
macro_rules! get_wasmtime_func {
    ($func:expr; module( $module_name:path )) => {{
        use $module_name as m;
        let (i, e) = $func.borrow().raw_parts();
        m::Wrapper::new(i, e)
    }};
}

#[macro_export]
macro_rules! map_to_wasmtime_trait {
    ($instance:expr; module( $module_name:path )) => {{
        use $module_name as m;
        let handle = $instance.borrow().handle().clone();
        m::Wrapper::new(handle)
    }};
}

#[macro_export]
macro_rules! wrap_wasmtime_module {
    ($store:expr, |$imports:ident| $factory:expr; module( $module_name:path )) => {{
        use $module_name as m;
        struct T;
        impl ::wasmtime_api::HandleStateBuilder for T {
            fn build_state(&self, imports: &[::wasmtime_api::Extern]) -> Box<dyn std::any::Any> {
                let imp = |$imports: &[::wasmtime_api::Extern]| $factory;
                let state = m::State {
                    subject: ::std::cell::RefCell::new(::std::boxed::Box::new(imp(imports))),
                };
                ::std::boxed::Box::new(state)
            }
        }
        let exports = m::metadata()
            .into_iter()
            .map(|f| (String::from(f.name), f.signature, f.address))
            .collect::<Vec<_>>();
        ::wasmtime_api::Module::from_raw_parts($store, &exports, ::std::rc::Rc::new(T))
    }};
}

#[macro_export]
macro_rules! wrap_wasmtime_method {
    ($export:literal in $instance:expr; module( $module_name:path )) => {{
        use $module_name as m;
        let mut instance = $instance.clone();
        let export = instance.lookup($export).unwrap();
        m::Wrapper::new(instance, export)
    }};
}

#[macro_export]
macro_rules! wrap_wasmtime_instance {
    ($instance:expr; module( $module_name:path )) => {{
        use $module_name as m;
        m::Wrapper::new($instance)
    }};
}

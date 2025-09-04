mod control_effect;
pub(crate) mod fatpointer;
pub(crate) mod instructions;

pub(crate) mod builtins {
    macro_rules! define_builtin_accessors {
        ( $( $name:ident , )* ) => {
            $(
                #[inline]
                pub fn $name(
                    func_env: &mut crate::func_environ::FuncEnvironment<'_>,
                    func: &mut crate::ir::Function,
                ) -> wasmtime_environ::WasmResult<crate::ir::FuncRef> {
                    #[cfg(feature = "stack-switching")]
                    {
                        return Ok(func_env.builtin_functions.$name(func));
                    }

                    #[cfg(not(feature = "stack-switching"))]
                    {
                        let _ = (func, func_env);
                        return Err(wasmtime_environ::wasm_unsupported!(
                            "support for Wasm Stack Switching disabled at compile time because the `stack-switching` cargo \
                             feature was not enabled"
                        ));
                    }
                }
            )*
        };
    }

    define_builtin_accessors! {
        cont_new,
        table_grow_cont_obj,
        table_fill_cont_obj,
    }
}

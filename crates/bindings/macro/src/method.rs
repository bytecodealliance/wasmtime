use crate::attr::TransformAttributes;
use crate::signature::{read_signature, MethodSignature, ParameterType, PtrOrRef, Return};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{self, Ident, ItemFn};

pub(crate) fn need_context(sig: &MethodSignature) -> bool {
    for p in &sig.params {
        if let ParameterType::Ptr(_, _, _) = p.ty {
            return true;
        }
        if let ParameterType::Context(_) = p.ty {
            return true;
        }
    }
    if let Some(Return::Ptr(_, _, _)) = sig.result {
        return true;
    }
    false
}

pub(crate) struct TransformSignatureResult {
    pub abi_params: TokenStream,
    pub abi_return: TokenStream,
    pub params_conversion: TokenStream,
    pub ret_conversion: TokenStream,
    pub call_args: TokenStream,
    pub sig_build: TokenStream,
    pub cb_abi_params: TokenStream,
    pub cb_abi_return: TokenStream,
    pub cb_params_conversion: TokenStream,
    pub cb_ret_conversion: TokenStream,
    pub cb_call_args: TokenStream,
}
pub(crate) fn transform_sig(
    sig: &MethodSignature,
    context_name: TokenStream,
    wasmtime_bindings: TokenStream,
) -> TransformSignatureResult {
    let mut abi_params = TokenStream::new();
    let mut params_conversion = TokenStream::new();
    let mut call_args = TokenStream::new();
    let mut sig_build = TokenStream::new();

    let mut cb_abi_params = TokenStream::new();
    let mut cb_params_conversion = TokenStream::new();
    let mut cb_call_args = TokenStream::new();

    abi_params.extend(quote! {
        vmctx: *mut VMContext,
    });
    cb_call_args.extend(quote! {
        vmctx,
    });
    sig_build.extend(quote! {
        let call_conv = #wasmtime_bindings :: get_host_call_conv();
        let mut sig = ir::Signature::new(call_conv);
        sig.params.push(ir::AbiParam::special(
            ir::types::I64,
            ir::ArgumentPurpose::VMContext,
        ));
    });
    for (i, p) in sig.params.iter().enumerate() {
        let id = p.id;
        let internal_id = Ident::new(&format!("_a{}", i), Span::call_site());
        match p.ty {
            ParameterType::VMContextMutPtr => {
                call_args.extend(quote! { vmctx, });
            }
            ParameterType::Context(_ref) => {
                // TODO ref
                call_args.extend(quote! { _ctx, });
            }
            ParameterType::Ptr(_ty, _rp, _mut) => {
                abi_params.extend(quote! {
                    #internal_id: <#context_name as #wasmtime_bindings :: WasmMem>::Abi,
                });
                params_conversion.extend(quote! {
                    let #id = _ctx.as_ptr(#internal_id);
                });
                if let PtrOrRef::Ptr = _rp {
                    call_args.extend(quote! { #id , });
                } else if _mut.is_some() {
                    call_args.extend(quote! { unsafe { &mut *#id } , });
                } else {
                    call_args.extend(quote! { unsafe { &*#id } , });
                }
                sig_build.extend(quote! {
                    sig.params.push(ir::AbiParam::new(
                        #wasmtime_bindings :: get_ir_type::<<#context_name as #wasmtime_bindings :: WasmMem>::Abi>()
                    ));
                });
                let param_ty = sig.original_params[i];
                cb_abi_params.extend(quote! {
                    #id: #param_ty,
                });
                cb_params_conversion.extend(quote! {
                    let #internal_id = _ctx.as_off(#id);
                });
                cb_call_args.extend(quote! { #internal_id , });
            }
            ParameterType::Simple(ty) => {
                abi_params.extend(quote! {
                    #internal_id: <#ty as #wasmtime_bindings :: AbiPrimitive>::Abi,
                });
                params_conversion.extend(quote! {
                    let #id = <#ty as #wasmtime_bindings :: AbiPrimitive>::create_from_abi(#internal_id);
                });
                call_args.extend(quote! { #id , });
                sig_build.extend(quote! {
                    sig.params.push(ir::AbiParam::new(
                        #wasmtime_bindings :: get_ir_type::<<#ty as #wasmtime_bindings :: AbiPrimitive>::Abi>()
                    ));
                });
                let param_ty = sig.original_params[i];
                cb_abi_params.extend(quote! {
                    #id: #param_ty,
                });
                cb_params_conversion.extend(quote! {
                    let #internal_id = #id.convert_to_abi();
                });
                cb_call_args.extend(quote! { #internal_id , });
            }
            ParameterType::SelfRef(ref r) => {
                params_conversion.extend(if r.is_some() {
                    quote! { let mut _self = get_self_mut(vmctx); }
                } else {
                    quote! { let _self = get_self(vmctx); }
                });
            }
        }
    }
    let (abi_return, ret_conversion, sig_return, cb_ret_conversion) = match sig.result {
        Some(Return::Ptr(_ty, _rp, _mut)) => (
            quote! {
                -> <#context_name as #wasmtime_bindings :: WasmMem>::Abi
            },
            quote! {
                _ctx.as_off(_res)
            },
            quote! {
                sig.returns.push(ir::AbiParam::new(
                    #wasmtime_bindings :: get_ir_type::<<#context_name as #wasmtime_bindings :: WasmMem>::Abi>()
                ));
            },
            quote! {
                _ctx.as_ptr(_res)
            },
        ),
        Some(Return::Simple(ty)) => (
            quote! {
                -> <#ty as #wasmtime_bindings :: AbiPrimitive>::Abi
            },
            quote! {
                _res.convert_to_abi()
            },
            quote! {
                sig.returns.push(ir::AbiParam::new(
                    #wasmtime_bindings :: get_ir_type::<<#ty as #wasmtime_bindings :: AbiPrimitive>::Abi>()
                ));
            },
            quote! {
                <#ty as #wasmtime_bindings :: AbiPrimitive>::create_from_abi(_res)
            },
        ),
        None => (
            TokenStream::new(),
            TokenStream::new(),
            TokenStream::new(),
            TokenStream::new(),
        ),
    };
    sig_build.extend(sig_return);
    let cb_abi_return = if let Some(ref r) = sig.original_result {
        quote! { -> #r }
    } else {
        TokenStream::new()
    };

    TransformSignatureResult {
        abi_params,
        abi_return,
        params_conversion,
        ret_conversion,
        call_args,
        sig_build,
        cb_abi_params,
        cb_abi_return,
        cb_params_conversion,
        cb_ret_conversion,
        cb_call_args,
    }
}

pub(crate) fn wrap_method(f: ItemFn, attr: TransformAttributes) -> TokenStream {
    let sig = &f.sig;
    let name = &sig.ident;
    let vis = &f.vis;
    assert!(sig.constness.is_none());
    assert!(sig.asyncness.is_none());
    assert!(sig.unsafety.is_none());
    assert!(sig.abi.is_none());
    //assert!(sig.generics)
    let inputs = &sig.inputs;
    assert!(sig.variadic.is_none());
    let output = &sig.output;

    let rsig = read_signature(sig, &attr.context);

    let body = &f.block;

    let wasmtime_bindings = quote! { :: wasmtime_bindings };
    let (build_context, build_cb_context, context_name) = if need_context(&rsig) {
        if let Some(context_name) = attr.context {
            (
                quote! {
                    let _ctx = #context_name :: from_vmctx(vmctx);
                },
                quote! {
                    let _ctx = #context_name :: from_vmctx(self.vmctx);
                },
                quote! { #context_name },
            )
        } else {
            (
                quote! {
                    let _ctx = #wasmtime_bindings :: VMContextWrapper(vmctx);
                },
                quote! {
                    let _ctx = #wasmtime_bindings :: VMContextWrapper(self.vmctx);
                },
                quote! { #wasmtime_bindings :: VMContextWrapper },
            )
        }
    } else {
        (quote! {}, quote! {}, quote! { panic!("context n/a") })
    };

    let TransformSignatureResult {
        abi_params,
        abi_return,
        params_conversion,
        ret_conversion,
        call_args,
        sig_build,
        cb_abi_params,
        cb_abi_return,
        cb_params_conversion,
        cb_ret_conversion,
        cb_call_args,
    } = transform_sig(&rsig, context_name, wasmtime_bindings.clone());

    let def_module = if let Some(mod_name) = attr.module {
        quote! {
            #vis mod #mod_name {
                use super::*;
                use #wasmtime_bindings :: codegen :: {ir, isa};
                use #wasmtime_bindings :: { VMContext, InstanceHandle, InstanceHandleExport };
                use #wasmtime_bindings :: get_host_call_conv;
                pub fn signature() -> ir::Signature {
                    #sig_build
                    sig
                }

                pub struct Wrapper {
                    vmctx: *mut VMContext,
                    export: InstanceHandleExport,
                }
                impl Wrapper {
                    pub fn new(mut instance: InstanceHandle, export: InstanceHandleExport) -> Self {
                        Wrapper {
                            vmctx: instance.vmctx_mut_ptr(),
                            export,
                        }
                    }
                    pub fn call(&self, #cb_abi_params) #cb_abi_return {
                        type F = extern fn(#abi_params) #abi_return;
                        let (_f, vmctx) = #wasmtime_bindings :: get_body(&self . export);
                        let _f: F = unsafe { std::mem::transmute(_f) };
                        #build_cb_context
                        #cb_params_conversion
                        let _res = _f(#cb_call_args);
                        #cb_ret_conversion
                    }
                }
                pub fn metadata() -> #wasmtime_bindings :: FnMetadata {
                    #wasmtime_bindings :: FnMetadata {
                        name: stringify!(#name),
                        signature: signature(),
                        address: super :: #name as extern fn(#abi_params) #abi_return as *const _,
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        pub extern fn #name (#abi_params) #abi_return {
            let _closure = | #inputs | #output #body;
            #build_context
            #params_conversion
            let _res = _closure( #call_args );
            #ret_conversion
        }
        #def_module
    }
}

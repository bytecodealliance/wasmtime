//! Wasmtime's WASI HTTP Implementation
//!
//! This crate's implementation is primarily built on top of [`hyper`].
//!
//! # WASI HTTP Interfaces
//!
//! This crate contains implementations of the following interfaces:
//!
//! * `wasi:http/incoming-handler`
//! * `wasi:http/outgoing-handler`
//! * `wasi:http/types`
//!
//! The crate also contains an implementation of the [`wasi:http/proxy`] world.
//!
//! [`wasi:http/proxy`]: crate::proxy

//! All traits are implemented in terms of a [`WasiHttpView`] trait which provides
//! basic access to [`WasiHttpCtx`], configuration for WASI HTTP, and a [`wasmtime_wasi::ResourceTable`],
//! the state for all host-defined component model resources.

//! # Examples
//!
//! Usage of this crate is done through a few steps to get everything hooked up:
//!
//! 1. First implement [`WasiHttpView`] for your type which is the `T` in
//!    [`wasmtime::Store<T>`].
//! 2. Add WASI HTTP interfaces to a [`wasmtime::component::Linker<T>`]. This is either
//!    done through functions like [`proxy::add_to_linker`] (which bundles all interfaces
//!    in the `wasi:http/proxy` world together) or through individual interfaces like the
//!    [`bindings::http::outgoing_handler::add_to_linker_get_host`] function.
//! 3. Use the previous [`wasmtime::component::Linker<T>::instantiate`] to instantiate
//!    a [`wasmtime::component::Component`] within a [`wasmtime::Store<T>`]. If you're
//!    targeting the `wasi:http/proxy` world, you can instantiate the component with
//!    [`proxy::Proxy::instantiate_async`] or [`proxy::sync::Proxy::instantiate`] functions.

#![deny(missing_docs)]

mod error;
mod http_impl;
mod types_impl;

pub mod body;
pub mod io;
pub mod proxy;
pub mod types;

/// Raw bindings to the `wasi:http` package.
pub mod bindings {
    #![allow(missing_docs)]
    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
            import wasi:http/incoming-handler@0.2.0;
            import wasi:http/outgoing-handler@0.2.0;
            import wasi:http/types@0.2.0;
        ",
        tracing: true,
        async: false,
        trappable_imports: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::bindings::io,

            // Configure all WIT http resources to be defined types in this
            // crate to use the `ResourceTable` helper methods.
            "wasi:http/types/outgoing-body": super::body::HostOutgoingBody,
            "wasi:http/types/future-incoming-response": super::types::HostFutureIncomingResponse,
            "wasi:http/types/outgoing-response": super::types::HostOutgoingResponse,
            "wasi:http/types/future-trailers": super::body::HostFutureTrailers,
            "wasi:http/types/incoming-body": super::body::HostIncomingBody,
            "wasi:http/types/incoming-response": super::types::HostIncomingResponse,
            "wasi:http/types/response-outparam": super::types::HostResponseOutparam,
            "wasi:http/types/outgoing-request": super::types::HostOutgoingRequest,
            "wasi:http/types/incoming-request": super::types::HostIncomingRequest,
            "wasi:http/types/fields": super::types::HostFields,
            "wasi:http/types/request-options": super::types::HostRequestOptions,
        },
        trappable_error_type: {
            "wasi:http/types/error-code" => crate::HttpError,
        },
    });

    pub use wasi::http;
}

pub use crate::error::{
    http_request_error, hyper_request_error, hyper_response_error, HttpError, HttpResult,
};
#[doc(inline)]
pub use crate::types::{WasiHttpCtx, WasiHttpImpl, WasiHttpView};

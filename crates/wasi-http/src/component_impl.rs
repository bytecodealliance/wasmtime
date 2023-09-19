use crate::bindings::http::{
    outgoing_handler,
    types::{Error, Host, Method, RequestOptions, Scheme},
};
use crate::WasiHttpView;
use anyhow::anyhow;
use std::str;
use std::vec::Vec;
use wasmtime::{AsContext, AsContextMut, Caller, Extern, Memory};
use wasmtime_wasi::preview2::bindings::{io, poll};

const MEMORY: &str = "memory";

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error("Memory not found")]
    MemoryNotFound,
    #[error("Memory access error")]
    MemoryAccessError(#[from] wasmtime::MemoryAccessError),
    #[error("Buffer too small")]
    BufferTooSmall,
    #[error("UTF-8 error")]
    Utf8Error(#[from] std::str::Utf8Error),
}

fn memory_get<T>(caller: &mut Caller<'_, T>) -> Result<Memory, HttpError> {
    if let Some(Extern::Memory(mem)) = caller.get_export(MEMORY) {
        Ok(mem)
    } else {
        Err(HttpError::MemoryNotFound)
    }
}

/// Get a slice of length `len` from `memory`, starting at `offset`.
/// This will return an `HttpError::BufferTooSmall` if the size of the
/// requested slice is larger than the memory size.
fn slice_from_memory(
    memory: &Memory,
    mut ctx: impl AsContextMut,
    offset: u32,
    len: u32,
) -> Result<Vec<u8>, HttpError> {
    let required_memory_size = offset.checked_add(len).ok_or(HttpError::BufferTooSmall)? as usize;

    if required_memory_size > memory.data_size(&mut ctx) {
        return Err(HttpError::BufferTooSmall);
    }

    let mut buf = vec![0u8; len as usize];
    memory.read(&mut ctx, offset as usize, buf.as_mut_slice())?;
    Ok(buf)
}

fn u32_from_memory(memory: &Memory, ctx: impl AsContextMut, ptr: u32) -> Result<u32, HttpError> {
    let slice = slice_from_memory(memory, ctx, ptr, 4)?;
    let mut dst = [0u8; 4];
    dst.clone_from_slice(&slice[0..4]);
    Ok(u32::from_le_bytes(dst))
}

/// Read a string of byte length `len` from `memory`, starting at `offset`.
fn string_from_memory(
    memory: &Memory,
    ctx: impl AsContextMut,
    offset: u32,
    len: u32,
) -> Result<String, HttpError> {
    let slice = slice_from_memory(memory, ctx, offset, len)?;
    Ok(std::str::from_utf8(&slice)?.to_string())
}

fn read_option_string(
    memory: &Memory,
    ctx: impl AsContextMut,
    is_some: i32,
    ptr: u32,
    len: u32,
) -> Result<Option<String>, HttpError> {
    if is_some == 1 {
        Ok(Some(string_from_memory(&memory, ctx, ptr, len)?))
    } else {
        Ok(None)
    }
}

async fn allocate_guest_pointer<T: Send>(
    caller: &mut Caller<'_, T>,
    size: u32,
) -> anyhow::Result<u32> {
    let realloc = caller
        .get_export("cabi_realloc")
        .ok_or_else(|| anyhow!("missing required export cabi_realloc"))?;
    let func = realloc
        .into_func()
        .ok_or_else(|| anyhow!("cabi_realloc must be a func"))?;
    let typed = func.typed::<(u32, u32, u32, u32), u32>(caller.as_context())?;
    Ok(typed
        .call_async(caller.as_context_mut(), (0, 0, 4, size))
        .await?)
}

fn u32_array_to_u8(arr: &[u32]) -> Vec<u8> {
    let mut result = std::vec::Vec::new();
    for val in arr.iter() {
        let bytes = val.to_le_bytes();
        for b in bytes.iter() {
            result.push(*b);
        }
    }
    result
}

pub fn add_component_to_linker<T: WasiHttpView>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut T + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    linker.func_wrap8_async(
        "wasi:http/outgoing-handler",
        "handle",
        move |mut caller: Caller<'_, T>,
              request: u32,
              has_options: i32,
              has_timeout: i32,
              timeout_ms: u32,
              has_first_byte_timeout: i32,
              first_byte_timeout_ms: u32,
              has_between_bytes_timeout: i32,
              between_bytes_timeout_ms: u32| {
            Box::new(async move {
                let options = if has_options == 1 {
                    Some(RequestOptions {
                        connect_timeout_ms: if has_timeout == 1 {
                            Some(timeout_ms)
                        } else {
                            None
                        },
                        first_byte_timeout_ms: if has_first_byte_timeout == 1 {
                            Some(first_byte_timeout_ms)
                        } else {
                            None
                        },
                        between_bytes_timeout_ms: if has_between_bytes_timeout == 1 {
                            Some(between_bytes_timeout_ms)
                        } else {
                            None
                        },
                    })
                } else {
                    None
                };

                let ctx = get_cx(caller.data_mut());
                tracing::trace!("[module='wasi:http/outgoing-handler' function='handle'] call request={:?} options={:?}", request, options);
                let result = outgoing_handler::Host::handle(ctx, request, options).await;
                tracing::trace!(
                    "[module='wasi:http/outgoing-handler' function='handle'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap14_async(
        "wasi:http/types",
        "new-outgoing-request",
        move |mut caller: Caller<'_, T>,
              method: i32,
              method_ptr: i32,
              method_len: i32,
              path_is_some: i32,
              path_ptr: u32,
              path_len: u32,
              scheme_is_some: i32,
              scheme: i32,
              scheme_ptr: i32,
              scheme_len: i32,
              authority_is_some: i32,
              authority_ptr: u32,
              authority_len: u32,
              headers: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;
                let path = read_option_string(
                    &memory,
                    caller.as_context_mut(),
                    path_is_some,
                    path_ptr,
                    path_len,
                )?;
                let authority = read_option_string(
                    &memory,
                    caller.as_context_mut(),
                    authority_is_some,
                    authority_ptr,
                    authority_len,
                )?;

                let mut s = Some(Scheme::Https);
                if scheme_is_some == 1 {
                    s = Some(match scheme {
                        0 => Scheme::Http,
                        1 => Scheme::Https,
                        _ => {
                            let value = string_from_memory(
                                &memory,
                                caller.as_context_mut(),
                                scheme_ptr.try_into()?,
                                scheme_len.try_into()?,
                            )?;
                            Scheme::Other(value)
                        }
                    });
                }
                let m = match method {
                    0 => Method::Get,
                    1 => Method::Head,
                    2 => Method::Post,
                    3 => Method::Put,
                    4 => Method::Delete,
                    5 => Method::Connect,
                    6 => Method::Options,
                    7 => Method::Trace,
                    8 => Method::Patch,
                    _ => {
                        let value = string_from_memory(
                            &memory,
                            caller.as_context_mut(),
                            method_ptr.try_into()?,
                            method_len.try_into()?,
                        )?;
                        Method::Other(value)
                    }
                };

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='new-outgoing-request'] call method={:?} path={:?} scheme={:?} authority={:?} headers={:?}",
                    m,
                    path,
                    s,
                    authority,
                    headers
                );
                let result =
                    Host::new_outgoing_request(ctx, m, path, s, authority, headers).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='new-outgoing-request'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "incoming-response-status",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-status'] call id={:?}",
                    id
                );
                let result = Ok(u32::from(Host::incoming_response_status(ctx, id).await?));
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-status'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "drop-future-incoming-response",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-future-incoming-response'] call id={:?}",
                    id
                );
                let result = Host::drop_future_incoming_response(ctx, id).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-future-incoming-response'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:http/types",
        "future-incoming-response-get",
        move |mut caller: Caller<'_, T>, future: u32, ptr: i32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='future-incoming-response-get'] call future={:?}",
                    future
                );
                let result = Host::future_incoming_response_get(ctx, future).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='future-incoming-response-get'] return result={:?}",
                    result
                );
                let response = result?;

                let memory = memory_get(&mut caller)?;

                // First == is_some
                // Second == is_err
                // Third == {ok: is_err = false, tag: is_err = true}
                // Fourth == string ptr
                // Fifth == string len
                let result: [u32; 5] = match response {
                    Some(inner) => match inner {
                        Ok(value) => [1, 0, value, 0, 0],
                        Err(error) => {
                            let (tag, err_string) = match error {
                                Error::InvalidUrl(e) => (0u32, e),
                                Error::TimeoutError(e) => (1u32, e),
                                Error::ProtocolError(e) => (2u32, e),
                                Error::UnexpectedError(e) => (3u32, e),
                            };
                            let bytes = err_string.as_bytes();
                            let len = bytes.len().try_into().unwrap();
                            let ptr = allocate_guest_pointer(&mut caller, len).await?;
                            memory.write(caller.as_context_mut(), ptr as _, bytes)?;
                            [1, 1, tag, ptr, len]
                        }
                    },
                    None => [0, 0, 0, 0, 0],
                };
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "listen-to-future-incoming-response",
        move |mut caller: Caller<'_, T>, future: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='listen-to-future-incoming-response'] call future={:?}",
                    future
                );
                let result = Host::listen_to_future_incoming_response(ctx, future).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='listen-to-future-incoming-response'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:http/types",
        "incoming-response-consume",
        move |mut caller: Caller<'_, T>, response: u32, ptr: i32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                    tracing::trace!(
                        "[module='wasi:http/types' function='incoming-response-consume'] call response={:?}",
                        response
                    );
                    let result = Host::incoming_response_consume(ctx, response).await;
                    tracing::trace!(
                        "[module='wasi:http/types' function='incoming-response-consume'] return result={:?}",
                        result
                    );
                    let stream = result?.unwrap_or(0);

                let memory = memory_get(&mut caller).unwrap();

                // First == is_some
                // Second == stream_id
                let result: [u32; 2] = [0, stream];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:poll/poll",
        "drop-pollable",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:poll/poll' function='drop-pollable'] call id={:?}",
                    id
                );
                let result = poll::poll::Host::drop_pollable(ctx, id);
                tracing::trace!(
                    "[module='wasi:poll/poll' function='drop-pollable'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap3_async(
        "wasi:poll/poll",
        "poll-oneoff",
        move |mut caller: Caller<'_, T>, base_ptr: u32, len: u32, out_ptr: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;

                let mut vec = Vec::new();
                let mut i = 0;
                while i < len {
                    let ptr = base_ptr + i * 4;
                    let pollable_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr)?;
                    vec.push(pollable_ptr);
                    i = i + 1;
                }

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] call in={:?}",
                    vec
                );
                let result = poll::poll::Host::poll_oneoff(ctx, vec).await;
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] return result={:?}",
                    result
                );
                let result = result?;

                let result_len = result.len();
                let result_ptr =
                    allocate_guest_pointer(&mut caller, (4 * result_len).try_into()?).await?;
                let mut ptr = result_ptr;
                for item in result.iter() {
                    let completion: u32 = match item {
                        true => 1,
                        false => 0,
                    };
                    memory.write(caller.as_context_mut(), ptr as _, &completion.to_be_bytes())?;

                    ptr = ptr + 4;
                }

                let result: [u32; 2] = [result_ptr, result_len.try_into()?];
                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), out_ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:io/streams",
        "drop-input-stream",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-input-stream'] call id={:?}",
                    id
                );
                let result = io::streams::Host::drop_input_stream(ctx, id);
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-input-stream'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:io/streams",
        "drop-output-stream",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-output-stream'] call id={:?}",
                    id
                );
                let result = io::streams::Host::drop_output_stream(ctx, id);
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-output-stream'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap3_async(
        "wasi:io/streams",
        "read",
        move |mut caller: Caller<'_, T>, stream: u32, len: u64, ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='read'] call this={:?} len={:?}",
                    stream,
                    len
                );
                let result = io::streams::Host::read(ctx, stream, len).await;
                tracing::trace!(
                    "[module='wasi:io/streams' function='read'] return result={:?}",
                    result
                );
                let (bytes, status) = result?.map_err(|_| anyhow!("read failed"))?;

                let done = match status {
                    io::streams::StreamStatus::Open => 0,
                    io::streams::StreamStatus::Ended => 1,
                };
                let body_len: u32 = bytes.len().try_into()?;
                let out_ptr = allocate_guest_pointer(&mut caller, body_len).await?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                // Third == bytes length
                // Fourth == enum status
                let result: [u32; 4] = [0, out_ptr, body_len, done];
                let raw = u32_array_to_u8(&result);

                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), out_ptr as _, &bytes)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap3_async(
        "wasi:io/streams",
        "blocking-read",
        move |mut caller: Caller<'_, T>, stream: u32, len: u64, ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-read'] call this={:?} len={:?}",
                    stream,
                    len
                );
                let result = io::streams::Host::blocking_read(ctx, stream, len).await;
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-read'] return result={:?}",
                    result
                );
                let (bytes, status) = result?.map_err(|_| anyhow!("read failed"))?;

                let done = match status {
                    io::streams::StreamStatus::Open => 0,
                    io::streams::StreamStatus::Ended => 1,
                };
                let body_len: u32 = bytes.len().try_into()?;
                let out_ptr = allocate_guest_pointer(&mut caller, body_len).await?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                // Third == bytes length
                // Fourth == enum status
                let result: [u32; 4] = [0, out_ptr, body_len, done];
                let raw = u32_array_to_u8(&result);

                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), out_ptr as _, &bytes)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:io/streams",
        "subscribe-to-input-stream",
        move |mut caller: Caller<'_, T>, stream: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-input-stream'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::subscribe_to_input_stream(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-input-stream'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:io/streams",
        "subscribe-to-output-stream",
        move |mut caller: Caller<'_, T>, stream: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-output-stream'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::subscribe_to_output_stream(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-output-stream'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:io/streams",
        "check-write",
        move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='check-write'] call stream={:?}",
                    stream,
                );
                let result = io::streams::Host::check_write(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='check-write'] return result={:?}",
                    result
                );

                let result: [u32; 3] = match result {
                    // 0 == outer result tag (success)
                    // 1 == result value (u64 upper 32 bits)
                    // 2 == result value (u64 lower 32 bits)
                    Ok(len) => [0, (len >> 32) as u32, len as u32],

                    // 0 == outer result tag (failure)
                    // 1 == result value (unused)
                    // 2 == result value (error type)
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:io/streams",
        "flush",
        move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='flush'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::flush(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='flush'] return result={:?}",
                    result
                );

                let result: [u32; 2] = match result {
                    // 0 == outer result tag
                    // 1 == unused
                    Ok(_) => [0, 0],

                    // 0 == outer result tag
                    // 1 == inner result tag
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:io/streams",
        "blocking-flush",
        move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-flush'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::blocking_flush(ctx, stream).await;
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-flush'] return result={:?}",
                    result
                );

                let result: [u32; 2] = match result {
                    // 0 == outer result tag
                    // 1 == unused
                    Ok(_) => [0, 0],

                    // 0 == outer result tag
                    // 1 == inner result tag
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            })
        },
    )?;
    linker.func_wrap4_async(
        "wasi:io/streams",
        "write",
        move |mut caller: Caller<'_, T>, stream: u32, body_ptr: u32, body_len: u32, ptr: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;
                let body = slice_from_memory(&memory, caller.as_context_mut(), body_ptr, body_len)?;

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='write'] call stream={:?} body={:?}",
                    stream,
                    body
                );
                let result = io::streams::Host::write(ctx, stream, body.into()).await;
                tracing::trace!(
                    "[module='wasi:io/streams' function='write'] return result={:?}",
                    result
                );
                result?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                let result: [u32; 2] = [0, 0];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            })
        },
    )?;
    linker.func_wrap4_async(
        "wasi:io/streams",
        "blocking-write-and-flush",
        move |mut caller: Caller<'_, T>, stream: u32, body_ptr: u32, body_len: u32, ptr: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;
                let body = slice_from_memory(&memory, caller.as_context_mut(), body_ptr, body_len)?;

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-write-and-flush'] call stream={:?} body={:?}",
                    stream,
                    body
                );
                let result = io::streams::Host::blocking_write_and_flush(ctx, stream, body.into()).await;
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-write-and-flush'] return result={:?}",
                    result
                );
                result?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                let result: [u32; 2] = [0, 0];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "drop-fields",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-fields'] call id={:?}",
                    id
                );
                let result = Host::drop_fields(ctx, id).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-fields'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:http/types",
        "outgoing-request-write",
        move |mut caller: Caller<'_, T>, request: u32, ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='outgoing-request-write'] call request={:?}",
                    request
                );
                let result = Host::outgoing_request_write(ctx, request).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='outgoing-request-write'] return result={:?}",
                    result
                );
                let stream = result?
                    .map_err(|_| anyhow!("no outgoing stream present"))?;

                let memory = memory_get(&mut caller)?;
                // First == is_some
                // Second == stream_id
                let result: [u32; 2] = [0, stream];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "drop-outgoing-request",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-outgoing-request'] call id={:?}",
                    id
                );
                let result = Host::drop_outgoing_request(ctx, id).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-outgoing-request'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "drop-incoming-response",
        move |mut caller: Caller<'_, T>, id: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-incoming-response'] call id={:?}",
                    id
                );
                let result = Host::drop_incoming_response(ctx, id).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-incoming-response'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:http/types",
        "new-fields",
        move |mut caller: Caller<'_, T>, base_ptr: u32, len: u32| {
            Box::new(async move {
                let memory = memory_get(&mut caller)?;

                let mut vec = Vec::new();
                let mut i = 0;
                // TODO: read this more efficiently as a single block.
                while i < len {
                    let ptr = base_ptr + i * 16;
                    let name_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr)?;
                    let name_len = u32_from_memory(&memory, caller.as_context_mut(), ptr + 4)?;
                    let value_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr + 8)?;
                    let value_len = u32_from_memory(&memory, caller.as_context_mut(), ptr + 12)?;

                    let name =
                        string_from_memory(&memory, caller.as_context_mut(), name_ptr, name_len)?;
                    let value =
                        string_from_memory(&memory, caller.as_context_mut(), value_ptr, value_len)?;

                    vec.push((name, value));
                    i = i + 1;
                }

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='new-fields'] call entries={:?}",
                    vec
                );
                let result = Host::new_fields(ctx, vec).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='new-fields'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    linker.func_wrap2_async(
        "wasi:http/types",
        "fields-entries",
        move |mut caller: Caller<'_, T>, fields: u32, out_ptr: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='fields-entries'] call fields={:?}",
                    fields
                );
                let result = Host::fields_entries(ctx, fields).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='fields-entries'] return result={:?}",
                    result
                );
                let entries = result?;

                let header_len = entries.len();
                let tuple_ptr =
                    allocate_guest_pointer(&mut caller, (16 * header_len).try_into()?).await?;
                let mut ptr = tuple_ptr;
                for item in entries.iter() {
                    let name = &item.0;
                    let value = &item.1;
                    let name_len: u32 = name.len().try_into()?;
                    let value_len: u32 = value.len().try_into()?;

                    let name_ptr = allocate_guest_pointer(&mut caller, name_len).await?;
                    let value_ptr = allocate_guest_pointer(&mut caller, value_len).await?;

                    let memory = memory_get(&mut caller)?;
                    memory.write(caller.as_context_mut(), name_ptr as _, &name.as_bytes())?;
                    memory.write(caller.as_context_mut(), value_ptr as _, value)?;

                    let pair: [u32; 4] = [name_ptr, name_len, value_ptr, value_len];
                    let raw_pair = u32_array_to_u8(&pair);
                    memory.write(caller.as_context_mut(), ptr as _, &raw_pair)?;

                    ptr = ptr + 16;
                }

                let memory = memory_get(&mut caller)?;
                let result: [u32; 2] = [tuple_ptr, header_len.try_into()?];
                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), out_ptr as _, &raw)?;
                Ok(())
            })
        },
    )?;
    linker.func_wrap1_async(
        "wasi:http/types",
        "incoming-response-headers",
        move |mut caller: Caller<'_, T>, handle: u32| {
            Box::new(async move {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-headers'] call handle={:?}",
                    handle
                );
                let result = Host::incoming_response_headers(ctx, handle).await;
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-headers'] return result={:?}",
                    result
                );
                result
            })
        },
    )?;
    Ok(())
}

pub mod sync {
    use super::{
        memory_get, read_option_string, string_from_memory, u32_array_to_u8, u32_from_memory,
    };
    use crate::bindings::sync::http::{
        outgoing_handler,
        types::{Error, Host, Method, RequestOptions, Scheme},
    };
    use crate::WasiHttpView;
    use anyhow::anyhow;
    use wasmtime::{AsContext, AsContextMut, Caller};
    use wasmtime_wasi::preview2::bindings::sync_io::{io, poll};

    fn allocate_guest_pointer<T: Send>(
        caller: &mut Caller<'_, T>,
        size: u32,
    ) -> anyhow::Result<u32> {
        let realloc = caller
            .get_export("cabi_realloc")
            .ok_or_else(|| anyhow!("missing required export cabi_realloc"))?;
        let func = realloc
            .into_func()
            .ok_or_else(|| anyhow!("cabi_realloc must be a func"))?;
        let typed = func.typed::<(u32, u32, u32, u32), u32>(caller.as_context())?;
        Ok(typed.call(caller.as_context_mut(), (0, 0, 4, size))?)
    }

    pub fn add_component_to_linker<T: WasiHttpView>(
        linker: &mut wasmtime::Linker<T>,
        get_cx: impl Fn(&mut T) -> &mut T + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        linker.func_wrap(
            "wasi:http/outgoing-handler",
            "handle",
            move |mut caller: Caller<'_, T>,
                  request: u32,
                  has_options: i32,
                  has_timeout: i32,
                  timeout_ms: u32,
                  has_first_byte_timeout: i32,
                  first_byte_timeout_ms: u32,
                  has_between_bytes_timeout: i32,
                  between_bytes_timeout_ms: u32|
                  -> anyhow::Result<u32> {
                let options = if has_options == 1 {
                    Some(RequestOptions {
                        connect_timeout_ms: if has_timeout == 1 {
                            Some(timeout_ms)
                        } else {
                            None
                        },
                        first_byte_timeout_ms: if has_first_byte_timeout == 1 {
                            Some(first_byte_timeout_ms)
                        } else {
                            None
                        },
                        between_bytes_timeout_ms: if has_between_bytes_timeout == 1 {
                            Some(between_bytes_timeout_ms)
                        } else {
                            None
                        },
                    })
                } else {
                    None
                };

                let ctx = get_cx(caller.data_mut());
                tracing::trace!("[module='wasi:http/outgoing-handler' function='handle'] call request={:?} options={:?}", request, options);
                let result = outgoing_handler::Host::handle(ctx, request, options);
                tracing::trace!(
                    "[module='wasi:http/outgoing-handler' function='handle'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "new-outgoing-request",
            move |mut caller: Caller<'_, T>,
                  method: i32,
                  method_ptr: i32,
                  method_len: i32,
                  path_is_some: i32,
                  path_ptr: u32,
                  path_len: u32,
                  scheme_is_some: i32,
                  scheme: i32,
                  scheme_ptr: i32,
                  scheme_len: i32,
                  authority_is_some: i32,
                  authority_ptr: u32,
                  authority_len: u32,
                  headers: u32|
                  -> anyhow::Result<u32> {
                let memory = memory_get(&mut caller)?;
                let path = read_option_string(
                    &memory,
                    caller.as_context_mut(),
                    path_is_some,
                    path_ptr,
                    path_len,
                )?;
                let authority = read_option_string(
                    &memory,
                    caller.as_context_mut(),
                    authority_is_some,
                    authority_ptr,
                    authority_len,
                )?;

                let mut s = Some(Scheme::Https);
                if scheme_is_some == 1 {
                    s = Some(match scheme {
                        0 => Scheme::Http,
                        1 => Scheme::Https,
                        _ => {
                            let value = string_from_memory(
                                &memory,
                                caller.as_context_mut(),
                                scheme_ptr.try_into()?,
                                scheme_len.try_into()?,
                            )?;
                            Scheme::Other(value)
                        }
                    });
                }
                let m = match method {
                    0 => Method::Get,
                    1 => Method::Head,
                    2 => Method::Post,
                    3 => Method::Put,
                    4 => Method::Delete,
                    5 => Method::Connect,
                    6 => Method::Options,
                    7 => Method::Trace,
                    8 => Method::Patch,
                    _ => {
                        let value = string_from_memory(
                            &memory,
                            caller.as_context_mut(),
                            method_ptr.try_into()?,
                            method_len.try_into()?,
                        )?;
                        Method::Other(value)
                    }
                };

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='new-outgoing-request'] call method={:?} path={:?} scheme={:?} authority={:?} headers={:?}",
                    m,
                    path,
                    s,
                    authority,
                    headers
                );
                let result =
                    Host::new_outgoing_request(ctx, m, path, s, authority, headers);
                tracing::trace!(
                    "[module='wasi:http/types' function='new-outgoing-request'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "incoming-response-status",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<u32> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-status'] call id={:?}",
                    id
                );
                let result = Ok(u32::from(Host::incoming_response_status(ctx, id)?));
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-status'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "drop-future-incoming-response",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-future-incoming-response'] call id={:?}",
                    id
                );
                let result = Host::drop_future_incoming_response(ctx, id);
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-future-incoming-response'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "future-incoming-response-get",
            move |mut caller: Caller<'_, T>, future: u32, ptr: i32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());

                tracing::trace!(
                    "[module='wasi:http/types' function='future-incoming-response-get'] call future={:?}",
                    future
                );
                let result = Host::future_incoming_response_get(ctx, future);
                tracing::trace!(
                    "[module='wasi:http/types' function='future-incoming-response-get'] return result={:?}",
                    result
                );
                let response = result?;

                let memory = memory_get(&mut caller)?;

                // First == is_some
                // Second == is_err
                // Third == {ok: is_err = false, tag: is_err = true}
                // Fourth == string ptr
                // Fifth == string len
                let result: [u32; 5] = match response {
                    Some(inner) => match inner {
                        Ok(value) => [1, 0, value, 0, 0],
                        Err(error) => {
                            let (tag, err_string) = match error {
                                Error::InvalidUrl(e) => (0u32, e),
                                Error::TimeoutError(e) => (1u32, e),
                                Error::ProtocolError(e) => (2u32, e),
                                Error::UnexpectedError(e) => (3u32, e),
                            };
                            let bytes = err_string.as_bytes();
                            let len = bytes.len().try_into().unwrap();
                            let ptr = allocate_guest_pointer(&mut caller, len)?;
                            memory.write(caller.as_context_mut(), ptr as _, bytes)?;
                            [1, 1, tag, ptr, len]
                        }
                    },
                    None => [0, 0, 0, 0, 0],
                };
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "listen-to-future-incoming-response",
            move |mut caller: Caller<'_, T>, future: u32| -> anyhow::Result<u32> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='listen-to-future-incoming-response'] call future={:?}",
                    future
                );
                let result = Host::listen_to_future_incoming_response(ctx, future);
                tracing::trace!(
                    "[module='wasi:http/types' function='listen-to-future-incoming-response'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "incoming-response-consume",
            move |mut caller: Caller<'_, T>, response: u32, ptr: i32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-consume'] call response={:?}",
                    response
                );
                let result = Host::incoming_response_consume(ctx, response);
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-consume'] return result={:?}",
                    result
                );
                let stream = result?.unwrap_or(0);

                let memory = memory_get(&mut caller).unwrap();

                // First == is_some
                // Second == stream_id
                let result: [u32; 2] = [0, stream];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:poll/poll",
            "drop-pollable",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:poll/poll' function='drop-pollable'] call id={:?}",
                    id
                );
                let result = poll::poll::Host::drop_pollable(ctx, id);
                tracing::trace!(
                    "[module='wasi:poll/poll' function='drop-pollable'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:poll/poll",
            "poll-oneoff",
            move |mut caller: Caller<'_, T>,
                  base_ptr: u32,
                  len: u32,
                  out_ptr: u32|
                  -> anyhow::Result<()> {
                let memory = memory_get(&mut caller)?;

                let mut vec = Vec::new();
                let mut i = 0;
                while i < len {
                    let ptr = base_ptr + i * 4;
                    let pollable_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr)?;
                    vec.push(pollable_ptr);
                    i = i + 1;
                }

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] call in={:?}",
                    vec
                );
                let result = poll::poll::Host::poll_oneoff(ctx, vec);
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] return result={:?}",
                    result
                );
                let result = result?;

                let result_len = result.len();
                let result_ptr = allocate_guest_pointer(&mut caller, (4 * result_len).try_into()?)?;
                let mut ptr = result_ptr;
                for item in result.iter() {
                    let completion: u32 = match item {
                        true => 1,
                        false => 0,
                    };
                    memory.write(caller.as_context_mut(), ptr as _, &completion.to_be_bytes())?;

                    ptr = ptr + 4;
                }

                let result: [u32; 2] = [result_ptr, result_len.try_into()?];
                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), out_ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "drop-input-stream",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-input-stream'] call id={:?}",
                    id
                );
                let result = io::streams::Host::drop_input_stream(ctx, id);
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-input-stream'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "drop-output-stream",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-output-stream'] call id={:?}",
                    id
                );
                let result = io::streams::Host::drop_output_stream(ctx, id);
                tracing::trace!(
                    "[module='wasi:io/streams' function='drop-output-stream'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "read",
            move |mut caller: Caller<'_, T>,
                  stream: u32,
                  len: u64,
                  ptr: u32|
                  -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='read'] call this={:?} len={:?}",
                    stream,
                    len
                );
                let result = io::streams::Host::read(ctx, stream, len);
                tracing::trace!(
                    "[module='wasi:io/streams' function='read'] return result={:?}",
                    result
                );
                let (bytes, status) = result?.map_err(|_| anyhow!("read failed"))?;

                let done = match status {
                    io::streams::StreamStatus::Open => 0,
                    io::streams::StreamStatus::Ended => 1,
                };
                let body_len: u32 = bytes.len().try_into()?;
                let out_ptr = allocate_guest_pointer(&mut caller, body_len)?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                // Third == bytes length
                // Fourth == enum status
                let result: [u32; 4] = [0, out_ptr, body_len, done];
                let raw = u32_array_to_u8(&result);

                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), out_ptr as _, &bytes)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "blocking-read",
            move |mut caller: Caller<'_, T>,
                  stream: u32,
                  len: u64,
                  ptr: u32|
                  -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());

                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-read'] call this={:?} len={:?}",
                    stream,
                    len
                );
                let result = io::streams::Host::blocking_read(ctx, stream, len);
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-read'] return result={:?}",
                    result
                );
                let (bytes, status) = result?.map_err(|_| anyhow!("read failed"))?;

                let done = match status {
                    io::streams::StreamStatus::Open => 0,
                    io::streams::StreamStatus::Ended => 1,
                };
                let body_len: u32 = bytes.len().try_into()?;
                let out_ptr = allocate_guest_pointer(&mut caller, body_len)?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                // Third == bytes length
                // Fourth == enum status
                let result: [u32; 4] = [0, out_ptr, body_len, done];
                let raw = u32_array_to_u8(&result);

                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), out_ptr as _, &bytes)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "subscribe-to-input-stream",
            move |mut caller: Caller<'_, T>, stream: u32| -> anyhow::Result<u32> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-input-stream'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::subscribe_to_input_stream(ctx, stream)?;
                // TODO: necessary until this PR has been merged:
                // https://github.com/bytecodealliance/wasmtime/pull/6877
                let oneoff_result = poll::poll::Host::poll_oneoff(ctx, vec![result])?;
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] return result={:?}",
                    oneoff_result
                );
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-input-stream'] return result=Ok({:?})",
                    result
                );
                Ok(result)
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "subscribe-to-output-stream",
            move |mut caller: Caller<'_, T>, stream: u32| -> anyhow::Result<u32> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-output-stream'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::subscribe_to_output_stream(ctx, stream)?;
                // TODO: necessary until this PR has been merged:
                // https://github.com/bytecodealliance/wasmtime/pull/6877
                let oneoff_result = poll::poll::Host::poll_oneoff(ctx, vec![result])?;
                tracing::trace!(
                    "[module='wasi:poll/poll' function='poll-oneoff'] return result={:?}",
                    oneoff_result
                );
                tracing::trace!(
                    "[module='wasi:io/streams' function='subscribe-to-output-stream'] return result=Ok({:?})",
                    result
                );
                Ok(result)
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "write",
            move |mut caller: Caller<'_, T>,
                  stream: u32,
                  body_ptr: u32,
                  body_len: u32,
                  ptr: u32|
                  -> anyhow::Result<()> {
                let memory = memory_get(&mut caller)?;
                let body =
                    string_from_memory(&memory, caller.as_context_mut(), body_ptr, body_len)?;

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='write'] call stream={:?} body={:?}",
                    stream,
                    body
                );
                let result = io::streams::Host::write(ctx, stream, body.into());
                tracing::trace!(
                    "[module='wasi:io/streams' function='write'] return result={:?}",
                    result
                );
                result?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                let result: [u32; 2] = [0, 0];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "blocking-write-and-flush",
            move |mut caller: Caller<'_, T>,
                  stream: u32,
                  body_ptr: u32,
                  body_len: u32,
                  ptr: u32|
                  -> anyhow::Result<()> {
                let memory = memory_get(&mut caller)?;
                let body =
                    string_from_memory(&memory, caller.as_context_mut(), body_ptr, body_len)?;

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-write-and-flush'] call stream={:?} body={:?}",
                    stream,
                    body
                );
                let result = io::streams::Host::blocking_write_and_flush(ctx, stream, body.into());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-write-and-flush'] return result={:?}",
                    result
                );
                result?;

                // First == is_err
                // Second == {ok: is_err = false, tag: is_err = true}
                let result: [u32; 2] = [0, 0];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "check-write",
            move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
                let memory = memory_get(&mut caller)?;
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='check-write'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::check_write(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='check-write'] return result={:?}",
                    result
                );

                let result: [u32; 3] = match result {
                    // 0 == outer result tag (success)
                    // 1 == result value (u64 upper 32 bits)
                    // 2 == result value (u64 lower 32 bits)
                    Ok(len) => [0, (len >> 32) as u32, len as u32],

                    // 0 == outer result tag (failure)
                    // 1 == result value (unused)
                    // 2 == result value (error type)
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "flush",
            move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='flush'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::flush(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='flush'] return result={:?}",
                    result
                );

                let result: [u32; 2] = match result {
                    // 0 == outer result tag
                    // 1 == unused
                    Ok(_) => [0, 0],

                    // 0 == outer result tag
                    // 1 == inner result tag
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:io/streams",
            "blocking-flush",
            move |mut caller: Caller<'_, T>, stream: u32, ptr: u32| {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-flush'] call stream={:?}",
                    stream
                );
                let result = io::streams::Host::blocking_flush(ctx, stream);
                tracing::trace!(
                    "[module='wasi:io/streams' function='blocking-flush'] return result={:?}",
                    result
                );

                let result: [u32; 2] = match result {
                    // 0 == outer result tag
                    // 1 == unused
                    Ok(_) => [0, 0],

                    // 0 == outer result tag
                    // 1 == inner result tag
                    Err(_) => todo!("how do we extract runtime error cases?"),
                };

                let raw = u32_array_to_u8(&result);
                let memory = memory_get(&mut caller)?;
                memory.write(caller.as_context_mut(), ptr as _, &raw)?;

                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "drop-fields",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-fields'] call id={:?}",
                    id
                );
                let result = Host::drop_fields(ctx, id);
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-fields'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "outgoing-request-write",
            move |mut caller: Caller<'_, T>, request: u32, ptr: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='outgoing-request-write'] call request={:?}",
                    request
                );
                let result = Host::outgoing_request_write(ctx, request);
                tracing::trace!(
                    "[module='wasi:http/types' function='outgoing-request-write'] return result={:?}",
                    result
                );
                let stream = result?
                    .map_err(|_| anyhow!("no outgoing stream present"))?;

                let memory = memory_get(&mut caller)?;
                // First == is_some
                // Second == stream_id
                let result: [u32; 2] = [0, stream];
                let raw = u32_array_to_u8(&result);

                memory.write(caller.as_context_mut(), ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "drop-outgoing-request",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-outgoing-request'] call id={:?}",
                    id
                );
                let result = Host::drop_outgoing_request(ctx, id);
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-outgoing-request'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "drop-incoming-response",
            move |mut caller: Caller<'_, T>, id: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-incoming-response'] call id={:?}",
                    id
                );
                let result = Host::drop_incoming_response(ctx, id);
                tracing::trace!(
                    "[module='wasi:http/types' function='drop-incoming-response'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "new-fields",
            move |mut caller: Caller<'_, T>, base_ptr: u32, len: u32| -> anyhow::Result<u32> {
                let memory = memory_get(&mut caller)?;

                let mut vec = Vec::new();
                let mut i = 0;
                // TODO: read this more efficiently as a single block.
                while i < len {
                    let ptr = base_ptr + i * 16;
                    let name_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr)?;
                    let name_len = u32_from_memory(&memory, caller.as_context_mut(), ptr + 4)?;
                    let value_ptr = u32_from_memory(&memory, caller.as_context_mut(), ptr + 8)?;
                    let value_len = u32_from_memory(&memory, caller.as_context_mut(), ptr + 12)?;

                    let name =
                        string_from_memory(&memory, caller.as_context_mut(), name_ptr, name_len)?;
                    let value =
                        string_from_memory(&memory, caller.as_context_mut(), value_ptr, value_len)?;

                    vec.push((name, value));
                    i = i + 1;
                }

                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='new-fields'] call entries={:?}",
                    vec
                );
                let result = Host::new_fields(ctx, vec);
                tracing::trace!(
                    "[module='wasi:http/types' function='new-fields'] return result={:?}",
                    result
                );
                result
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "fields-entries",
            move |mut caller: Caller<'_, T>, fields: u32, out_ptr: u32| -> anyhow::Result<()> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='fields-entries'] call fields={:?}",
                    fields
                );
                let result = Host::fields_entries(ctx, fields);
                tracing::trace!(
                    "[module='wasi:http/types' function='fields-entries'] return result={:?}",
                    result
                );
                let entries = result?;

                let header_len = entries.len();
                let tuple_ptr = allocate_guest_pointer(&mut caller, (16 * header_len).try_into()?)?;
                let mut ptr = tuple_ptr;
                for item in entries.iter() {
                    let name = &item.0;
                    let value = &item.1;
                    let name_len: u32 = name.len().try_into()?;
                    let value_len: u32 = value.len().try_into()?;

                    let name_ptr = allocate_guest_pointer(&mut caller, name_len)?;
                    let value_ptr = allocate_guest_pointer(&mut caller, value_len)?;

                    let memory = memory_get(&mut caller)?;
                    memory.write(caller.as_context_mut(), name_ptr as _, &name.as_bytes())?;
                    memory.write(caller.as_context_mut(), value_ptr as _, value)?;

                    let pair: [u32; 4] = [name_ptr, name_len, value_ptr, value_len];
                    let raw_pair = u32_array_to_u8(&pair);
                    memory.write(caller.as_context_mut(), ptr as _, &raw_pair)?;

                    ptr = ptr + 16;
                }

                let memory = memory_get(&mut caller)?;
                let result: [u32; 2] = [tuple_ptr, header_len.try_into()?];
                let raw = u32_array_to_u8(&result);
                memory.write(caller.as_context_mut(), out_ptr as _, &raw)?;
                Ok(())
            },
        )?;
        linker.func_wrap(
            "wasi:http/types",
            "incoming-response-headers",
            move |mut caller: Caller<'_, T>, handle: u32| -> anyhow::Result<u32> {
                let ctx = get_cx(caller.data_mut());
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-headers'] call handle={:?}",
                    handle
                );
                let result = Host::incoming_response_headers(ctx, handle);
                tracing::trace!(
                    "[module='wasi:http/types' function='incoming-response-headers'] return result={:?}",
                    result
                );
                result
            },
        )?;
        Ok(())
    }
}

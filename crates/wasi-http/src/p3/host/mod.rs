use crate::p3::bindings::http::types::{Fields, Request, Response};
use anyhow::Context as _;
use core::ops::Deref;
use wasmtime::component::{Resource, ResourceTable};

mod handler;
mod types;

fn get_fields<'a>(
    table: &'a ResourceTable,
    fields: &Resource<Fields>,
) -> wasmtime::Result<&'a Fields> {
    table
        .get(&fields)
        .context("failed to get fields from table")
}

fn get_fields_mut<'a>(
    table: &'a mut ResourceTable,
    fields: &Resource<Fields>,
) -> wasmtime::Result<&'a mut Fields> {
    table
        .get_mut(&fields)
        .context("failed to get fields from table")
}

fn push_fields(table: &mut ResourceTable, fields: Fields) -> wasmtime::Result<Resource<Fields>> {
    table.push(fields).context("failed to push fields to table")
}

fn delete_fields(table: &mut ResourceTable, fields: Resource<Fields>) -> wasmtime::Result<Fields> {
    table
        .delete(fields)
        .context("failed to delete fields from table")
}

fn get_request<'a>(
    table: &'a ResourceTable,
    req: &Resource<Request>,
) -> wasmtime::Result<&'a Request> {
    table.get(req).context("failed to get request from table")
}

fn get_request_mut<'a>(
    table: &'a mut ResourceTable,
    req: &Resource<Request>,
) -> wasmtime::Result<&'a mut Request> {
    table
        .get_mut(req)
        .context("failed to get request from table")
}

fn push_request(table: &mut ResourceTable, req: Request) -> wasmtime::Result<Resource<Request>> {
    table.push(req).context("failed to push request to table")
}

fn delete_request(table: &mut ResourceTable, req: Resource<Request>) -> wasmtime::Result<Request> {
    table
        .delete(req)
        .context("failed to delete request from table")
}

fn get_response<'a>(
    table: &'a ResourceTable,
    res: &Resource<Response>,
) -> wasmtime::Result<&'a Response> {
    table.get(res).context("failed to get response from table")
}

fn get_response_mut<'a>(
    table: &'a mut ResourceTable,
    res: &Resource<Response>,
) -> wasmtime::Result<&'a mut Response> {
    table
        .get_mut(res)
        .context("failed to get response from table")
}

fn push_response(table: &mut ResourceTable, res: Response) -> wasmtime::Result<Resource<Response>> {
    table.push(res).context("failed to push response to table")
}

fn delete_response(
    table: &mut ResourceTable,
    res: Resource<Response>,
) -> wasmtime::Result<Response> {
    table
        .delete(res)
        .context("failed to delete response from table")
}

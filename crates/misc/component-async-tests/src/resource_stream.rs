use crate::util::PipeProducer;
use futures::channel::mpsc;
use wasmtime::Result;
use wasmtime::component::{Accessor, Resource, StreamReader};

use super::Ctx;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "read-resource-stream",
        with: {
            "local:local/resource-stream.x": super::ResourceStreamX,
        },
        imports: {
            "local:local/resource-stream.foo": async | store | trappable,
            default: trappable,
        },
    });
}

pub struct ResourceStreamX;

impl bindings::local::local::resource_stream::HostX for Ctx {
    fn foo(&mut self, x: Resource<ResourceStreamX>) -> Result<()> {
        self.table.get(&x)?;
        Ok(())
    }

    fn drop(&mut self, x: Resource<ResourceStreamX>) -> Result<()> {
        self.table.delete(x)?;
        Ok(())
    }
}

impl bindings::local::local::resource_stream::HostWithStore for Ctx {
    async fn foo<T: 'static>(
        accessor: &Accessor<T, Self>,
        count: u32,
    ) -> wasmtime::Result<StreamReader<Resource<ResourceStreamX>>> {
        accessor.with(|mut access| {
            let (mut tx, rx) = mpsc::channel(usize::try_from(count).unwrap());
            for _ in 0..count {
                tx.try_send(access.get().table.push(ResourceStreamX)?)
                    .unwrap()
            }
            Ok(StreamReader::new(access, PipeProducer::new(rx)))
        })
    }
}

impl bindings::local::local::resource_stream::Host for Ctx {}

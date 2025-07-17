use anyhow::Result;
use wasmtime::component::{
    Accessor, AccessorTask, GuardedStreamWriter, Resource, StreamReader, StreamWriter,
};
use wasmtime_wasi::p2::IoView;

use super::Ctx;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "read-resource-stream",
        with: {
            "local:local/resource-stream/x": super::ResourceStreamX,
        },
        imports: {
            "local:local/resource-stream/foo": async | store | trappable,
            default: trappable,
        },
    });
}

pub struct ResourceStreamX;

impl bindings::local::local::resource_stream::HostX for Ctx {
    fn foo(&mut self, x: Resource<ResourceStreamX>) -> Result<()> {
        self.table().get(&x)?;
        Ok(())
    }

    fn drop(&mut self, x: Resource<ResourceStreamX>) -> Result<()> {
        IoView::table(self).delete(x)?;
        Ok(())
    }
}

impl bindings::local::local::resource_stream::HostWithStore for Ctx {
    async fn foo<T: 'static>(
        accessor: &Accessor<T, Self>,
        count: u32,
    ) -> wasmtime::Result<StreamReader<Resource<ResourceStreamX>>> {
        struct Task {
            tx: StreamWriter<Resource<ResourceStreamX>>,

            count: u32,
        }

        impl<T> AccessorTask<T, Ctx, Result<()>> for Task {
            async fn run(self, accessor: &Accessor<T, Ctx>) -> Result<()> {
                let mut tx = GuardedStreamWriter::new(accessor, self.tx);
                for _ in 0..self.count {
                    let item =
                        accessor.with(|mut view| view.get().table().push(ResourceStreamX))?;
                    tx.write_all(Some(item)).await;
                }
                Ok(())
            }
        }

        let (tx, rx) = accessor.with(|mut view| {
            let instance = view.instance();
            instance.stream(&mut view)
        })?;
        accessor.spawn(Task { tx, count });
        Ok(rx)
    }
}

impl bindings::local::local::resource_stream::Host for Ctx {}

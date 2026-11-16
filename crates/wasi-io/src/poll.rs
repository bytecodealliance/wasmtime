use alloc::boxed::Box;
use core::any::Any;
use core::fmt;
use core::future::Future;
use core::pin::Pin;
use wasmtime::Result;
use wasmtime::component::{Resource, ResourceTable};

pub type DynFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type MakeFuture = for<'a> fn(&'a mut dyn Any) -> DynFuture<'a>;

/// The host representation of the `wasi:io/poll.pollable` resource.
///
/// A pollable is not the same thing as a Rust Future: the same pollable may be used to
/// repeatedly check for readiness of a given condition, e.g. if a stream is readable
/// or writable. So, rather than containing a Future, which can only become Ready once, a
/// `DynPollable` contains a way to create a Future in each call to `poll`.
pub struct DynPollable {
    pub(crate) index: u32,
    pub(crate) make_future: MakeFuture,
    pub(crate) remove_index_on_delete: Option<fn(&mut ResourceTable, u32) -> Result<()>>,
    type_name: &'static str,
}

impl DynPollable {
    /// The name of the concrete type this pollable is subscribed to, as
    /// reported by [`Pollable::type_name`] when it was created by
    /// [`subscribe`].
    ///
    /// This is intended for debugging purposes only.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl fmt::Debug for DynPollable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynPollable")
            .field("index", &self.index)
            .field("type_name", &self.type_name)
            .finish_non_exhaustive()
    }
}

/// The trait used to implement [`DynPollable`] to create a `pollable`
/// resource in `wasi:io/poll`.
///
/// This trait is the internal implementation detail of any pollable resource in
/// this crate's implementation of WASI. The `ready` function is an `async fn`
/// which resolves when the implementation is ready. Using native `async` Rust
/// enables this type's readiness to compose with other types' readiness
/// throughout the WASI implementation.
///
/// This trait is used in conjunction with [`subscribe`] to create a `pollable`
/// resource.
///
/// # Example
///
/// This is a simple example of creating a `Pollable` resource from a few
/// parameters.
///
/// ```
/// # // stub out so we don't need a dep to build the doctests:
/// # mod tokio { pub mod time { pub use std::time::{Duration, Instant}; pub async fn sleep_until(_: Instant) {} } }
/// use tokio::time::{self, Duration, Instant};
/// use wasmtime_wasi_io::{IoView, poll::{Pollable, subscribe, DynPollable}, async_trait};
/// use wasmtime::component::Resource;
/// use wasmtime::Result;
///
/// fn sleep(cx: &mut dyn IoView, dur: Duration) -> Result<Resource<DynPollable>> {
///     let end = Instant::now() + dur;
///     let sleep = MySleep { end };
///     let sleep_resource = cx.table().push(sleep)?;
///     subscribe(cx.table(), sleep_resource)
/// }
///
/// struct MySleep {
///     end: Instant,
/// }
///
/// #[async_trait]
/// impl Pollable for MySleep {
///     async fn ready(&mut self) {
///         tokio::time::sleep_until(self.end).await;
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Pollable: Send + 'static {
    /// An asynchronous function which resolves when this object's readiness
    /// operation is ready.
    ///
    /// This function is invoked as part of `poll` in `wasi:io/poll`. The
    /// meaning of when this function Returns depends on what object this
    /// [`Pollable`] is attached to. When the returned future resolves then the
    /// corresponding call to `wasi:io/poll` will return.
    ///
    /// Note that this method does not return an error. Returning an error
    /// should be done through accessors on the object that this `pollable` is
    /// connected to. The call to `wasi:io/poll` itself does not return errors,
    /// only a list of ready objects.
    async fn ready(&mut self);

    /// The name of the concrete type implementing this trait.
    ///
    /// The default implementation returns the value of
    /// [`core::any::type_name::<Self>()`](core::any::type_name). Calling this
    /// on a trait object such as
    /// [`DynInputStream`](crate::streams::DynInputStream) reports the
    /// underlying concrete type rather than the trait object type.
    ///
    /// This is intended for debugging purposes only.
    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}

/// Creates a `wasi:io/poll.pollable` resource which is subscribed to the provided
/// `resource`.
///
/// If `resource` is an owned resource then it will be deleted when the returned
/// resource is deleted. Otherwise the returned resource is considered a "child"
/// of the given `resource` which means that the given resource cannot be
/// deleted while the `pollable` is still alive.
pub fn subscribe<T>(
    table: &mut ResourceTable,
    resource: Resource<T>,
) -> Result<Resource<DynPollable>>
where
    T: Pollable,
{
    fn make_future<'a, T>(stream: &'a mut dyn Any) -> DynFuture<'a>
    where
        T: Pollable,
    {
        stream.downcast_mut::<T>().unwrap().ready()
    }

    let type_name = table.get(&resource)?.type_name();
    let pollable = DynPollable {
        index: resource.rep(),
        type_name,
        remove_index_on_delete: if resource.owned() {
            Some(|table, idx| {
                let resource = Resource::<T>::new_own(idx);
                table.delete(resource)?;
                Ok(())
            })
        } else {
            None
        },
        make_future: make_future::<T>,
    };

    Ok(table.push_child(pollable, &resource)?)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::streams::{
        DynInputStream, DynOutputStream, InputStream, OutputStream, StreamResult,
    };
    use bytes::Bytes;

    struct TestPollable;

    #[async_trait::async_trait]
    impl Pollable for TestPollable {
        async fn ready(&mut self) {}
    }

    struct TestInput;

    #[async_trait::async_trait]
    impl Pollable for TestInput {
        async fn ready(&mut self) {}
    }

    impl InputStream for TestInput {
        fn read(&mut self, _size: usize) -> StreamResult<Bytes> {
            Ok(Bytes::new())
        }
    }

    struct TestOutput;

    #[async_trait::async_trait]
    impl Pollable for TestOutput {
        async fn ready(&mut self) {}
    }

    impl OutputStream for TestOutput {
        fn write(&mut self, _bytes: Bytes) -> StreamResult<()> {
            Ok(())
        }
        fn flush(&mut self) -> StreamResult<()> {
            Ok(())
        }
        fn check_write(&mut self) -> StreamResult<usize> {
            Ok(0)
        }
    }

    #[test]
    fn dyn_streams_report_concrete_type_name() {
        let input: DynInputStream = Box::new(TestInput);
        assert_eq!(input.type_name(), core::any::type_name::<TestInput>());
        let output: DynOutputStream = Box::new(TestOutput);
        assert_eq!(output.type_name(), core::any::type_name::<TestOutput>());
    }

    #[test]
    fn subscribe_records_type_name() -> Result<()> {
        let mut table = ResourceTable::new();

        let owned = table.push(TestPollable)?;
        let pollable = subscribe(&mut table, owned)?;
        let pollable = table.get(&pollable)?;
        assert_eq!(pollable.type_name(), core::any::type_name::<TestPollable>());
        assert!(format!("{pollable:?}").contains("TestPollable"));

        let input: DynInputStream = Box::new(TestInput);
        let input = table.push(input)?;
        let borrowed = Resource::<DynInputStream>::new_borrow(input.rep());
        let pollable = subscribe(&mut table, borrowed)?;
        assert_eq!(
            table.get(&pollable)?.type_name(),
            core::any::type_name::<TestInput>()
        );

        let output: DynOutputStream = Box::new(TestOutput);
        let output = table.push(output)?;
        let pollable = subscribe(&mut table, output)?;
        assert_eq!(
            table.get(&pollable)?.type_name(),
            core::any::type_name::<TestOutput>()
        );
        Ok(())
    }
}

use super::*;
use std::path::Path;
use test_programs_artifacts::*;
use wasmtime_wasi::preview2::{
    command::{add_to_linker, Command},
    HostInputStream, HostOutputStream, StdinStream, StdoutStream, StreamResult, Subscribe,
};

struct BufferInner {
    // Individual writes
    buffer: std::collections::VecDeque<bytes::Bytes>,

    // Write budget
    budget: usize,
}

impl BufferInner {
    fn new(budget: usize) -> Self {
        BufferInner {
            buffer: std::collections::VecDeque::new(),
            budget,
        }
    }
}

#[derive(Clone)]
struct Buffer(std::sync::Arc<std::sync::Mutex<BufferInner>>);

impl Buffer {
    fn new(budget: usize) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(
            BufferInner::new(budget),
        )))
    }

    fn read_ready(&mut self) -> bool {
        let mut inner = self.0.lock().unwrap();
        while let Some(bytes) = inner.buffer.front() {
            if !bytes.is_empty() {
                return true;
            }

            inner.buffer.pop_front();
        }

        false
    }

    fn write_ready(&self) -> bool {
        let inner = self.0.lock().unwrap();
        inner.budget > 0
    }

    fn read(&mut self, mut size: usize) -> StreamResult<bytes::Bytes> {
        let mut buf = bytes::BytesMut::new();
        let mut inner = self.0.lock().unwrap();
        while size > 0 {
            if inner.buffer.is_empty() {
                break;
            }

            let chunk = inner.buffer.front_mut().unwrap();

            let off = size.min(chunk.len());
            let bs = chunk.split_to(off);
            size -= off;
            buf.extend(bs);
            if chunk.is_empty() {
                inner.buffer.pop_front();
            }
        }

        let buf = buf.freeze();
        inner.budget += buf.len();

        Ok(buf)
    }

    fn check_write(&mut self) -> usize {
        let inner = self.0.lock().unwrap();
        inner.budget
    }

    fn write(&mut self, chunk: bytes::Bytes) {
        let mut inner = self.0.lock().unwrap();
        assert!(chunk.len() <= inner.budget);
        inner.budget -= chunk.len();
        inner.buffer.push_back(chunk);
    }
}

impl StdinStream for Buffer {
    fn stream(&self) -> Box<dyn wasmtime_wasi::preview2::HostInputStream> {
        Box::new(ReadStream(self.clone()))
    }

    fn isatty(&self) -> bool {
        false
    }
}

struct ReadStream(Buffer);

#[async_trait::async_trait]
impl Subscribe for ReadStream {
    async fn ready(&mut self) {
        if !self.0.read_ready() {
            std::future::pending().await
        }
    }
}

impl HostInputStream for ReadStream {
    fn read(&mut self, size: usize) -> StreamResult<bytes::Bytes> {
        self.0.read(size)
    }
}

impl StdoutStream for Buffer {
    fn stream(&self) -> Box<dyn wasmtime_wasi::preview2::HostOutputStream> {
        Box::new(WriteStream(self.clone()))
    }

    fn isatty(&self) -> bool {
        false
    }
}

struct WriteStream(Buffer);

#[async_trait::async_trait]
impl Subscribe for WriteStream {
    async fn ready(&mut self) {
        if !self.0.write_ready() {
            std::future::pending().await
        }
    }
}

impl HostOutputStream for WriteStream {
    fn write(&mut self, bytes: bytes::Bytes) -> StreamResult<()> {
        Ok(self.0.write(bytes))
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(self.0.check_write())
    }
}

async fn run(path: &str) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_str().unwrap();
    let mut config = Config::new();
    config.async_support(true).wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker)?;

    let buffer = Buffer::new(128);

    let producer = {
        let mut builder = StoreBuilder::new(name)?;
        builder.builder.env("PIPED_SIDE", "PRODUCER");
        builder.stdout(Box::new(buffer.clone()));
        let (mut store, _td) = builder.build(&engine)?;
        let component = Component::from_file(&engine, path)?;
        let (producer, _) = Command::instantiate_async(&mut store, &component, &linker).await?;

        tokio::task::spawn(async move {
            producer
                .wasi_cli_run()
                .call_run(&mut store)
                .await?
                .map_err(|()| anyhow::anyhow!("run returned a failure"))
        })
    };

    let consumer = {
        let mut builder = StoreBuilder::new(name)?;
        builder.stdin(Box::new(buffer));
        builder.builder.env("PIPED_SIDE", "CONSUMER");
        let (mut store, _td) = builder.build(&engine)?;
        let component = Component::from_file(&engine, path)?;
        let (consumer, _) = Command::instantiate_async(&mut store, &component, &linker).await?;

        tokio::task::spawn(async move {
            consumer
                .wasi_cli_run()
                .call_run(&mut store)
                .await?
                .map_err(|()| anyhow::anyhow!("run returned a failure"))
        })
    };

    let (producer, consumer) = tokio::try_join!(producer, consumer)?;
    producer?;
    consumer
}

foreach_piped!(assert_test_exists);

// Below here is mechanical: there should be one test for every binary in
// wasi-tests.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn piped_simple() {
    run(PIPED_SIMPLE_COMPONENT).await.unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn piped_polling() {
    run(PIPED_POLLING_COMPONENT).await.unwrap()
}

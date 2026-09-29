use futures::join;
use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, OpenFlags, PathFlags,
};
use test_programs::p3::{wasi, wit_stream};
use wit_bindgen::StreamResult;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];
        test_chunked_write(dir, "chunked_write.txt").await;
        Ok(())
    }
}

fn bytes(offset: &mut usize, len: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        buf.push(((*offset + i) % 251) as u8);
    }
    *offset += len;
    buf
}

async fn test_chunked_write(dir: &Descriptor, filename: &str) {
    let mut len = 16;
    let mut pos = 0;

    let file = dir
        .open_at(
            PathFlags::empty(),
            filename.to_string(),
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .await
        .expect("creating a file for writing");

    let (mut tx, rx) = wit_stream::new();
    join! {
        async {
            file.write_via_stream(rx, 0).await.unwrap();
        },
        async {
            loop {
                // Wasmtime shouldn't buffer this much data by default on the
                // host, something should have done a short write earlier.
                assert!(len <= 128 << 20);
                let (result, remaining) = tx.write(bytes(&mut pos, len)).await;
                assert!(matches!(result, StreamResult::Complete(_)), "bad result {result:?}");
                if remaining.remaining() == 0 {
                    len = len.checked_mul(2).unwrap();
                } else {
                    pos -= remaining.remaining();
                    break;
                }
            }
            drop(tx);
        },
    };

    let expected = bytes(&mut 0, pos);
    let (rx, result) = file.read_via_stream(0);
    let read_back = rx.collect().await;
    result.await.unwrap();

    assert_eq!(
        read_back.len(),
        expected.len(),
        "wrong number of bytes read back"
    );
    assert!(
        read_back == expected,
        "contents differ after a chunked write"
    );
}

fn main() {
    unreachable!()
}

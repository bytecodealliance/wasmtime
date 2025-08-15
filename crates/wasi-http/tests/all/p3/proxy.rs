use crate::p3::incoming::run_wasi_http;
use anyhow::Result;
use bytes::Bytes;
use flate2::{
    Compression,
    write::{DeflateDecoder, DeflateEncoder},
};
use futures::SinkExt;
use http::HeaderValue;
use std::io::Write;
use std::path::Path;
use tokio::fs;
use wasm_compose::{
    composer::ComponentComposer,
    config::{Config, Dependency, Instantiation, InstantiationArg},
};
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;

#[ignore = "unimplemented"] // TODO: implement
#[tokio::test]
pub async fn p3_http_echo() -> Result<()> {
    test_http_echo(test_programs_artifacts::P3_HTTP_ECHO_COMPONENT, false).await
}

#[ignore = "unimplemented"] // TODO: implement
#[tokio::test]
pub async fn p3_http_middleware() -> Result<()> {
    let tempdir = tempfile::tempdir()?;
    let echo = &fs::read(test_programs_artifacts::P3_HTTP_ECHO_COMPONENT).await?;
    let middleware = &fs::read(test_programs_artifacts::P3_HTTP_MIDDLEWARE_COMPONENT).await?;

    let path = tempdir.path().join("temp.wasm");
    fs::write(&path, compose(middleware, echo).await?).await?;
    test_http_echo(&path.to_str().unwrap(), true).await
}

async fn compose(a: &[u8], b: &[u8]) -> Result<Vec<u8>> {
    let dir = tempfile::tempdir()?;

    let a_file = dir.path().join("a.wasm");
    fs::write(&a_file, a).await?;

    let b_file = dir.path().join("b.wasm");
    fs::write(&b_file, b).await?;

    ComponentComposer::new(
        &a_file,
        &wasm_compose::config::Config {
            dir: dir.path().to_owned(),
            definitions: vec![b_file.to_owned()],
            ..Default::default()
        },
    )
    .compose()
}

#[ignore = "unimplemented"] // TODO: implement
#[tokio::test]
pub async fn p3_http_middleware_with_chain() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("temp.wasm");

    fs::copy(
        test_programs_artifacts::P3_HTTP_ECHO_COMPONENT,
        &dir.path().join("chain-http.wasm"),
    )
    .await?;

    let bytes = ComponentComposer::new(
        Path::new(test_programs_artifacts::P3_HTTP_MIDDLEWARE_WITH_CHAIN_COMPONENT),
        &Config {
            dir: dir.path().to_owned(),
            definitions: Vec::new(),
            search_paths: Vec::new(),
            skip_validation: false,
            import_components: false,
            disallow_imports: false,
            dependencies: [(
                "local:local/chain-http".to_owned(),
                Dependency {
                    path: test_programs_artifacts::P3_HTTP_ECHO_COMPONENT.into(),
                },
            )]
            .into_iter()
            .collect(),
            instantiations: [(
                "root".to_owned(),
                Instantiation {
                    dependency: Some("local:local/chain-http".to_owned()),
                    arguments: [(
                        "local:local/chain-http".to_owned(),
                        InstantiationArg {
                            instance: "local:local/chain-http".into(),
                            export: Some("wasi:http/handler@0.3.0-draft".into()),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
        },
    )
    .compose()?;
    fs::write(&path, &bytes).await?;

    test_http_echo(&path.to_str().unwrap(), true).await
}

async fn test_http_echo(component: &str, use_compression: bool) -> Result<()> {
    let body = b"And the mome raths outgrabe";

    // Prepare the raw body, optionally compressed if that's what we're
    // testing.
    let raw_body = if use_compression {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(body).unwrap();
        Bytes::from(encoder.finish().unwrap())
    } else {
        Bytes::copy_from_slice(body)
    };

    // Prepare the http_body body, modeled here as a channel with the body
    // chunk above buffered up followed by some trailers. Note that trailers
    // are always here to test that code paths throughout the components.
    let (mut body_tx, body_rx) = futures::channel::mpsc::channel::<Result<_, ErrorCode>>(2);
    body_tx
        .send(Ok(http_body::Frame::data(raw_body)))
        .await
        .unwrap();
    body_tx
        .send(Ok(http_body::Frame::trailers({
            let mut trailers = http::HeaderMap::new();
            assert!(
                trailers
                    .insert("fizz", http::HeaderValue::from_static("buzz"))
                    .is_none()
            );
            trailers
        })))
        .await
        .unwrap();

    // Build the `http::Request`, optionally specifying compression-related
    // headers.
    let mut request = http::Request::builder()
        .uri("http://localhost/")
        .method(http::Method::GET)
        .header("foo", "bar");
    if use_compression {
        request = request
            .header("content-encoding", "deflate")
            .header("accept-encoding", "nonexistent-encoding, deflate");
    }

    // Send this request to wasm and assert that success comes back.
    //
    // Note that this will read the entire body internally and wait for
    // everything to get collected before proceeding to below.
    let response = run_wasi_http(
        component,
        request.body(http_body_util::StreamBody::new(body_rx))?,
    )
    .await?
    .unwrap();
    assert!(response.status().as_u16() == 200);

    // Our input header should be echo'd back.
    assert_eq!(
        response.headers().get("foo"),
        Some(&HeaderValue::from_static("bar"))
    );

    // The compression headers should be set if `use_compression` was turned
    // on.
    if use_compression {
        assert_eq!(
            response.headers().get("content-encoding"),
            Some(&HeaderValue::from_static("deflate"))
        );
        assert!(response.headers().get("content-length").is_none());
    }

    // Trailers should be echo'd back as well.
    assert_eq!(
        response.body().trailers().unwrap().get("fizz"),
        Some(&HeaderValue::from_static("buzz"))
    );

    // And our body should match our original input body as well.
    let (_, collected_body) = response.into_parts();
    let collected_body = collected_body.to_bytes();

    let response_body = if use_compression {
        let mut decoder = DeflateDecoder::new(Vec::new());
        decoder.write_all(&collected_body)?;
        decoder.finish()?
    } else {
        collected_body.to_vec()
    };
    assert_eq!(response_body, body.as_slice());
    Ok(())
}

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::header::HOST;
use hyper::Request;
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tracing::debug;
use url::{Position, Url};

use crate::io::TokioIo;
use crate::{ast::Block, opcode::Opcode, parser};

pub struct Client {
    runtime: Runtime,
    base: Url,
}

impl Client {
    pub fn new(server_url: impl AsRef<str>) -> Result<Self> {
        let base = Url::parse(server_url.as_ref()).context("invalid server URL")?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        Ok(Self { runtime, base })
    }

    pub fn opcode(&self, opcode: Opcode) -> Result<Block> {
        // Model for response JSON data.
        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct Response {
            instruction: String,
            encoding: String,
            semantics: String,
        }

        // Build request URL with the opcode query parameter.
        let opcode = opcode.to_string();
        let mut url = self.base.clone();
        url.query_pairs_mut().append_pair("opcode", &opcode);

        // Issue GET request.
        let body = self.runtime.block_on(self.get(&url))?;
        let res: Response = serde_json::from_slice(&body).context("invalid server response")?;

        debug!(%res.encoding, %res.semantics);

        // Ensure response instruction matches.
        if res.instruction != opcode {
            bail!("response opcode mismatch");
        }

        // Parse semantics.
        let block = parser::parse(&res.semantics)?;

        Ok(block)
    }

    // Perform an HTTP/1.1 GET against the ASLp server and return the response
    // body bytes.
    async fn get(&self, url: &Url) -> Result<Bytes> {
        let host = url.host_str().context("server URL missing host")?;
        let port = url
            .port_or_known_default()
            .context("server URL missing port")?;
        let authority = format!("{host}:{port}");

        // Origin-form request target: path and query.
        let target = &url[Position::BeforePath..];

        // Connect and drive the connection.
        let stream = TcpStream::connect(&authority)
            .await
            .with_context(|| format!("failed to connect to ASLp server at {authority}"))?;
        let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo(stream)).await?;
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                debug!(%err, "ASLp server connection error");
            }
        });

        // Send the request and collect the response body.
        let req = Request::get(target)
            .header(HOST, &authority)
            .body(Empty::<Bytes>::new())?;
        let res = sender.send_request(req).await?;
        if !res.status().is_success() {
            bail!("ASLp server returned status {}", res.status());
        }

        Ok(res.into_body().collect().await?.to_bytes())
    }
}

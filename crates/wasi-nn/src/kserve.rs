//! Implements the wasi-nn API.

use std::collections::HashMap;
use std::io::Read;
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};

use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Method, Request, Response, Uri};
use hyper::client::conn::http1::{Connection, SendRequest};
use hyper::http::uri::Authority;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use hyper::body::Buf;
use wiggle::async_trait_crate::async_trait;


#[derive(Default)]
pub(crate) struct KServeBackend();

unsafe impl Send for KServeBackend {}

unsafe impl Sync for KServeBackend {}

impl Backend for KServeBackend {
    fn name(&self) -> &str {
        "KServe"
    }

    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        return Err(BackendError::UnsupportedOperation("load"));
    }

    fn load_from_bytes(
        &mut self,
        model_bytes: &Vec<Vec<u8>>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        return Err(BackendError::UnsupportedOperation("load_from_bytes"));
    }
}

struct KServeGraph();

unsafe impl Send for KServeGraph {}

unsafe impl Sync for KServeGraph {}

impl BackendGraph for KServeGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }
}

struct KServeExecutionContext {
    client: KServeClient,
    inputs: Vec<KServeTensorMetadata>,
    outputs: Vec<KServeTensorMetadata>,
}

#[async_trait]
impl BackendExecutionContext for KServeExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }

    async fn compute(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        return Err(BackendError::UnsupportedOperation("get_output"));
        // let output_name = self.0.get_output_name(index as usize)?;
        // let blob = self.1.get_blob(&output_name)?;
        // let blob_size = blob.byte_len()?;
        // if blob_size > destination.len() {
        //     return Err(BackendError::NotEnoughMemory(blob_size));
        // }
        //
        // // Copy the tensor data into the destination buffer.
        // destination[..blob_size].copy_from_slice(blob.buffer()?);
        // Ok(blob_size as u32)
    }
}

/// Return the execution target string expected by OpenVINO from the
/// `ExecutionTarget` enum provided by wasi-nn.
fn map_execution_target_to_string(target: ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::Cpu => "CPU",
        ExecutionTarget::Gpu => "GPU",
        ExecutionTarget::Tpu => "TPU",
    }
}

struct KServeClient {
    server_url: String,
    url: Uri,
    // stream: TcpStream,
    task: JoinHandle<()>,
    authority: Authority,
    sender: SendRequest<Full<Bytes>>,
    // conn: Connection<TcpStream,Full<Bytes>>
}

impl KServeClient {
    //Bulk of the logic will be here.
    pub async fn new(server_url: String) -> Self {
        // Parse the server url
        let url = server_url.parse::<hyper::Uri>().expect("Unable to parse url.");

        // Get the host and the port
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // Open a TCP connection to the remote host
        let stream = TcpStream::connect(address).await
            .expect("Unable to connect to server.");

        // The authority of our URL will be the hostname of the httpbin remote
        let authority = url.authority().unwrap().clone();
        let (sender, conn) = hyper::client::conn::http1::handshake(stream).await
            .expect("Unable to perform http handshake with server.");
        // Spawn a task to poll the connection, driving the HTTP state
        let task = tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });
        Self {
            server_url,
            url,
            // stream,
            task,
            authority,
            sender,
            // conn
        }
    }


    async fn send_request(&mut self, request: Request<Full<Bytes>>) -> Response<Incoming> {
        // Perform a TCP handshake
        // let (mut sender, conn) = hyper::client::conn::http1::handshake(self.stream).await?;

        // Await the response...
        let mut res = self.sender.send_request(request).await
            .expect("Unable to send HTTP request to server.");

        println!("Response status: {}", res.status());
        res
    }

    pub async fn get_server_metadata(&mut self) -> KServeServerMetadata {
        let server_metadata_url = build_server_metadata_url(&self.server_url)
            .parse::<hyper::Uri>()
            .expect("Unable to parse url.");
        println!("{:?}", server_metadata_url);
        // Create an HTTP request with an empty body and a HOST header
        let req = Request::builder()
            .uri(server_metadata_url)
            .method(Method::GET)
            .header(hyper::header::HOST, self.authority.as_str())
            .body(Full::from(""))
            .expect("Unable to build HTTP request");
        let res = self.send_request(req).await;
        // .expect("Unable to receive HTTP response from server.");


        // asynchronously aggregate the chunks of the body
        let body = res.collect().await.expect("Unable to collect response form server").aggregate();
        // let mut s: String = String::new();
        // body.reader().read_to_string(&mut s);
        // println!("{:?}", s);
        // try to parse as json with serde_json
        let server_metadata: KServeServerMetadata = serde_json::from_reader(body.reader())
            .expect("Unable to deserialize json response from server.");
        server_metadata
    }

    // async fn get_model_metadata(&self) -> KServeModelConfig {
    //
    //     // Create an HTTP request with an empty body and a HOST header
    //     let req = Request::builder()
    //         .uri(self.url.clone())
    //         .method(Method::GET)
    //         .header(hyper::header::HOST, self.authority.as_str())
    //         .header()
    //         .body(Empty::<Bytes>::new())?;
    //
    //     // Await the response...
    //     let mut res = self.send_request(req);
    //
    //     println!("Response status: {:?}", res.status());
    //
    // }
}

#[derive(Debug, Serialize, Deserialize)]
struct KServeServerMetadata {
    name: String,
    version: String,
    extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KServeTensorParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeTensorMetadata {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatype: Option<KServeDatatype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<KServeTensorParameters>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceRequest {
    model_name: String,
    inputs: Vec<KServeTensorMetadata>,
    outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceResult {
    outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeModelConfig {
    //For some weird reason the json here expects name, instead of model_name like in inference request.
    pub name: String,
    pub backend: String,
    pub inputs: Vec<KServeTensorMetadata>,
    pub outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeRepositoryLoadRequest {
    parameters: KServeParameters,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KServeRepositoryLoadErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KServeParameterValue {
    Bool(bool),
    Number(Number),
    String(String),
    Config(KServeModelConfig),
}

type KServeParameters = HashMap<String, KServeParameterValue>;

#[derive(Debug, Serialize, Deserialize)]
pub enum KServeDatatype {
    BOOL,
    UINT8,
    UINT16,
    UINT32,
    UINT64,
    INT8,
    INT16,
    INT32,
    INT64,
    FP16,
    FP32,
    FP64,
    BYTES,
    BF16,
}

fn build_server_metadata_url(server_url: &String) -> String {
    format!("{}/v2", server_url)
}

fn build_inference_url(model_name: &String) -> String {
    format!("/v2/models/{}/infer", model_name)
}

#[tokio::test]
async fn test_get_server_metadata() {
    eprintln!("Attempting to retrieve server metadata from Triton.");
    let mut kserve_client = KServeClient::new(String::from("http://localhost:8000")).await;

    let server_metadata = kserve_client.get_server_metadata().await;

    eprintln!("{:?}", server_metadata);
}

#[test]
fn test_kserve_model_config_deserialization() {
    let json = r#"
    {
          "name": "mymodel",
          "backend": "onnxruntime",
          "inputs": [{
              "name": "INPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
            }
          ],
          "outputs": [{
              "name": "OUTPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
          }]
    }
    "#;

    let output = serde_json::from_str::<KServeModelConfig>(json).unwrap();
}

#[test]
fn test_deserialization() {
    let json = r#"
    {
      "parameters": {
        "a" : true,
        "b" : 1.0,
        "c" : 1,
        "d" : "efg",
        "config": {
          "name": "mymodel",
          "backend": "onnxruntime",
          "inputs": [{
              "name": "INPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
            }
          ],
          "outputs": [{
              "name": "OUTPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
          }]
        }
      }
    }
    "#;

    let output = serde_json::from_str::<KServeRepositoryLoadRequest>(json).unwrap();
}
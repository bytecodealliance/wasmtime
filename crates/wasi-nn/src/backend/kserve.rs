//! Implements the wasi-nn API.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Error, ErrorKind, Read};
use std::sync::{Arc, Mutex};

use byteorder::{LittleEndian, ReadBytesExt};
use http_body_util::{BodyExt, Empty, Full};
use hyper::{body, Method, Request, Response, StatusCode, Uri};
use hyper::body::{Bytes, Incoming};
use hyper::body::Buf;
use hyper::client::conn::http1::{Connection, SendRequest};
use hyper::header::HeaderName;
use hyper::http::uri::Authority;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::{json, Number};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use wasmtime::component::__internal::wasmtime_environ::object::BigEndian;
use wiggle::async_trait_crate::async_trait;

use crate::{ExecutionContext, Graph};
use crate::backend::{Backend, BackendError, BackendExecutionContext, BackendFromDir, BackendGraph};
use crate::wit::types::{ExecutionTarget, Tensor, TensorType};

const INFERENCE_HEADER_CONTENT_LENGTH: &str = "inference-header-content-length";
const BINARY_DATA_SIZE: &str = "binary_data_size";

pub(crate) struct KServeBackend {
    server_url: String,
}

impl Default for KServeBackend {
    fn default() -> Self {
        Self {
            server_url: String::from("http://localhost:8000"),
        }
    }
}

impl Backend for KServeBackend {
    fn name(&self) -> &str {
        "KServe"
    }

    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError> {
        return Err(BackendError::UnsupportedOperation("load"));
    }

    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir> {
        None
    }
}

struct KServeGraph();

impl BackendGraph for KServeGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }
}

struct KServeExecutionContext {
    client: KServeClient,
    input_mapping: HashMap<u32, String>,
    model_metadata: KServeModelMetadata,
    inputs: Vec<KServeTensor>,
    outputs: Vec<KServeTensor>,
}

#[async_trait]
impl BackendExecutionContext for KServeExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor) -> Result<(), BackendError> {
        let datatype = map_tensor_type_to_datatype(tensor.tensor_type);
        let data = read_tensor_elements(tensor);

        let kserve_tensor = KServeTensor {
            metadata: KServeTensorMetadata {
                name: self.input_mapping[&index].clone(),
                shape: tensor.dimensions.to_vec(),
                datatype,
                parameters: None,
            },
            data,
        };

        self.inputs.push(kserve_tensor);

        // return Err(BackendError::UnsupportedOperation("init_execution_context"));;
        Ok(())
    }

    async fn compute(&mut self) -> Result<(), BackendError> {
        let outputs = self.model_metadata.outputs.iter().map(|mm| KServeRequestOutput {
            name: mm.name.clone(),
            parameters: KServeBinaryInferenceParameters {
                binary_data: None,
                binary_data_output: Some(true),
                binary_data_size: None,
            },
        }).collect();

        let inference_request = KServeInferenceRequest {
            id: None,
            parameters: None,
            inputs: &self.inputs,
            outputs: &outputs,
        };
        let result = self.client.inference_request(&self.model_metadata.name, &inference_request).await?;
        self.outputs = result.outputs;
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

fn read_tensor_elements(tensor: &Tensor) -> Vec<KServeTensorElement> {
    let mut cursor = Cursor::new(tensor.data.as_slice().to_vec());

    let mut data = Vec::with_capacity(tensor.data.len() / map_tensor_type_to_size(tensor.tensor_type));

    while cursor.has_remaining() {
        data.push(match tensor.tensor_type {
            TensorType::U8 => KServeTensorElement::Number(Number::from(cursor.read_u8().unwrap())),
            TensorType::Bf16 => panic!("bf16 is not supported for kserve backend."),
            TensorType::Fp16 => panic!("bf16 is not supported for kserve backend."),
            TensorType::I32 => KServeTensorElement::Number(Number::from(cursor.read_i32::<LittleEndian>().unwrap())),
            TensorType::Fp32 => KServeTensorElement::Number(Number::from_f64(cursor.read_f32::<LittleEndian>().unwrap() as f64).unwrap()),
            TensorType::Bytes => unsafe {
                assert_eq!(tensor.dimensions[0], 1);
                assert_eq!(tensor.dimensions[1], 1);
                KServeTensorElement::String(String::from_utf8_unchecked(tensor.data.to_vec()))
            }
        });
    }

    data
}

fn map_tensor_type_to_size(tensor_type: TensorType) -> usize {
    match tensor_type {
        TensorType::U8 => 1,
        TensorType::Bf16 => 2,
        TensorType::Fp16 => 2,
        TensorType::I32 => 4,
        TensorType::Fp32 => 4,
        TensorType::Bytes => 0,
    }
}

fn map_tensor_type_to_datatype(tensor_type: TensorType) -> KServeDatatype {
    match tensor_type {
        TensorType::U8 => KServeDatatype::UINT8,
        TensorType::Bf16 => KServeDatatype::BF16,
        TensorType::Fp16 => KServeDatatype::FP16,
        TensorType::I32 => KServeDatatype::INT32,
        TensorType::Fp32 => KServeDatatype::FP32,
        TensorType::Bytes => KServeDatatype::BYTES,
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
    /// Creates a new KServe client.
    /// TODO: Add support for HTTPS connections.
    pub async fn new(server_url: String) -> Self {
        // Parse the server url
        let url = server_url
            .parse::<hyper::Uri>()
            .expect("Unable to parse url.");

        // Get the host and the port
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // Open a TCP connection to the remote host
        let stream = TcpStream::connect(address)
            .await
            .expect("Unable to connect to server.");

        // The authority of our URL will be the hostname of the httpbin remote
        let authority = url.authority().unwrap().clone();
        let (sender, conn) = hyper::client::conn::http1::handshake(stream)
            .await
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
        let mut res = self
            .sender
            .send_request(request)
            .await
            .expect("Unable to send HTTP request to server.");

        println!("Response status: {}", res.status());
        res
    }

    pub async fn get_server_metadata(&mut self) -> Result<KServeServerMetadata, BackendError> {
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
        if res.status() == StatusCode::OK {
            try_deserialize(res).await
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to retrieve server metadata."),
            )))
        }
        // let mut s: String = String::new();
        // body.reader().read_to_string(&mut s);
        // println!("{:?}", s);
        // try to parse as json with serde_json
    }

    async fn get_model_metadata(
        &mut self,
        model_name: &String,
    ) -> Result<KServeModelMetadata, BackendError> {
        let model_metadata_url = build_model_metadata_url(&self.server_url, model_name);

        let req = Request::builder()
            .uri(model_metadata_url)
            .method(Method::GET)
            .header(hyper::header::HOST, self.authority.as_str())
            .body(Full::from(""))
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        let mut res = self.send_request(req).await;

        println!("Response status: {:?}", res.status());

        if res.status() == StatusCode::OK {
            try_deserialize(res).await
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to retrieve model metadata."),
            )))
        }
    }

    async fn inference_request(
        &mut self,
        model_name: &String,
        request: &KServeInferenceRequest<'_>,
    ) -> Result<KServeInferenceResult, BackendError> {
        let inference_url = build_inference_url(&self.server_url, model_name);
        println!("Inference url: {}", inference_url);
        let json_bytes =
            serde_json::to_vec(request).expect("Unable to serialize inference request.");
        // Create an HTTP request with an empty body and a HOST header
        let req = Request::builder()
            .uri(inference_url)
            .method(Method::POST)
            .header(hyper::header::HOST, self.authority.as_str())
            .body(Full::<Bytes>::from(json_bytes))
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        // Await the response...
        let mut res = self.send_request(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        println!("Response status: {:?}", res.status());

        if res.status() == StatusCode::OK {
            try_deserialize(res).await
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to perform inference request."),
            )))
        }
    }

    async fn binary_inference_request(
        &mut self,
        model_name: &String,
        request: &KServeBinaryInferenceRequest,
        tensors: &Vec<Vec<u8>>,
    ) -> Result<(KServeBinaryInferenceResult, Vec<Vec<u8>>), BackendError> {
        let mut tensor_length: usize = 0;

        //Make sure that tensors are the expected length
        for i in 0..request.inputs.len() {
            let binary_data_size: usize = request.inputs[i]
                .parameters
                .as_ref()
                .unwrap()
                .binary_data_size
                .unwrap() as usize;
            assert_eq!(tensors[i].len(), binary_data_size);
            tensor_length += binary_data_size;
        }

        //TODO: It's not clear from the docs whether outputs can be of mixed binary and json types. If so, we need to handle mixed types.
        //If not, then we need to make sure that binary_data: true is present for all outputs.

        let inference_url = build_inference_url(&self.server_url, model_name);
        println!("Inference url: {}", inference_url);

        let json_bytes =
            serde_json::to_vec(request).expect("Unable to serialize inference request.");
        let inference_header_length = json_bytes.len();
        let content_length = tensor_length + inference_header_length;
        let mut body: Vec<u8> = Vec::with_capacity(content_length);
        body.extend(&json_bytes);
        for tensor in tensors {
            body.extend(tensor);
        }
        println!(
            "JSON Header: {}",
            std::str::from_utf8(json_bytes.as_slice()).unwrap()
        );
        println!(
            "JSON Header: {}",
            std::str::from_utf8(body.as_slice()).unwrap()
        );
        // Create an HTTP request with an empty body and a HOST header
        let req = Request::builder()
            .uri(inference_url)
            .method(Method::POST)
            .header(hyper::header::HOST, self.authority.as_str())
            .header(
                inference_content_length_header(),
                inference_header_length.to_string(),
            )
            .header(hyper::header::CONTENT_LENGTH, content_length)
            .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
            .body(Full::<Bytes>::from(body))
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        // Await the response...
        let mut res = self.send_request(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        println!("Response status: {:?}", res.status());

        let response_inference_content_length_header = res
            .headers()
            .get(inference_content_length_header())
            .unwrap()
            .to_str()
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        let response_inference_content_length: usize = response_inference_content_length_header
            .parse()
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        if res.status() == StatusCode::OK {
            let response_bytes = get_body_bytes(res).await?;
            let response_json_bytes = &response_bytes[..response_inference_content_length];
            let inference_result: KServeBinaryInferenceResult =
                serde_json::from_slice(&response_json_bytes)
                    .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;
            let mut start_index = 0;
            let output_tensors = inference_result
                .outputs
                .iter()
                .map(|output_metadata| {
                    let tensor_length = output_metadata
                        .parameters
                        .as_ref()
                        .unwrap()
                        .binary_data_size
                        .unwrap();
                    let end_index = start_index + tensor_length as usize;
                    let tensor_bytes = (&response_bytes[start_index..end_index]).to_vec();
                    start_index = end_index;
                    tensor_bytes
                })
                .collect();
            Ok((inference_result, output_tensors))
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to retrieve model metadata."),
            )))
        }
    }

    async fn inference_request_bytes(
        &mut self,
        model_name: &String,
        request: &KServeBinaryInferenceRequest,
    ) -> Result<Vec<u8>, BackendError> {
        let model_metadata_url = build_inference_url(&self.server_url, model_name);
        let json_bytes =
            serde_json::to_vec(request).expect("Unable to serialize inference request.");
        let inference_header_length = json_bytes.len();

        // Create an HTTP request with an empty body and a HOST header
        let req = Request::builder()
            .uri(model_metadata_url)
            .method(Method::POST)
            .header(hyper::header::HOST, self.authority.as_str())
            .header(
                HeaderName::from_static(INFERENCE_HEADER_CONTENT_LENGTH),
                inference_header_length.to_string(),
            )
            .body(Full::<Bytes>::from(json_bytes))
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        // Await the response...
        let mut res = self.send_request(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        println!("Response status: {:?}", res.status());

        if res.status() == StatusCode::OK {
            get_body_bytes(res).await
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to perform inference request"),
            )))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct KServeServerMetadata {
    name: String,
    version: String,
    extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct KServeBinaryInferenceParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeTensor {
    #[serde(flatten)]
    metadata: KServeTensorMetadata,
    data: Vec<KServeTensorElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeRequestOutput {
    name: String,
    parameters: KServeBinaryInferenceParameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeTensorMetadata {
    name: String,
    shape: Vec<u32>,
    datatype: KServeDatatype,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<KServeBinaryInferenceParameters>,
}

#[derive(Debug, Serialize)]
pub struct KServeInferenceRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<KServeParameters>,
    inputs: &'a Vec<KServeTensor>,
    outputs: &'a Vec<KServeRequestOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeInferenceResult {
    model_name : String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_version: Option<u32>,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameter: Option<KServeParameters>,
    outputs: Vec<KServeTensor>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceRequest {
    model_name: String,
    inputs: Vec<KServeTensorMetadata>,
    outputs: Vec<KServeRequestOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceResult {
    outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeModelMetadata {
    //For some weird reason the json here expects name, instead of model_name like in inference request.
    pub name: String,
    pub platform: String,
    pub versions: Option<Vec<String>>,
    pub inputs: Vec<KServeTensorMetadata>,
    pub outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeModelConfig {
    //For some weird reason the json here expects name, instead of model_name like in inference request.
    pub name: String,
    pub backend: String,
    pub versions: Option<Vec<String>>,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KServeTensorElement {
    Bool(bool),
    Number(Number),
    String(String),
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

async fn get_body_bytes(response: Response<Incoming>) -> Result<Vec<u8>, BackendError> {
    response
        .collect()
        .await
        .map(|collected| collected.to_bytes().to_vec())
        .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))
}

async fn try_deserialize<T: DeserializeOwned>(
    response: Response<Incoming>,
) -> Result<T, BackendError> {
    let body = response
        .collect()
        .await
        .map(|collected| collected.aggregate())
        .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

    serde_json::from_reader(body.reader())
        .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))
}

fn build_server_metadata_url(server_url: &String) -> String {
    format!("{}/v2", server_url)
}

fn build_model_metadata_url(server_url: &String, model_name: &String) -> String {
    format!("{}/v2/models/{}", server_url, model_name)
}

fn build_model_metadata_url_for_version(
    server_url: &String,
    model_name: &String,
    version: u32,
) -> String {
    format!(
        "{}/v2/models/{}/versions/{}",
        server_url, model_name, version
    )
}

fn build_inference_url(server_url: &String, model_name: &String) -> String {
    format!("{}/v2/models/{}/infer", server_url, model_name)
}

fn inference_content_length_header() -> HeaderName {
    hyper::header::HeaderName::from_static(INFERENCE_HEADER_CONTENT_LENGTH)
}

#[tokio::test]
async fn test_binary_inference() {
    let mut kserve_client = KServeClient::new(String::from("http://localhost:8000")).await;
    let prompt = String::from("captain america, 4k");
    let input = KServeTensorMetadata {
        name: String::from("prompt"),
        shape: vec![1, 1],
        datatype: KServeDatatype::BYTES,
        parameters: Some(KServeBinaryInferenceParameters {
            binary_data_size: Some(prompt.len()),
            ..Default::default()
        }),
    };

    let output = vec![KServeRequestOutput {
        name: String::from("generated_image"),
        parameters: KServeBinaryInferenceParameters {
            binary_data: Some(true),
            ..Default::default()
        },
    }];
    let inference_request = KServeBinaryInferenceRequest {
        model_name: "pipeline".to_string(),
        inputs: vec![input],
        outputs: output,
    };
    let tensors = vec![prompt.as_bytes().to_vec()];
    let result = kserve_client
        .binary_inference_request(&String::from("pipeline"), &inference_request, &tensors)
        .await
        .expect("Unable to get inference request.");

    println!("{:?}", result);
}

#[tokio::test]
async fn test_inference() {
    println!("Attempting to retrieve server metadata from Triton.");
    let mut kserve_client = KServeClient::new(String::from("http://localhost:8000")).await;
    let prompt = String::from("captain america, 4k");
    let input = vec![KServeTensor {
        metadata: KServeTensorMetadata {
            name: String::from("prompt"),
            shape: vec![1, 1],
            datatype: KServeDatatype::BYTES,
            parameters: Option::None,
        },
        data: vec![KServeTensorElement::String(prompt)],
    }];

    let output = vec![KServeRequestOutput {
        name: String::from("generated_image"),
        parameters: KServeBinaryInferenceParameters::default(),
    }];
    let inference_request = KServeInferenceRequest {
        id: None,
        parameters: None,
        inputs: &input,
        outputs: &output,
    };

    let result = kserve_client
        .inference_request(&String::from("pipeline"), &inference_request)
        .await
        .expect("Unable to get inference request.");
    result.outputs[0].
    println!("{:?}", result);
}

#[tokio::test]
async fn test_get_server_metadata() {
    eprintln!("Attempting to retrieve server metadata from Triton.");
    let mut kserve_client = KServeClient::new(String::from("http://localhost:8000")).await;

    let server_metadata = kserve_client.get_server_metadata().await;

    println!("{:?}", server_metadata);
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

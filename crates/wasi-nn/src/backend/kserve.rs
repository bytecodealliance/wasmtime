//! Implements the wasi-nn API.

use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Error, ErrorKind, Read};
use std::sync::Arc;

use byteorder::{LittleEndian, ReadBytesExt};
use http_body_util::{BodyExt, Full};
use hyper::body::Buf;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::SendRequest;
use hyper::header::HeaderName;
use hyper::http::uri::Authority;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use wiggle::async_trait_crate::async_trait;

use crate::backend::{
    BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner,
};
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor, TensorType};
use crate::{ExecutionContext, Graph, GraphRegistry};

#[allow(dead_code)]
const INFERENCE_HEADER_CONTENT_LENGTH: &str = "inference-header-content-length";
#[allow(dead_code)]
const BINARY_DATA_SIZE: &str = "binary_data_size";

pub(crate) struct KServeBackend {
    pub server_url: String,
    pub registry: HashMap<String, Graph>,
    // client: Mutex<KServeClient> //TODO: Replace with a client pool.
}

impl Default for KServeBackend {
    fn default() -> Self {
        let server_url = String::from("http://localhost:8000");
        Self {
            // client: Mutex::new(KServeClient::new(&server_url)),
            server_url,
            registry: HashMap::new(),
        }
    }
}

impl BackendInner for KServeBackend {
    fn encoding(&self) -> GraphEncoding {
        GraphEncoding::Autodetect
    }

    fn load(
        &mut self,
        _builders: &[&[u8]],
        _target: ExecutionTarget,
    ) -> Result<Graph, BackendError> {
        return Err(BackendError::UnsupportedOperation("load"));
    }

    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir> {
        None
    }
}

struct KServeGraph {
    model_name: String,
    server_url: String,
}

#[async_trait]
impl BackendGraph for KServeGraph {
    async fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let client = KServeClient::new(&self.server_url).await;
        Ok(ExecutionContext(Box::new(
            KServeExecutionContext::new(client, &self.model_name).await?,
        )))
        // return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }
}

struct KServeExecutionContext {
    client: Box<KServeClient>,
    input_mapping: HashMap<usize, String>,
    model_metadata: KServeModelMetadata,
    inputs: Vec<KServeTensor>,
    outputs: Vec<KServeTensor>,
}

impl KServeExecutionContext {
    async fn new(client: KServeClient, model_name: &String) -> Result<Self, BackendError> {
        let mut client = client;
        let model_metadata = client.get_model_metadata(model_name).await?;
        let mut input_list: Vec<&String> =
            model_metadata.inputs.iter().map(|tm| &tm.name).collect();
        let mut input_mapping: HashMap<usize, String> = HashMap::new();
        input_list.sort();

        for i in 0..input_list.len() {
            input_mapping.insert(i, input_list[i].clone());
        }

        Ok(Self {
            client: Box::new(client),
            input_mapping: input_mapping,
            model_metadata,
            inputs: Vec::new(),
            outputs: Vec::new(),
        })
    }
}

#[async_trait]
impl BackendExecutionContext for KServeExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor) -> Result<(), BackendError> {
        let datatype = map_tensor_type_to_datatype(tensor.tensor_type);
        let data = read_tensor_elements(tensor);
        let shape: Vec<i64> = tensor.dimensions.iter().map(|e| *e as i64).collect();

        let kserve_tensor = KServeTensor {
            metadata: KServeTensorMetadata {
                name: self.input_mapping[&(index as usize)].clone(),
                shape: shape,
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
        let outputs = self
            .model_metadata
            .outputs
            .iter()
            .map(|mm| KServeRequestOutput {
                name: mm.name.clone(),
                parameters: KServeBinaryInferenceParameters {
                    binary_data: None,
                    binary_data_output: None,
                    binary_data_size: None,
                },
            })
            .collect();

        let inference_request = KServeInferenceRequest {
            id: None,
            parameters: None,
            inputs: &self.inputs,
            outputs: &outputs,
        };
        let mut result = self
            .client
            .inference_request(&self.model_metadata.name, &inference_request)
            .await?;
        self.outputs.append(&mut result.outputs);
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let output = &self.outputs[index as usize];
        let total_size = output.data.len()
            * map_tensor_type_to_size(&map_datatype_to_tensor_type(&output.metadata.datatype));
        if total_size > destination.len() {
            return Err(BackendError::NotEnoughMemory(total_size));
        }
        let mut start_index = 0;
        for elem in &output.data {
            match elem {
                KServeTensorElement::Bool(b) => copy_to_destination(
                    destination,
                    &mut start_index,
                    if *b { &[1u8] } else { &[0u8] }.as_slice(),
                ),
                KServeTensorElement::Number(n) => match output.metadata.datatype {
                    KServeDatatype::UINT8 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_u64().unwrap() as u8).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::UINT16 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_u64().unwrap() as u16).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::UINT32 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_u64().unwrap() as u32).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::UINT64 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_u64().unwrap()).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::INT8 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_i64().unwrap() as i8).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::INT16 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_i64().unwrap() as i16).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::INT32 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_i64().unwrap() as i32).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::INT64 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_i64().unwrap() as i64).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::FP32 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_f64().unwrap() as f32).to_le_bytes().as_slice(),
                    ),
                    KServeDatatype::FP64 => copy_to_destination(
                        destination,
                        &mut start_index,
                        (n.as_f64().unwrap()).to_le_bytes().as_slice(),
                    ),
                    _ => panic!("Unsupported kserve datatype for rust"),
                },
                KServeTensorElement::String(s) => {
                    copy_to_destination(destination, &mut start_index, s.as_bytes())
                }
            };
        }
        Ok(total_size as u32)
    }
}

#[async_trait]
impl GraphRegistry for KServeBackend {
    async fn get_mut(&mut self, name: &str) -> Result<Option<&mut Graph>, BackendError> {
        let graph_present = self.registry.contains_key(name);
        let model_name = name.to_string();

        //We don't already have the graph retrieve the model metadata to make sure it exists
        if !graph_present {
            let mut client = KServeClient::new(&self.server_url).await;
            let model_metadata = client.get_model_metadata(&name.to_string()).await?;

            let g = Arc::new(KServeGraph {
                model_name: name.to_string(),
                server_url: self.server_url.clone(),
            });
            let graph: Graph = Graph(g);
            self.registry.insert(model_metadata.name.clone(), graph);
        }

        Ok(self.registry.get_mut(&model_name))

        //This has to be here, because you can't do a mutable borrow twice.
    }
}

fn copy_to_destination(destination: &mut [u8], start_index: &mut usize, src: &[u8]) {
    let end_index = *start_index + src.len();

    destination[*start_index..end_index].copy_from_slice(src);
    *start_index = end_index;
}

fn read_tensor_elements(tensor: &Tensor) -> Vec<KServeTensorElement> {
    let mut cursor = Cursor::new(tensor.data.as_slice().to_vec());

    let mut data = match tensor.tensor_type {
        TensorType::U8 => Vec::with_capacity(1),
        _ => Vec::with_capacity(tensor.data.len() / map_tensor_type_to_size(&tensor.tensor_type)),
    };

    let parse_bytes_as_string = tensor.dimensions[0] == 0;

    let expected_size = if parse_bytes_as_string {
        //Skip the first dimension if it is 0
        tensor
            .dimensions
            .iter()
            .skip(1)
            .fold(1u32, |acc, d| acc * d) as usize
    } else {
        tensor.dimensions.iter().fold(1u32, |acc, d| acc * d) as usize
    };

    //TODO: We've determined alignment is not an issue so in theory as long as there are enough
    //bytes passed in the tensor to match the expected dimensions it should be safe to just
    //do an unsafe Vec::from_raw_parts.
    while cursor.has_remaining() {
        data.push(match tensor.tensor_type {
            TensorType::Bf16 => panic!("bf16 is not supported for kserve backend."),
            TensorType::Fp16 => panic!("bf16 is not supported for kserve backend."),
            TensorType::I32 => KServeTensorElement::Number(Number::from(
                cursor.read_i32::<LittleEndian>().unwrap(),
            )),
            TensorType::I64 => KServeTensorElement::Number(Number::from(
                cursor.read_i64::<LittleEndian>().unwrap(),
            )),
            TensorType::Fp32 => KServeTensorElement::Number(
                Number::from_f64(cursor.read_f32::<LittleEndian>().unwrap() as f64).unwrap(),
            ),
            TensorType::Fp64 => KServeTensorElement::Number(
                Number::from_f64(cursor.read_f64::<LittleEndian>().unwrap()).unwrap(),
            ),
            TensorType::U8 => {
                if parse_bytes_as_string {
                    let mut s: String = String::new();
                    let strlen = cursor
                        .read_to_string(&mut s)
                        .expect("Unable to read string from tensor");
                    assert!(tensor.dimensions[1] == 1 || (tensor.dimensions[1] as usize) == strlen);
                    KServeTensorElement::String(s)
                } else {
                    KServeTensorElement::Number(Number::from(cursor.read_u8().unwrap()))
                }
            }
        });
    }

    if parse_bytes_as_string {
        //It can either be 1 or the length of data, if it was specified in the tensor.
        assert!(expected_size == 1 || expected_size == data.len())
    } else {
        assert_eq!(expected_size, data.len());
    }
    data
}

fn map_tensor_type_to_size(tensor_type: &TensorType) -> usize {
    match tensor_type {
        TensorType::U8 => 1,
        TensorType::Bf16 => 2,
        TensorType::Fp16 => 2,
        TensorType::I32 => 4,
        TensorType::Fp32 => 4,
        TensorType::Fp64 => 8,
        TensorType::I64 => 8,
    }
}

fn map_datatype_to_tensor_type(datatype: &KServeDatatype) -> TensorType {
    match datatype {
        KServeDatatype::UINT8 => TensorType::U8,
        KServeDatatype::INT32 => TensorType::I32,
        KServeDatatype::FP16 => TensorType::Fp16,
        KServeDatatype::FP32 => TensorType::Fp32,
        KServeDatatype::BF16 => TensorType::Bf16,
        _ => panic!("Unsupported operation."),
    }
}

fn map_tensor_type_to_datatype(tensor_type: TensorType) -> KServeDatatype {
    match tensor_type {
        TensorType::U8 => KServeDatatype::BYTES,
        TensorType::Bf16 => KServeDatatype::BF16,
        TensorType::Fp16 => KServeDatatype::FP16,
        TensorType::I32 => KServeDatatype::INT32,
        TensorType::Fp32 => KServeDatatype::FP32,
        TensorType::I64 => KServeDatatype::INT64,
        TensorType::Fp64 => KServeDatatype::FP64,
    }
}

pub struct KServeClient {
    server_url: String,
    #[allow(dead_code)]
    url: Uri,
    #[allow(dead_code)]
    task: JoinHandle<()>,
    authority: Authority,
    sender: SendRequest<Full<Bytes>>,
}

impl KServeClient {
    /// Creates a new KServe client.
    /// TODO: Add support for HTTPS connections.
    pub async fn new(server_url: &String) -> Self {
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
        let (sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .expect("Unable to perform http handshake with server.");
        // Spawn a task to poll the connection, driving the HTTP state
        let task = tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        Self {
            server_url: server_url.clone(),
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
        let res = self
            .sender
            .send_request(request)
            .await
            .expect("Unable to send HTTP request to server.");

        println!("Response status: {}", res.status());
        res
    }

    #[allow(dead_code)]
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

        let res = self.send_request(req).await;

        println!("Response status: {:?}", res.status());

        if res.status() == StatusCode::OK {
            try_deserialize(res).await
        } else {
            Err(BackendError::BackendAccess(anyhow::Error::from(
                Error::new(ErrorKind::Other, "Unable to retrieve model metadata."),
            )))
        }
    }

    #[allow(dead_code)]
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
        println!("req body: {:?}", String::from_utf8(json_bytes.clone()));
        let req = Request::builder()
            .uri(inference_url)
            .method(Method::POST)
            .header(hyper::header::HOST, self.authority.as_str())
            .body(Full::<Bytes>::from(json_bytes))
            .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;

        // Await the response...
        let res = self.send_request(req).await;
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
        let res = self.send_request(req).await;
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

    #[allow(dead_code)]
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
        let res = self.send_request(req).await;
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
pub struct KServeServerMetadata {
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
    shape: Vec<i64>,
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
    model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
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

#[allow(dead_code)]
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
    // let body = response
    //     .collect()
    //     .await
    //     .map(|collected| collected.aggregate())
    //     .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))?;
    let body = response.collect().await.unwrap().to_bytes().to_vec();
    println!(
        "Body: {:?}",
        String::from_utf8(body[..min(body.len(), 256)].to_vec())
    );
    serde_json::from_slice(body.as_slice()) //from_reader(body.reader())
        .map_err(|e| BackendError::BackendAccess(anyhow::Error::from(e)))
}

#[allow(dead_code)]
fn build_server_metadata_url(server_url: &String) -> String {
    format!("{}/v2", server_url)
}

fn build_model_metadata_url(server_url: &String, model_name: &String) -> String {
    format!("{}/v2/models/{}", server_url, model_name)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn inference_content_length_header() -> HeaderName {
    hyper::header::HeaderName::from_static(INFERENCE_HEADER_CONTENT_LENGTH)
}

#[tokio::test]
#[ignore]
async fn test_binary_inference() {
    let mut kserve_client = KServeClient::new(&String::from("http://localhost:8000")).await;
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
#[ignore]
async fn test_inference() {
    println!("Attempting to retrieve server metadata from Triton.");
    let mut kserve_client = KServeClient::new(&String::from("http://localhost:8000")).await;
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

    println!("{:?}", result);
}

#[tokio::test]
#[ignore]
async fn test_get_server_metadata() {
    eprintln!("Attempting to retrieve server metadata from Triton.");
    let mut kserve_client = KServeClient::new(&String::from("http://localhost:8000")).await;

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
    println!("{:?}", output)
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
    println!("{:?}", output)
}

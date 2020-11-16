use std::convert::TryInto;
use std::fs;
use wasi_nn;

pub fn main() {
    let xml = fs::read_to_string("fixture/frozen_inference_graph.xml").unwrap();
    println!("First 50 characters of graph: {}", &xml[..50]);

    let weights = fs::read("fixture/frozen_inference_graph.bin").unwrap();
    println!("Size of weights: {}", weights.len());

    let graph = unsafe {
        wasi_nn::load(
            &[&xml.into_bytes(), &weights],
            wasi_nn::GRAPH_ENCODING_OPENVINO,
            wasi_nn::EXECUTION_TARGET_CPU,
        )
        .unwrap()
    };
    println!("Graph handle ID: {}", graph);

    let context = unsafe { wasi_nn::init_execution_context(graph).unwrap() };
    println!("Execution context ID: {}", context);

    // Load a tensor that precisely matches the graph input tensor (see
    // `fixture/frozen_inference_graph.xml`).
    let tensor_data = fs::read("fixture/tensor-1x3x300x300-f32.bgr").unwrap();
    println!("Tensor bytes: {}", tensor_data.len());
    let tensor = wasi_nn::Tensor {
        dimensions: &[1, 3, 300, 300],
        r#type: wasi_nn::TENSOR_TYPE_F32,
        data: &tensor_data,
    };
    unsafe {
        wasi_nn::set_input(context, 0, tensor).unwrap();
    }

    // Execute the inference.
    unsafe {
        wasi_nn::compute(context).unwrap();
    }

    // Retrieve the output (TODO output looks incorrect).
    let mut output_buffer = vec![0f32; 1 << 20];
    unsafe {
        wasi_nn::get_output(
            context,
            0,
            &mut output_buffer[..] as *mut [f32] as *mut u8,
            (output_buffer.len() * 4).try_into().unwrap(),
        );
    }
    println!("output tensor: {:?}", &output_buffer[..1000])
}

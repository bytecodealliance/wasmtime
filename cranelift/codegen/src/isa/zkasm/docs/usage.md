
# Compiling `.wat` to `.zkasm` using Cranelift

In this guide, you'll learn how to use `cranelift` to compile `.wat` files into `.zkasm` files.

## Getting Started

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/near/wasmtime
   ```

2. **Update Submodules**:
   ```bash
   git submodule update --init --recursive
   ```

## Compilation Process

To compile a `.wat` file to `.zkasm`:

1. Write your `.wat` code to a file in the `cranelift/zkasm_data` directory.

2. Add the name of your file to the `testcases!` macro in `cranelift/filetests/src/test_zkasm.rs`. For instance:

   ```rust
   testcases! {
       add,
       locals,
       locals_simple,
       counter,
       fibonacci,
       add_func,
       // Add your file name here
   }
   ```

3. Execute the following command to compile your file to `.zkasm`:

   ```bash
   env UPDATE_EXPECT=1 cargo test --package cranelift-filetests --lib -- test_zkasm::tests::<filename> --exact --nocapture
   ```

   For example:

   ```bash
   env UPDATE_EXPECT=1 cargo test --package cranelift-filetests --lib -- test_zkasm::tests::add --exact --nocapture
   ```

   The result of the compilation will be stored in `cranelift/zkasm_data/generated`. Without setting `env UPDATE_EXPECT=1`, it will assert that the generated code matches the code in the `.zkasm` file.

## Testing `.zkasm` Files

To test a `.zkasm` file:

1. Clone the `zkevm-rom` repository in the same directory as `wasmtime`:

   ```bash
   git clone https://github.com/0xPolygonHermez/zkevm-rom
   ```

2. Install necessary packages in the `zkevm-rom` directory:

   ```bash
   npm install
   ```

3. Execute the following from the `wasmtime/` directory to test the zk-asm:

   ```bash
   ./ci/test-zkasm.sh --all
   ```

   Or, for a specific file:

   ```bash
   ./ci/test-zkasm.sh <filename>
   ```

   For example:

   ```bash
   ./ci/test-zkasm.sh zkasm_data/generated/add.zkasm
   ```

## Logging during Compilation

If you wish to compile a `.wat` file with logging (without generating a `.zkasm` file), you can use the following command:

   ```bash
   RUST_LOG=trace cargo run --features=all-arch -p -D cranelift-tools --bin=clif-util wasm --target=zkasm <filepath>
   ```

   For example:

   ```bash
   RUST_LOG=trace cargo run --features=all-arch -p cranelift-tools --bin=clif-util wasm --target=zkasm cranelift/zkasm_data/add.wat 2>trace.txt
   ```

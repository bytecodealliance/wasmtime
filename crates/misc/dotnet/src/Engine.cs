using System;
using System.Text;
using System.Runtime.InteropServices;

namespace Wasmtime
{
    /// <summary>
    /// Represents the Wasmtime engine.
    /// </summary>
    public class Engine : IDisposable
    {
        /// <summary>
        /// Constructs a new <see cref="Engine" />.
        /// </summary>
        public Engine()
        {
            Handle = Interop.wasm_engine_new();

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime engine.");
            }
        }

        internal Engine(Interop.WasmConfigHandle config)
        {
            Handle = Interop.wasm_engine_new_with_config(config);
            config.SetHandleAsInvalid();

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime engine.");
            }
        }

        /// <summary>
        /// Creates a new Wasmtime <see cref="Store" />.
        /// </summary>
        /// <returns>Returns the new <see cref="Store" />.</returns>
        public Store CreateStore()
        {
            return new Store(this);
        }

        /// <summary>
        /// Converts the WebAssembly text format to the binary format
        /// </summary>
        /// <returns>Returns the binary-encoded wasm module.</returns>
        public byte[] WatToWasm(string wat)
        {
            var watBytes = Encoding.UTF8.GetBytes(wat);
            unsafe
            {
                fixed (byte *ptr = watBytes)
                {
                    Interop.wasm_byte_vec_t watByteVec;
                    watByteVec.size = (UIntPtr)watBytes.Length;
                    watByteVec.data = ptr;
                    if (!Interop.wasmtime_wat2wasm(Handle, ref watByteVec, out var bytes, out var error)) {
                        var errorSpan = new ReadOnlySpan<byte>(error.data, checked((int)error.size));
                        var message = Encoding.UTF8.GetString(errorSpan);
                        Interop.wasm_byte_vec_delete(ref error);
                        throw new WasmtimeException("failed to parse input wat: " + message);
                    }
                    var byteSpan = new ReadOnlySpan<byte>(bytes.data, checked((int)bytes.size));
                    var ret = byteSpan.ToArray();
                    Interop.wasm_byte_vec_delete(ref bytes);
                    return ret;
                }
            }
        }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
        }

        internal Interop.EngineHandle Handle { get; private set; }
    }
}

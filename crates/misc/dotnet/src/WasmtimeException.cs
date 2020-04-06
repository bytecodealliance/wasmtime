using System;
using System.Runtime.Serialization;
using System.Text;

namespace Wasmtime
{
    /// <summary>
    /// The base type for Wasmtime exceptions.
    /// </summary>
    [System.Serializable]
    public class WasmtimeException : Exception
    {
        /// <inheritdoc/>
        public WasmtimeException() { }

        /// <inheritdoc/>
        public WasmtimeException(string message) : base(message) { }

        /// <inheritdoc/>
        public WasmtimeException(string message, Exception inner) : base(message, inner) { }

        /// <inheritdoc/>
        protected WasmtimeException(SerializationInfo info, StreamingContext context) : base(info, context) { }

        internal static WasmtimeException FromOwnedError(IntPtr error)
        {
            unsafe
            {
                Interop.wasmtime_error_message(error, out var bytes);
                var byteSpan = new ReadOnlySpan<byte>(bytes.data, checked((int)bytes.size));
                var message = Encoding.UTF8.GetString(byteSpan);
                Interop.wasm_byte_vec_delete(ref bytes);

                Interop.wasmtime_error_delete(error);

                return new WasmtimeException(message);
            }
        }
    }
}

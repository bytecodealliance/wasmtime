using System;
using System.Runtime.Serialization;

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
    }
}

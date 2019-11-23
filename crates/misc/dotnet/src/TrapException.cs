using System;
using System.Runtime.InteropServices;
using System.Runtime.Serialization;

namespace Wasmtime
{
    /// <summary>
    /// The exception for WebAssembly traps.
    /// </summary>
    [Serializable]
    public class TrapException : WasmtimeException
    {
        /// <inheritdoc/>
        public TrapException() { }

        /// <inheritdoc/>
        public TrapException(string message) : base(message) { }

        /// <inheritdoc/>
        public TrapException(string message, Exception inner) : base(message, inner) { }

        /// <inheritdoc/>
        protected TrapException(SerializationInfo info, StreamingContext context) : base(info, context) { }

        internal static TrapException FromOwnedTrap(IntPtr trap)
        {
            unsafe
            {
                Interop.wasm_trap_message(trap, out var bytes);
                var message = Marshal.PtrToStringUTF8((IntPtr)bytes.data, (int)bytes.size - 1 /* remove null */);
                Interop.wasm_byte_vec_delete(ref bytes);

                Interop.wasm_trap_delete(trap);

                return new TrapException(message);
            }
        }

        // TODO: expose trap frames
    }
}

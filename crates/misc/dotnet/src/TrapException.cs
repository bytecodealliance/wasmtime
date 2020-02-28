using System;
using System.Runtime.Serialization;
using System.Text;

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
                var byteSpan = new ReadOnlySpan<byte>(bytes.data, checked((int)bytes.size));

                int indexOfNull = byteSpan.LastIndexOf((byte)0);
                if (indexOfNull != -1)
                {
                    byteSpan = byteSpan.Slice(0, indexOfNull);
                }

                var message = Encoding.UTF8.GetString(byteSpan);
                Interop.wasm_byte_vec_delete(ref bytes);

                Interop.wasm_trap_delete(trap);

                return new TrapException(message);
            }
        }

        // TODO: expose trap frames
    }
}

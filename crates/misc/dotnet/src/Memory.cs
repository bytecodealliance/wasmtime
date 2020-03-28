using System;
using System.Text;
using System.Buffers.Binary;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WebAssembly memory.
    /// </summary>
    public class Memory : MemoryBase, IDisposable
    {
        /// <summary>
        /// The size, in bytes, of a WebAssembly memory page.
        /// </summary>
        public const int PageSize = 65536;

        /// <summary>
        /// The minimum memory size (in WebAssembly page units).
        /// </summary>
        public uint Minimum { get; private set; }

        /// <summary>
        /// The maximum memory size (in WebAssembly page units).
        /// </summary>
        public uint Maximum { get; private set; }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
        }

        internal Memory(Interop.StoreHandle store, uint minimum = 1, uint maximum = uint.MaxValue)
        {
            if (minimum == 0)
            {
                throw new ArgumentException("The minimum cannot be zero.", nameof(minimum));
            }

            if (maximum < minimum)
            {
                throw new ArgumentException("The maximum cannot be less than the minimum.", nameof(maximum));
            }

            Minimum = minimum;
            Maximum = maximum;

            unsafe
            {
                Interop.wasm_limits_t limits = new Interop.wasm_limits_t();
                limits.min = minimum;
                limits.max = maximum;

                using var memoryType = Interop.wasm_memorytype_new(&limits);
                Handle = Interop.wasm_memory_new(store, memoryType);

                if (Handle.IsInvalid)
                {
                    throw new WasmtimeException("Failed to create Wasmtime memory.");
                }
            }
        }

        protected override IntPtr MemoryHandle => Handle.DangerousGetHandle();

        internal Interop.MemoryHandle Handle { get; private set; }
    }
}

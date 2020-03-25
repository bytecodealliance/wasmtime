using System;
using System.Text;

namespace Wasmtime
{
    /// <summary>
    /// Represents an exported memory of a host function caller.
    /// </summary>
    public class CallerMemory : MemoryBase, IDisposable
    {
        /// <inheritdoc/>
        public void Dispose()
        {
            if (!_extern.IsInvalid)
            {
                _extern.Dispose();
                _extern.SetHandleAsInvalid();
            }
        }

        internal CallerMemory(Interop.ExternHandle ext, IntPtr memory)
        {
            _extern = ext;
            _memory = memory;
        }

        /// <inheritdoc/>
        protected override IntPtr MemoryHandle => _memory;

        private Interop.ExternHandle _extern;

        private IntPtr _memory;
    }

    /// <summary>
    /// Represents information of the caller of a host function.
    /// </summary>
    public class Caller
    {
        /// <summary>
        /// Gets an exported memory of the caller by the given name.
        /// </summary>
        /// <param name="name">The name of the exported memory.</param>
        /// <returns>Returns the exported memory if found or null if a memory of the requested name is not exported.</returns>
        public CallerMemory GetMemory(string name)
        {
            if (Handle == IntPtr.Zero)
            {
                throw new InvalidOperationException();
            }

            unsafe
            {
                var bytes = Encoding.UTF8.GetBytes(name);

                fixed (byte* ptr = bytes)
                {
                    Interop.wasm_byte_vec_t nameVec = new Interop.wasm_byte_vec_t();
                    nameVec.size = (UIntPtr)bytes.Length;
                    nameVec.data = ptr;

                    var export = Interop.wasmtime_caller_export_get(Handle, ref nameVec);
                    if (export.IsInvalid)
                    {
                        return null;
                    }

                    var memory = Interop.wasm_extern_as_memory(export.DangerousGetHandle());
                    if (memory == IntPtr.Zero)
                    {
                        export.Dispose();
                        return null;
                    }

                    return new CallerMemory(export, memory);
                }
            }
        }

        internal IntPtr Handle { get; set; }
    }
}
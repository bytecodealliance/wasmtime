using System;
using System.Runtime.InteropServices;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WebAssembly module.
    /// </summary>
    public class Module : IDisposable
    {
        /// <summary>
        /// The name of the module.
        /// </summary>
        public string Name { get; private set; }

        /// <summary>
        /// The imports of the module.
        /// </summary>
        public Wasmtime.Imports.Imports Imports { get; private set; }

        /// <summary>
        /// The exports of the module.
        /// </summary>
        /// <value></value>
        public Wasmtime.Exports.Exports Exports { get; private set; }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
            if (!(Imports is null))
            {
                Imports.Dispose();
                Imports = null;
            }
        }

        internal Module(Interop.StoreHandle store, string name, byte[] bytes)
        {
            unsafe
            {
                fixed (byte *ptr = bytes)
                {
                    Interop.wasm_byte_vec_t vec;
                    vec.size = (UIntPtr)bytes.Length;
                    vec.data = ptr;

                    Handle = Interop.wasm_module_new(store, ref vec);
                }

                if (Handle.IsInvalid)
                {
                    throw new WasmtimeException($"WebAssembly module '{name}' is not valid.");
                }
            }

            Name = name;
            Imports = new Wasmtime.Imports.Imports(this);
            Exports = new Wasmtime.Exports.Exports(this);
        }

        internal Interop.ModuleHandle Handle { get; private set; }
    }
}

using System;
using System.Runtime.InteropServices;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WebAssembly module.
    /// </summary>
    public class Module : IDisposable
    {
        internal Module(Store store, string name, byte[] bytes)
        {
            if (store.Handle.IsInvalid)
            {
                throw new ArgumentNullException(nameof(store));
            }

            unsafe
            {
                fixed (byte *ptr = bytes)
                {
                    Interop.wasm_byte_vec_t vec;
                    vec.size = (UIntPtr)bytes.Length;
                    vec.data = ptr;

                    Handle = Interop.wasm_module_new(store.Handle, ref vec);
                }

                if (Handle.IsInvalid)
                {
                    throw new WasmtimeException($"WebAssembly module '{name}' is not valid.");
                }
            }

            Store = store;
            Name = name;
            Imports = new Wasmtime.Imports.Imports(this);
            Exports = new Wasmtime.Exports.Exports(this);
        }

        /// <summary>
        /// Instantiates a WebAssembly module for the given host.
        /// </summary>
        /// <param name="host">The host to use for the WebAssembly module's instance.</param>
        /// <returns>Returns a new <see cref="Instance" />.</returns>
        public Instance Instantiate(IHost host = null)
        {
            return Instantiate(null, host);
        }

        /// <summary>
        /// Instantiates a WebAssembly module for the given host.
        /// </summary>
        /// <param name="wasi">The WASI instance to use for WASI imports.</param>
        /// <param name="host">The host to use for the WebAssembly module's instance.</param>
        /// <returns>Returns a new <see cref="Instance" />.</returns>
        public Instance Instantiate(Wasi wasi, IHost host = null)
        {
            if (!(host?.Instance is null))
            {
                throw new InvalidOperationException("The host has already been associated with an instantiated module.");
            }

            var instance = new Instance(this, wasi, host);

            if (!(host is null))
            {
                host.Instance = instance;
                return instance;
            }

            return instance;
        }

        /// <summary>
        /// The <see cref="Store"/> associated with the module.
        /// </summary>
        public Store Store { get; private set; }

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

        internal Interop.ModuleHandle Handle { get; private set; }
    }
}

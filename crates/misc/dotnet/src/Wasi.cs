using System;
using Wasmtime.Bindings;
using Wasmtime.Imports;

namespace Wasmtime
{
    public class Wasi
    {
        /// <summary>
        /// Creates a default <see cref="Wasi"/> instance.
        /// </summary>
        /// <param name="store">The store to use for the new WASI instance.</param>
        /// <param name="name">The name of the WASI module to create.</param>
        public Wasi(Store store, string name) :
            this(
                (store ?? throw new ArgumentNullException(nameof(store))).Handle,
                Interop.wasi_config_new(),
                name
            )
        {
        }

        internal Wasi(Interop.StoreHandle store, Interop.WasiConfigHandle config, string name)
        {
            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentException("Name cannot be null or empty.", nameof(name));
            }

            IntPtr trap;
            Handle = Interop.wasi_instance_new(store, name, config, out trap);
            config.SetHandleAsInvalid();

            if (trap != IntPtr.Zero)
            {
                throw TrapException.FromOwnedTrap(trap);
            }
        }

        internal WasiBinding Bind(Import import)
        {
            var export = Interop.wasi_instance_bind_import(Handle, import.Handle);
            if (export == IntPtr.Zero)
            {
                return null;
            }
            return new WasiBinding(export);
        }

        internal Interop.WasiInstanceHandle Handle { get; private set; }
    }   
}

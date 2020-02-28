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
        public Wasi(Store store) :
            this(
                (store ?? throw new ArgumentNullException(nameof(store))).Handle,
                Interop.wasi_config_new()
            )
        {
        }

        internal Wasi(Interop.StoreHandle store, Interop.WasiConfigHandle config)
        {
            IntPtr trap;
            Handle = Interop.wasi_instance_new(store, config, out trap);
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

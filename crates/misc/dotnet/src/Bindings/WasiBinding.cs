using System;
using System.Runtime.InteropServices;

namespace Wasmtime.Bindings
{
    /// <summary>
    /// Represents a binding to a WASI export.
    /// </summary>
    internal class WasiBinding : Binding
    {
        public WasiBinding(IntPtr handle)
        {
            _handle = handle;
        }

        public override SafeHandle Bind(Store store, IHost host)
        {
            return new Interop.WasiExportHandle(_handle);
        }

        private IntPtr _handle;
    }
}

using System;
using System.Collections.Generic;
using Wasmtime.Bindings;

namespace Wasmtime
{
    /// <summary>
    /// The interface implemented by Wasmtime hosts.
    /// </summary>
    public interface IHost
    {
        /// <summary>
        /// The <see cref="Wasmtime.Instance" /> that the host is bound to.
        /// </summary>
        /// <remarks>A host can only bind to one module instance at a time.</remarks>
        Instance Instance { get; set; }

        /// <summary>
        /// Gets the import bindings of the host given a WebAssembly module.
        /// </summary>
        /// <param name="module">The WebAssembly module to get the import bindings for.</param>
        /// <returns>Returns the list of import bindings for the host.</returns>
        List<Binding> GetImportBindings(Module module) => Binding.GetImportBindings(this, module);
    }
}

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
    }
}

using System;

namespace Wasmtime
{
    /// <summary>
    /// Used to mark .NET methods and fields as imports to a WebAssembly module.
    /// </summary>
    [AttributeUsage(AttributeTargets.Method | AttributeTargets.Field)]
    public class ImportAttribute : Attribute
    {
        /// <summary>
        /// Constructs a new <see cref="ImportAttribute"/>.
        /// </summary>
        /// <param name="name">The name of the import.</param>
        public ImportAttribute(string name)
        {
            Name = name;
        }

        /// <summary>
        /// The name of the import.
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// The module name of the import.
        /// </summary>
        /// <remarks>A null or empty module name implies that the import is not scoped to a module.</remarks>
        public string Module { get; set; }
    }
}

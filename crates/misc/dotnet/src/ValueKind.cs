using System;

namespace Wasmtime
{
    /// <summary>
    /// Represents the possible kinds of WebAssembly values.
    /// </summary>
    public enum ValueKind : byte
    {
        /// <summary>
        /// The value is a 32-bit integer.
        /// </summary>
        Int32,
        /// <summary>
        /// The value is a 64-bit integer.
        /// </summary>
        Int64,
        /// <summary>
        /// The value is a 32-bit floating point number.
        /// </summary>
        Float32,
        /// <summary>
        /// The value is a 64-bit floating point number.
        /// </summary>
        Float64,
        /// <summary>
        /// The value is a reference.
        /// </summary>
        AnyRef = 128,
        /// <summary>
        /// The value is a function reference.
        /// </summary>
        FuncRef,
    }
}

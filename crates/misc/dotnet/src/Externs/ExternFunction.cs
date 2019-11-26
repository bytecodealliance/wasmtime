using System;
using System.Collections.Generic;
using Wasmtime.Exports;

namespace Wasmtime.Externs
{
    /// <summary>
    /// Represents an external (instantiated) WebAssembly function.
    /// </summary>
    public class ExternFunction
    {
        internal ExternFunction(FunctionExport export, IntPtr func)
        {
            _export = export;
            _func = func;
        }

        /// <summary>
        /// The name of the WebAssembly function.
        /// </summary>
        public string Name => _export.Name;

        /// <summary>
        /// The parameters of the WebAssembly function.
        /// </summary>
        public IReadOnlyList<ValueKind> Parameters => _export.Parameters;

        /// <summary>
        /// The results of the WebAssembly function.
        /// </summary>
        public IReadOnlyList<ValueKind> Results => _export.Results;

        /// <summary>
        /// Invokes the WebAssembly function.
        /// </summary>
        /// <param name="arguments">The array of arguments to pass to the function.</param>
        /// <returns>
        ///   Returns null if the function has no return value.
        ///   Returns the value if the function returns a single value.
        ///   Returns an array of values if the function returns more than one value.
        /// </returns>
        public object Invoke(params object[] arguments)
        {
            if (arguments.Length != Parameters.Count)
            {
                throw new WasmtimeException($"Argument mismatch when invoking function '{Name}': requires {Parameters.Count} but was given {arguments.Length}.");
            }

            unsafe
            {
                Interop.wasm_val_t* args = stackalloc Interop.wasm_val_t[Parameters.Count];
                Interop.wasm_val_t* results = stackalloc Interop.wasm_val_t[Results.Count];

                for (int i = 0; i < arguments.Length; ++i)
                {
                    args[i] = Interop.ToValue(arguments[i], Parameters[i]);
                }

                var trap = Interop.wasm_func_call(_func, args, results);
                if (trap != IntPtr.Zero)
                {
                    throw TrapException.FromOwnedTrap(trap);
                }

                if (Results.Count == 0)
                {
                    return null;
                }

                if (Results.Count == 1)
                {
                    return Interop.ToObject(&results[0]);
                }

                var ret = new object[Results.Count];
                for (int i = 0; i < Results.Count; ++i)
                {
                    ret[i] = Interop.ToObject(&results[i]);
                }
                return ret;
            }
        }

        private FunctionExport _export;
        private IntPtr _func;
    }
}

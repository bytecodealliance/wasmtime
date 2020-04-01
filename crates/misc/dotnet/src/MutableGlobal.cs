using System;

namespace Wasmtime
{
    /// <summary>
    /// Represents a mutable WebAssembly global value.
    /// </summary>
    public class MutableGlobal<T> : IDisposable
    {
        /// <summary>
        /// The value of the global.
        /// </summary>
        public T Value
        {
            get
            {
                if (Handle is null)
                {
                    throw new InvalidOperationException("The global cannot be used before it is instantiated.");
                }

                unsafe
                {
                    var v = stackalloc Interop.wasm_val_t[1];
                    Interop.wasm_global_get(Handle.DangerousGetHandle(), v);
                    return (T)Interop.ToObject(v);
                }
            }
            set
            {
                if (Handle is null)
                {
                    throw new InvalidOperationException("The global cannot be used before it is instantiated.");
                }

                var v = Interop.ToValue(value, Kind);

                unsafe
                {
                    Interop.wasm_global_set(Handle.DangerousGetHandle(), &v);
                }
            }
        }

        /// <summary>
        /// Gets the value kind of the global.
        /// </summary>
        /// <value></value>
        public ValueKind Kind { get; private set; }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
        }

        internal MutableGlobal(Interop.StoreHandle store, T initialValue)
        {
            if (!Interop.TryGetValueKind(typeof(T), out var kind))
            {
                throw new WasmtimeException($"Mutable global variables cannot be of type '{typeof(T).ToString()}'.");
            }

            Kind = kind;

            var value = Interop.ToValue((object)initialValue, Kind);

            var valueType = Interop.wasm_valtype_new(value.kind);
            var valueTypeHandle = valueType.DangerousGetHandle();
            valueType.SetHandleAsInvalid();

            using var globalType = Interop.wasm_globaltype_new(
                valueTypeHandle,
                Interop.wasm_mutability_t.WASM_VAR
            );

            unsafe
            {
                Handle = Interop.wasm_global_new(store, globalType, &value);

                if (Handle.IsInvalid)
                {
                    throw new WasmtimeException("Failed to create mutable Wasmtime global.");
                }
            }
        }

        internal Interop.GlobalHandle Handle { get; set; }
    }
}

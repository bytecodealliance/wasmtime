using System;

namespace Wasmtime
{
    /// <summary>
    /// Represents a mutable WebAssembly global value.
    /// </summary>
    public class MutableGlobal<T>
    {
        /// <summary>
        /// Creates a new <see cref="MutableGlobal&lt;T&gt;" /> with the given initial value.
        /// </summary>
        /// <param name="initialValue">The initial value of the global.</param>
        public MutableGlobal(T initialValue)
        {
            InitialValue = initialValue;
            Kind = Interop.ToValueKind(typeof(T));
        }

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

                    // TODO: figure out a way that doesn't box the value
                    return (T)Interop.ToObject(v);
                }
            }
            set
            {
                if (Handle is null)
                {
                    throw new InvalidOperationException("The global cannot be used before it is instantiated.");
                }

                // TODO: figure out a way that doesn't box the value
                var v = Interop.ToValue(value, Kind);

                unsafe
                {
                    Interop.wasm_global_set(Handle.DangerousGetHandle(), &v);
                }
            }
        }

        internal ValueKind Kind { get; private set; }

        internal Interop.GlobalHandle Handle { get; set; }

        internal T InitialValue { get; private set; }
    }
}

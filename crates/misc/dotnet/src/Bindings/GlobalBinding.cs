using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Wasmtime.Imports;

namespace Wasmtime.Bindings
{
    /// <summary>
    /// Represents a host global binding.
    /// </summary>
    public class GlobalBinding : Binding
    {
        /// <summary>
        /// Constructs a new global binding.
        /// </summary>
        /// <param name="import">The global import of the binding.</param>
        /// <param name="field">The field the import is bound to.</param>
        public GlobalBinding(GlobalImport import, FieldInfo field)
        {
            if (import is null)
            {
                throw new ArgumentNullException(nameof(import));
            }

            if (field is null)
            {
                throw new ArgumentNullException(nameof(field));
            }

            Import = import;
            Field = field;

            Validate();
        }

        /// <summary>
        /// The global import of the binding.
        /// </summary>
        public GlobalImport Import { get; private set; }

        /// <summary>
        /// The field the import is bound to.
        /// </summary>
        public FieldInfo Field { get; private set; }

        internal override SafeHandle Bind(Store store, IHost host)
        {
            unsafe
            {
                dynamic global = Field.GetValue(host);
                if (global.Handle != null)
                {
                    throw new InvalidOperationException("Cannot bind more than once.");
                }

                var v = Interop.ToValue((object)global.InitialValue, Import.Kind);

                var valueType = Interop.wasm_valtype_new(v.kind);
                var valueTypeHandle = valueType.DangerousGetHandle();
                valueType.SetHandleAsInvalid();

                using (var globalType = Interop.wasm_globaltype_new(
                    valueTypeHandle,
                    Import.IsMutable ? Interop.wasm_mutability_t.WASM_VAR : Interop.wasm_mutability_t.WASM_CONST))
                {
                    var handle = Interop.wasm_global_new(store.Handle, globalType, &v);
                    global.Handle = handle;
                    return handle;
                }
            }
        }

        private void Validate()
        {
            if (Field.IsStatic)
            {
                ThrowBindingException(Import, Field, "field cannot be static");
            }

            if (!Field.IsInitOnly)
            {
                ThrowBindingException(Import, Field, "field must be readonly");
            }

            if (!Field.FieldType.IsGenericType)
            {
                ThrowBindingException(Import, Field, "field is expected to be of type 'Global<T>'");
            }

            var definition = Field.FieldType.GetGenericTypeDefinition();
            if (definition == typeof(Global<>))
            {
                if (Import.IsMutable)
                {
                    ThrowBindingException(Import, Field, "the import is mutable (use the 'MutableGlobal' type)");
                }
            }
            else if (definition == typeof(MutableGlobal<>))
            {
                if (!Import.IsMutable)
                {
                    ThrowBindingException(Import, Field, "the import is constant (use the 'Global' type)");
                }
            }
            else
            {
                ThrowBindingException(Import, Field, "field is expected to be of type 'Global<T>' or 'MutableGlobal<T>'");
            }

            var arg = Field.FieldType.GetGenericArguments()[0];

            if (Interop.TryGetValueKind(arg, out var kind))
            {
                if (!Interop.IsMatchingKind(kind, Import.Kind))
                {
                    ThrowBindingException(Import, Field, $"global type argument is expected to be of type '{Interop.ToString(Import.Kind)}'");
                }
            }
            else
            {
                ThrowBindingException(Import, Field, $"'{arg}' is not a valid global type");
            }
        }
    }
}

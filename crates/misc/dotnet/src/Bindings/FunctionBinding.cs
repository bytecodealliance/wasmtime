using System;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using Wasmtime.Imports;

namespace Wasmtime.Bindings
{
    /// <summary>
    /// Represents a host function binding.
    /// </summary>
    public class FunctionBinding : Binding
    {
        /// <summary>
        /// Constructs a new function binding.
        /// </summary>
        /// <param name="import">The function import of the binding.</param>
        /// <param name="method">The method the import is bound to.</param>
        public FunctionBinding(FunctionImport import, MethodInfo method)
        {
            if (import is null)
            {
                throw new ArgumentNullException(nameof(import));
            }

            if (method is null)
            {
                throw new ArgumentNullException(nameof(method));
            }

            Import = import;
            Method = method;

            Validate();
        }

        /// <summary>
        /// The function import of the binding.
        /// </summary>
        public FunctionImport Import { get; private set; }

        /// <summary>
        /// The method the import is bound to.
        /// </summary>
        public MethodInfo Method { get; private set; }

        internal override SafeHandle Bind(Store store, IHost host)
        {
            unsafe
            {
                if (_callback != null)
                {
                    throw new InvalidOperationException("Cannot bind more than once.");
                }

                _callback = CreateCallback(store, host);

                var parameters = Interop.ToValueTypeVec(Import.Parameters);
                var results = Interop.ToValueTypeVec(Import.Results);
                using (var funcType = Interop.wasm_functype_new(ref parameters, ref results))
                {
                    return Interop.wasm_func_new(store.Handle, funcType, _callback);
                }
            }
        }

        private void Validate()
        {
            if (Method.IsStatic)
            {
                ThrowBindingException(Import, Method, "method cannot be static");
            }

            if (Method.IsGenericMethod)
            {
                ThrowBindingException(Import, Method, "method cannot be generic");
            }

            if (Method.IsConstructor)
            {
                ThrowBindingException(Import, Method, "method cannot be a constructor");
            }

            ValidateParameters();

            ValidateReturnType();
        }

        private void ValidateParameters()
        {
            var parameters = Method.GetParameters();
            if (parameters.Length != Import.Parameters.Count)
            {
                ThrowBindingException(
                    Import,
                    Method,
                    $"parameter mismatch: import requires {Import.Parameters.Count} but the method has {parameters.Length}");
            }

            for (int i = 0; i < parameters.Length; ++i)
            {
                var parameter = parameters[i];
                if (parameter.ParameterType.IsByRef)
                {
                    if (parameter.IsOut)
                    {
                        ThrowBindingException(Import, Method, $"parameter '{parameter.Name}' cannot be an 'out' parameter");
                    }
                    else
                    {
                        ThrowBindingException(Import, Method, $"parameter '{parameter.Name}' cannot be a 'ref' parameter");
                    }
                }

                var expected = Import.Parameters[i];
                if (!Interop.TryGetValueKind(parameter.ParameterType, out var kind) || !Interop.IsMatchingKind(kind, expected))
                {
                    ThrowBindingException(Import, Method, $"method parameter '{parameter.Name}' is expected to be of type '{Interop.ToString(expected)}'");
                }
            }
        }

        private void ValidateReturnType()
        {
            int resultsCount = Import.Results.Count();
            if (resultsCount == 0)
            {
                if (Method.ReturnType != typeof(void))
                {
                    ThrowBindingException(Import, Method, "method must return void");
                }
            }
            else if (resultsCount == 1)
            {
                var expected = Import.Results[0];
                if (!Interop.TryGetValueKind(Method.ReturnType, out var kind) || !Interop.IsMatchingKind(kind, expected))
                {
                    ThrowBindingException(Import, Method, $"return type is expected to be '{Interop.ToString(expected)}'");
                }
            }
            else
            {
                if (!IsTupleOfSize(Method.ReturnType, resultsCount))
                {
                    ThrowBindingException(Import, Method, $"return type is expected to be a tuple of size {resultsCount}");
                }

                var typeArguments =
                    Method.ReturnType.GetGenericArguments().SelectMany(type =>
                    {
                        if (type.IsConstructedGenericType)
                        {
                            return type.GenericTypeArguments;
                        }
                        return Enumerable.Repeat(type, 1);
                    });

                int i = 0;
                foreach (var typeArgument in typeArguments)
                {
                    var expected = Import.Results[i];
                    if (!Interop.TryGetValueKind(typeArgument, out var kind) || !Interop.IsMatchingKind(kind, expected))
                    {
                        ThrowBindingException(Import, Method, $"return tuple item #{i} is expected to be of type '{Interop.ToString(expected)}'");
                    }

                    ++i;
                }
            }
        }

        private static bool IsTupleOfSize(Type type, int size)
        {
            if (!type.IsConstructedGenericType)
            {
                return false;
            }

            var definition = type.GetGenericTypeDefinition();

            if (size == 0)
            {
                return definition == typeof(ValueTuple);
            }

            if (size == 1)
            {
                return definition == typeof(ValueTuple<>);
            }

            if (size == 2)
            {
                return definition == typeof(ValueTuple<,>);
            }

            if (size == 3)
            {
                return definition == typeof(ValueTuple<,,>);
            }

            if (size == 4)
            {
                return definition == typeof(ValueTuple<,,,>);
            }

            if (size == 5)
            {
                return definition == typeof(ValueTuple<,,,,>);
            }

            if (size == 6)
            {
                return definition == typeof(ValueTuple<,,,,,>);
            }

            if (size == 7)
            {
                return definition == typeof(ValueTuple<,,,,,,>);
            }

            if (definition != typeof(ValueTuple<,,,,,,,>))
            {
                return false;
            }

            return IsTupleOfSize(type.GetGenericArguments().Last(), size - 7);
        }

        private unsafe Interop.WasmFuncCallback CreateCallback(Store store, IHost host)
        {
            var args = new object[Import.Parameters.Count];
            bool hasReturn = Method.ReturnType != typeof(void);
            var storeHandle = store.Handle;

            Interop.WasmFuncCallback callback = (arguments, results) =>
            {
                try
                {
                    SetArgs(arguments, args);

                    var result = Method.Invoke(host, BindingFlags.DoNotWrapExceptions, null, args, null);

                    if (hasReturn)
                    {
                        SetResults(result, results);
                    }
                    return IntPtr.Zero;
                }
                catch (Exception ex)
                {
                    var bytes = Encoding.UTF8.GetBytes(ex.Message + "\0" /* exception messages need a null */);

                    fixed (byte* ptr = bytes)
                    {
                        Interop.wasm_byte_vec_t message = new Interop.wasm_byte_vec_t();
                        message.size = (UIntPtr)bytes.Length;
                        message.data = ptr;

                        return Interop.wasm_trap_new(storeHandle, ref message);
                    }
                }
            };

            return callback;
        }

        private static unsafe void SetArgs(Interop.wasm_val_t* arguments, object[] args)
        {
            for (int i = 0; i < args.Length; ++i)
            {
                var arg = arguments[i];

                switch (arg.kind)
                {
                    case Interop.wasm_valkind_t.WASM_I32:
                        args[i] = arg.of.i32;
                        break;

                    case Interop.wasm_valkind_t.WASM_I64:
                        args[i] = arg.of.i64;
                        break;

                    case Interop.wasm_valkind_t.WASM_F32:
                        args[i] = arg.of.f32;
                        break;

                    case Interop.wasm_valkind_t.WASM_F64:
                        args[i] = arg.of.f64;
                        break;

                    default:
                        throw new NotSupportedException("Unsupported value type.");
                }
            }
        }

        private static unsafe void SetResults(object value, Interop.wasm_val_t* results)
        {
            var tuple = value as ITuple;
            if (tuple is null)
            {
                SetResult(value, &results[0]);
            }
            else
            {
                for (int i = 0; i < tuple.Length; ++i)
                {
                    SetResults(tuple[i], &results[i]);
                }
            }
        }

        private static unsafe void SetResult(object value, Interop.wasm_val_t* result)
        {
            switch (value)
            {
                case int i:
                    result->kind = Interop.wasm_valkind_t.WASM_I32;
                    result->of.i32 = i;
                    break;

                case long l:
                    result->kind = Interop.wasm_valkind_t.WASM_I64;
                    result->of.i64 = l;
                    break;

                case float f:
                    result->kind = Interop.wasm_valkind_t.WASM_F32;
                    result->of.f32 = f;
                    break;

                case double d:
                    result->kind = Interop.wasm_valkind_t.WASM_F64;
                    result->of.f64 = d;
                    break;

                default:
                    throw new NotSupportedException("Unsupported return value type.");
            }
        }

        private Interop.WasmFuncCallback _callback;
    }
}

using System;
using System.Linq;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text;

namespace Wasmtime
{
    /// <summary>
    /// Represents a host function.
    /// </summary>
    public class Function : IDisposable
    {
        /// <inheritdoc/>
        public void Dispose()
        {
            if (!Handle.IsInvalid)
            {
                Handle.Dispose();
                Handle.SetHandleAsInvalid();
            }
        }

        internal Function(Interop.StoreHandle store, Delegate func, bool hasReturn)
        {
            if (func is null)
            {
                throw new ArgumentNullException(nameof(func));
            }

            var type = func.GetType();
            Span<Type> parameterTypes = null;
            Type returnType = null;

            if (hasReturn)
            {
                parameterTypes = type.GenericTypeArguments[0..^1];
                returnType = type.GenericTypeArguments[^1];
            }
            else
            {
                parameterTypes = type.GenericTypeArguments;
                returnType = null;
            }

            bool hasCaller = parameterTypes.Length > 0 && parameterTypes[0] == typeof(Caller);

            if (hasCaller)
            {
                parameterTypes = parameterTypes[1..];
            }

            ValidateParameterTypes(parameterTypes);

            ValidateReturnType(returnType);

            var parameters = CreateValueTypeVec(parameterTypes);
            var results = CreateReturnValueTypeVec(returnType);
            using var funcType = Interop.wasm_functype_new(ref parameters, ref results);

            if (hasCaller)
            {
                Callback = CreateCallbackWithCaller(store, func, parameterTypes.Length, hasReturn);
                Handle = Interop.wasmtime_func_new(store, funcType, (Interop.WasmtimeFuncCallback)Callback);
            }
            else
            {
                Callback = CreateCallback(store, func, parameterTypes.Length, hasReturn);
                Handle = Interop.wasm_func_new(store, funcType, (Interop.WasmFuncCallback)Callback);
            }

            if (Handle.IsInvalid)
            {
                throw new WasmtimeException("Failed to create Wasmtime function.");
            }
        }

        private static void ValidateParameterTypes(Span<Type> parameters)
        {
            foreach (var type in parameters)
            {
                if (type == typeof(Caller))
                {
                    throw new WasmtimeException($"A Caller parameter must be the first parameter of the function.");
                }

                if (!Interop.TryGetValueKind(type, out var kind))
                {
                    throw new WasmtimeException($"Unable to create a function with parameter of type '{type.ToString()}'.");
                }
            }
        }

        private static void ValidateReturnType(Type returnType)
        {
            if (returnType is null)
            {
                return;
            }

            if (IsTuple(returnType))
            {
                var types = returnType
                    .GetGenericArguments()
                    .SelectMany(type =>
                        {
                            if (type.IsConstructedGenericType)
                            {
                                return type.GenericTypeArguments;
                            }
                            return Enumerable.Repeat(type, 1);
                        });

                foreach (var type in types)
                {
                    ValidateReturnType(type);
                }
                return;
            }

            if (!Interop.TryGetValueKind(returnType, out var kind))
            {
                throw new WasmtimeException($"Unable to create a function with a return type of type '{returnType.ToString()}'.");
            }
        }

        private static bool IsTuple(Type type)
        {
            if (!type.IsConstructedGenericType)
            {
                return false;
            }

            var definition = type.GetGenericTypeDefinition();

            return definition == typeof(ValueTuple) ||
                   definition == typeof(ValueTuple<>) ||
                   definition == typeof(ValueTuple<,>) ||
                   definition == typeof(ValueTuple<,,>) ||
                   definition == typeof(ValueTuple<,,,>) ||
                   definition == typeof(ValueTuple<,,,,>) ||
                   definition == typeof(ValueTuple<,,,,,>) ||
                   definition == typeof(ValueTuple<,,,,,,>) ||
                   definition == typeof(ValueTuple<,,,,,,,>);
        }

        private static unsafe Interop.WasmFuncCallback CreateCallback(Interop.StoreHandle store, Delegate func, int parameterCount, bool hasReturn)
        {
            // NOTE: this capture is not thread-safe.
            var args = new object[parameterCount];
            var method = func.Method;
            var target = func.Target;

            return (arguments, results) =>
            {
                try
                {
                    SetArgs(arguments, args);

                    var result = method.Invoke(target, BindingFlags.DoNotWrapExceptions, null, args, null);

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

                        return Interop.wasm_trap_new(store, ref message);
                    }
                }
            };
        }

        private static unsafe Interop.WasmtimeFuncCallback CreateCallbackWithCaller(Interop.StoreHandle store, Delegate func, int parameterCount, bool hasReturn)
        {
            // NOTE: this capture is not thread-safe.
            var args = new object[parameterCount + 1];
            var caller = new Caller();
            var method = func.Method;
            var target = func.Target;

            args[0] = caller;

            return (callerHandle, arguments, results) =>
            {
                try
                {
                    caller.Handle = callerHandle;

                    SetArgs(arguments, args, 1);

                    var result = method.Invoke(target, BindingFlags.DoNotWrapExceptions, null, args, null);

                    caller.Handle = IntPtr.Zero;

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

                        return Interop.wasm_trap_new(store, ref message);
                    }
                }
            };
        }

        private static unsafe void SetArgs(Interop.wasm_val_t* arguments, object[] args, int offset = 0)
        {
            for (int i = 0; i < args.Length - offset; ++i)
            {
                var arg = arguments[i];

                switch (arg.kind)
                {
                    case Interop.wasm_valkind_t.WASM_I32:
                        args[i + offset] = arg.of.i32;
                        break;

                    case Interop.wasm_valkind_t.WASM_I64:
                        args[i + offset] = arg.of.i64;
                        break;

                    case Interop.wasm_valkind_t.WASM_F32:
                        args[i + offset] = arg.of.f32;
                        break;

                    case Interop.wasm_valkind_t.WASM_F64:
                        args[i + offset] = arg.of.f64;
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

        private static Interop.wasm_valtype_vec_t CreateValueTypeVec(Span<Type> types)
        {
            Interop.wasm_valtype_vec_t vec;
            Interop.wasm_valtype_vec_new_uninitialized(out vec, (UIntPtr)types.Length);

            int i = 0;
            foreach (var type in types)
            {
                var valType = Interop.wasm_valtype_new((Interop.wasm_valkind_t)Interop.ToValueKind(type));
                unsafe
                {
                    vec.data[i++] = valType.DangerousGetHandle();
                }
                valType.SetHandleAsInvalid();
            }

            return vec;
        }

        private static Interop.wasm_valtype_vec_t CreateReturnValueTypeVec(Type returnType)
        {
            if (returnType is null)
            {
                Interop.wasm_valtype_vec_t vec;
                Interop.wasm_valtype_vec_new_empty(out vec);
                return vec;
            }

            if (IsTuple(returnType))
            {
                return CreateValueTypeVec(
                    returnType
                        .GetGenericArguments()
                        .SelectMany(type =>
                            {
                                if (type.IsConstructedGenericType)
                                {
                                    return type.GenericTypeArguments;
                                }
                                return Enumerable.Repeat(type, 1);
                            })
                        .ToArray()
                );
            }

            return CreateValueTypeVec(new Type[] { returnType });
        }

        internal Interop.FunctionHandle Handle { get; private set; }
        internal Delegate Callback { get; private set; }
    }
}

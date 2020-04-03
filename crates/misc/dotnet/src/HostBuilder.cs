using System;
using System.Collections.Generic;
using System.Linq;

namespace Wasmtime
{
    /// <summary>
    /// Represents the Wasmtime compiler strategy.
    /// </summary>
    public enum CompilerStrategy
    {
        /// <summary>
        /// Automatically pick the compiler strategy.
        /// </summary>
        Auto,
        /// <summary>
        /// Use the Cranelift compiler.
        /// </summary>
        Cranelift,
        /// <summary>
        /// Use the Lightbeam compiler.
        /// </summary>
        Lightbeam
    }

    /// <summary>
    /// Represents the Wasmtime optimization level.
    /// </summary>
    public enum OptimizationLevel
    {
        /// <summary>
        /// Disable optimizations.
        /// </summary>
        None,
        /// <summary>
        /// Optimize for speed.
        /// </summary>
        Speed,
        /// <summary>
        /// Optimize for speed and size.
        /// </summary>
        SpeedAndSize
    }

    /// <summary>
    /// Represents a builder of <see cref="Host"/> instances.
    /// </summary>
    public class HostBuilder
    {
        /// <summary>
        /// Sets whether or not to enable debug information.
        /// </summary>
        /// <param name="enable">True to enable debug information or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithDebugInfo(bool enable)
        {
            _enableDebugInfo = enable;
            return this;
        }

        /// <summary>
        /// Sets whether or not enable WebAssembly threads support.
        /// </summary>
        /// <param name="enable">True to enable WebAssembly threads support or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithWasmThreads(bool enable)
        {
            _enableWasmThreads = enable;
            return this;
        }

        /// <summary>
        /// Sets whether or not enable WebAssembly reference types support.
        /// </summary>
        /// <param name="enable">True to enable WebAssembly reference types support or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithReferenceTypes(bool enable)
        {
            _enableReferenceTypes = enable;
            return this;
        }

        /// <summary>
        /// Sets whether or not enable WebAssembly SIMD support.
        /// </summary>
        /// <param name="enable">True to enable WebAssembly SIMD support or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithSIMD(bool enable)
        {
            _enableSIMD = enable;
            return this;
        }

        /// <summary>
        /// Sets whether or not enable WebAssembly multi-value support.
        /// </summary>
        /// <param name="enable">True to enable WebAssembly multi-value support or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithMultiValue(bool enable)
        {
            _enableMultiValue = enable;
            return this;
        }

        /// <summary>
        /// Sets whether or not enable WebAssembly bulk memory support.
        /// </summary>
        /// <param name="enable">True to enable WebAssembly bulk memory support or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithBulkMemory(bool enable)
        {
            _enableBulkMemory = enable;
            return this;
        }

        /// <summary>
        /// Sets the compiler strategy to use.
        /// </summary>
        /// <param name="strategy">The compiler strategy to use.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithCompilerStrategy(CompilerStrategy strategy)
        {
            switch (strategy)
            {
                case CompilerStrategy.Auto:
                    _strategy = Interop.wasmtime_strategy_t.WASMTIME_STRATEGY_AUTO;
                    break;

                case CompilerStrategy.Cranelift:
                    _strategy = Interop.wasmtime_strategy_t.WASMTIME_STRATEGY_CRANELIFT;
                    break;

                case CompilerStrategy.Lightbeam:
                    _strategy = Interop.wasmtime_strategy_t.WASMTIME_STRATEGY_LIGHTBEAM;
                    break;

                default:
                    throw new ArgumentOutOfRangeException(nameof(strategy));
            }
            return this;
        }

        /// <summary>
        /// Sets whether or not enable the Cranelift debug verifier.
        /// </summary>
        /// <param name="enable">True to enable the Cranelift debug verifier or false to disable.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithCraneliftDebugVerifier(bool enable)
        {
            _enableCraneliftDebugVerifier = enable;
            return this;
        }

        /// <summary>
        /// Sets the optimization level to use.
        /// </summary>
        /// <param name="level">The optimization level to use.</param>
        /// <returns>Returns the current builder.</returns>
        public HostBuilder WithOptimizationLevel(OptimizationLevel level)
        {
            switch (level)
            {
                case OptimizationLevel.None:
                    _optLevel = Interop.wasmtime_opt_level_t.WASMTIME_OPT_LEVEL_NONE;
                    break;

                case OptimizationLevel.Speed:
                    _optLevel = Interop.wasmtime_opt_level_t.WASMTIME_OPT_LEVEL_SPEED;
                    break;

                case OptimizationLevel.SpeedAndSize:
                    _optLevel = Interop.wasmtime_opt_level_t.WASMTIME_OPT_LEVEL_SPEED_AND_SIZE;
                    break;

                default:
                    throw new ArgumentOutOfRangeException(nameof(level));
            }
            return this;
        }

        /// <summary>
        /// Builds the <see cref="Host" /> instance.
        /// </summary>
        /// <returns>Returns the new <see cref="Host" /> instance.</returns>
        public Host Build()
        {
            var config = Interop.wasm_config_new();

            if (_enableDebugInfo.HasValue)
            {
                Interop.wasmtime_config_debug_info_set(config, _enableDebugInfo.Value);
            }

            if (_enableWasmThreads.HasValue)
            {
                Interop.wasmtime_config_wasm_threads_set(config, _enableWasmThreads.Value);
            }

            if (_enableReferenceTypes.HasValue)
            {
                Interop.wasmtime_config_wasm_reference_types_set(config, _enableReferenceTypes.Value);
            }

            if (_enableSIMD.HasValue)
            {
                Interop.wasmtime_config_wasm_simd_set(config, _enableSIMD.Value);
            }

            if (_enableBulkMemory.HasValue)
            {
                Interop.wasmtime_config_wasm_bulk_memory_set(config, _enableBulkMemory.Value);
            }

            if (_enableMultiValue.HasValue)
            {
                Interop.wasmtime_config_wasm_multi_value_set(config, _enableMultiValue.Value);
            }

            if (_strategy.HasValue)
            {
                Interop.wasmtime_config_strategy_set(config, _strategy.Value);
            }

            if (_enableCraneliftDebugVerifier.HasValue)
            {
                Interop.wasmtime_config_cranelift_debug_verifier_set(config, _enableCraneliftDebugVerifier.Value);
            }

            if (_optLevel.HasValue)
            {
                Interop.wasmtime_config_cranelift_opt_level_set(config, _optLevel.Value);
            }

            return new Host(config);
        }

        private bool? _enableDebugInfo;
        private bool? _enableWasmThreads;
        private bool? _enableReferenceTypes;
        private bool? _enableSIMD;
        private bool? _enableBulkMemory;
        private bool? _enableMultiValue;
        private Interop.wasmtime_strategy_t? _strategy;
        private bool? _enableCraneliftDebugVerifier;
        private Interop.wasmtime_opt_level_t? _optLevel;
    }
}

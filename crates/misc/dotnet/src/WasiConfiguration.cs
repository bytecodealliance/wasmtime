using System;
using System.Collections.Generic;
using System.Linq;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WASI configuration.
    /// </summary>
    public class WasiConfiguration
    {
        /// <summary>
        /// Adds a command line argument to the configuration.
        /// </summary>
        /// <param name="arg">The command line argument to add.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithArg(string arg)
        {
            if (arg is null)
            {
                throw new ArgumentNullException(nameof(arg));
            }

            if (_inheritArgs)
            {
                _args.Clear();
                _inheritArgs = false;
            }

            _args.Add(arg);
            return this;
        }

        /// <summary>
        /// Adds multiple command line arguments to the configuration.
        /// </summary>
        /// <param name="args">The command line arguments to add.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithArgs(IEnumerable<string> args)
        {
            if (args is null)
            {
                throw new ArgumentNullException(nameof(args));
            }

            if (_inheritArgs)
            {
                _args.Clear();
                _inheritArgs = false;
            }
            
            foreach (var arg in args)
            {
                _args.Add(arg);
            }
            return this;
        }

        /// <summary>
        /// Adds multiple command line arguments to the configuration.
        /// </summary>
        /// <param name="args">The command line arguments to add.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithArgs(params string[] args)
        {
            return WithArgs((IEnumerable<string>)args);
        }

        /// <summary>
        /// Sets the configuration to inherit command line arguments.
        /// </summary>
        /// <remarks>Any explicitly specified command line arguments will be removed.</remarks>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithInheritedArgs()
        {
            _inheritArgs = true;
            _args.Clear();
            _args.AddRange(Environment.GetCommandLineArgs());
            return this;
        }

        /// <summary>
        /// Adds an environment variable to the configuration.
        /// </summary>
        /// <param name="name">The name of the environment variable.</param>
        /// <param name="value">The value of the environment variable.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithEnvironmentVariable(string name, string value)
        {
            if (name is null)
            {
                throw new ArgumentNullException(nameof(name));
            }
            if (value is null)
            {
                throw new ArgumentNullException(nameof(value));
            }

            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentException("Environment variable name cannot be empty.", nameof(name));
            }

            _inheritEnv = false;
            _vars.Add((name, value));
            return this;
        }

        /// <summary>
        /// Adds multiple environment variables to the configuration.
        /// </summary>
        /// <param name="vars">The name-value tuples of the environment variables to add.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithEnvironmentVariables(IEnumerable<(string,string)> vars)
        {
            if (vars is null)
            {
                throw new ArgumentNullException(nameof(vars));
            }

            _inheritEnv = false;
           
            foreach (var v in vars)
            {
                _vars.Add(v);
            }

            return this;
        }

        /// <summary>
        /// Sets the configuration to inherit environment variables.
        /// </summary>
        /// <remarks>Any explicitly specified environment variables will be removed.</remarks>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithInheritedEnvironment()
        {
            _inheritEnv = true;
            _vars.Clear();
            return this;
        }

        /// <summary>
        /// Sets the configuration to use the given file path as stdin.
        /// </summary>
        /// <param name="path">The file to use as stdin.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithStandardInput(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new ArgumentException("The path cannot be null or empty.", nameof(path));
            }

            _inheritStandardInput = false;
            _standardInputPath = path;
            return this;
        }

        /// <summary>
        /// Sets the configuration to inherit stdin.
        /// </summary>
        /// <remarks>Any explicitly specified stdin file will be removed.</remarks>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithInheritedStandardInput()
        {
            _inheritStandardInput = true;
            _standardInputPath = null;
            return this;
        }

        /// <summary>
        /// Sets the configuration to use the given file path as stdout.
        /// </summary>
        /// <param name="path">The file to use as stdout.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithStandardOutput(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new ArgumentException("The path cannot be null or empty.", nameof(path));
            }

            _inheritStandardOutput = false;
            _standardOutputPath = path;
            return this;
        }

        /// <summary>
        /// Sets the configuration to inherit stdout.
        /// </summary>
        /// <remarks>Any explicitly specified stdout file will be removed.</remarks>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithInheritedStandardOutput()
        {
            _inheritStandardOutput = true;
            _standardOutputPath = null;
            return this;
        }

        /// <summary>
        /// Sets the configuration to use the given file path as stderr.
        /// </summary>
        /// <param name="path">The file to use as stderr.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithStandardError(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new ArgumentException("The path cannot be null or empty.", nameof(path));
            }

            _inheritStandardError = false;
            _standardErrorPath = path;
            return this;
        }

        /// <summary>
        /// Sets the configuration to inherit stderr.
        /// </summary>
        /// <remarks>Any explicitly specified stderr file will be removed.</remarks>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithInheritedStandardError()
        {
            _inheritStandardError = true;
            _standardErrorPath = null;
            return this;
        }

        /// <summary>
        /// Adds a preopen directory to the configuration.
        /// </summary>
        /// <param name="path">The path to the directory to add.</param>
        /// <param name="guestPath">The path the guest will use to open the directory.</param>
        /// <returns>Returns the current configuration.</returns>
        public WasiConfiguration WithPreopenedDirectory(string path, string guestPath)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new ArgumentException("The path cannot be null or empty.", nameof(path));
            }
            if (string.IsNullOrEmpty(guestPath))
            {
                throw new ArgumentException("The guest path cannot be null or empty.", nameof(guestPath));
            }

            _preopenDirs.Add((path, guestPath));
            return this;
        }

        internal Interop.WasiInstanceHandle CreateWasi(Interop.StoreHandle store, string name)
        {
            var config = Interop.wasi_config_new();

            SetConfigArgs(config);
            SetEnvironmentVariables(config);
            SetStandardIn(config);
            SetStandardOut(config);
            SetStandardError(config);
            SetPreopenDirectories(config);

            IntPtr trap;
            var wasi = Interop.wasi_instance_new(store, name, config, out trap);
            config.SetHandleAsInvalid();

            if (trap != IntPtr.Zero)
            {
                throw TrapException.FromOwnedTrap(trap);
            }

            if (wasi.IsInvalid)
            {
                throw new WasmtimeException($"Failed to create instance for WASI module '{name}'.");
            }

            return wasi;
        }

        private unsafe void SetConfigArgs(Interop.WasiConfigHandle config)
        {
            // Don't call wasi_config_inherit_argv as the command line to the .NET program may not be
            // the same as the process' command line (e.g. `dotnet foo.dll foo bar baz` => "foo.dll foo bar baz").
            if (_args.Count == 0)
            {
                return;
            }

            var (args, handles) = Interop.ToUTF8PtrArray(_args);
            
            try
            {
                Interop.wasi_config_set_argv(config, _args.Count, args);
            }
            finally
            {
                foreach (var handle in handles)
                {
                    handle.Free();
                }
            }
        }

        private unsafe void SetEnvironmentVariables(Interop.WasiConfigHandle config)
        {
            if (_inheritEnv)
            {
                Interop.wasi_config_inherit_env(config);
                return;
            }

            if (_vars.Count == 0)
            {
                return;
            }

            var (names, nameHandles) = Interop.ToUTF8PtrArray(_vars.Select(var => var.Name).ToArray());
            var (values, valueHandles) = Interop.ToUTF8PtrArray(_vars.Select(var => var.Value).ToArray());
            
            try
            {
                Interop.wasi_config_set_env(config, _vars.Count, names, values);
            }
            finally
            {
                foreach (var handle in nameHandles)
                {
                    handle.Free();
                }

                foreach (var handle in valueHandles)
                {
                    handle.Free();
                }
            }
        }

        private void SetStandardIn(Interop.WasiConfigHandle config)
        {
            if (_inheritStandardInput)
            {
                Interop.wasi_config_inherit_stdin(config);
                return;
            }

            if (!string.IsNullOrEmpty(_standardInputPath))
            {
                if (!Interop.wasi_config_set_stdin_file(config, _standardInputPath))
                {
                    throw new InvalidOperationException($"Failed to set stdin to file '{_standardInputPath}'.");
                }
            }
        }

        private void SetStandardOut(Interop.WasiConfigHandle config)
        {
            if (_inheritStandardOutput)
            {
                Interop.wasi_config_inherit_stdout(config);
                return;
            }

            if (!string.IsNullOrEmpty(_standardOutputPath))
            {
                if (!Interop.wasi_config_set_stdout_file(config, _standardOutputPath))
                {
                    throw new InvalidOperationException($"Failed to set stdout to file '{_standardOutputPath}'.");
                }
            }
        }
        
        private void SetStandardError(Interop.WasiConfigHandle config)
        {
            if (_inheritStandardError)
            {
                Interop.wasi_config_inherit_stderr(config);
                return;
            }

            if (!string.IsNullOrEmpty(_standardErrorPath))
            {
                if (!Interop.wasi_config_set_stderr_file(config, _standardErrorPath))
                {
                    throw new InvalidOperationException($"Failed to set stderr to file '{_standardErrorPath}'.");
                }
            }
        }

        private void SetPreopenDirectories(Interop.WasiConfigHandle config)
        {
            foreach (var dir in _preopenDirs)
            {
                if (!Interop.wasi_config_preopen_dir(config, dir.Path, dir.GuestPath))
                {
                    throw new InvalidOperationException($"Failed to preopen directory '{dir.Path}'.");
                }
            }
        }

        private readonly List<string> _args = new List<string>();
        private readonly List<(string Name, string Value)> _vars = new List<(string, string)>();
        private string _standardInputPath;
        private string _standardOutputPath;
        private string _standardErrorPath;
        private readonly List<(string Path, string GuestPath)> _preopenDirs = new List<(string, string)>();
        private bool _inheritArgs = false;
        private bool _inheritEnv = false;
        private bool _inheritStandardInput = false;
        private bool _inheritStandardOutput = false;
        private bool _inheritStandardError = false;
    }   
}

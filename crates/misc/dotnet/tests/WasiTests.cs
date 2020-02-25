using System;
using System.Collections.Generic;
using System.Linq;
using System.IO;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class WasiFixture : ModuleFixture
    {
        protected override string ModuleFileName => "Wasi.wasm";
    }

    public class WasiTests : IClassFixture<WasiFixture>
    {
        public WasiTests(WasiFixture fixture)
        {
            Fixture = fixture;
        }

        private WasiFixture Fixture { get; set; }

        [Fact]
        public void ItHasNoEnvironmentByDefault()
        {
            using var instance = Fixture.Module.Instantiate(new Wasi(Fixture.Module.Store));
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_environ_sizes_get(0, 4));
            Assert.Equal(0, memory.ReadInt32(0));
            Assert.Equal(0, memory.ReadInt32(4));
        }

        [Fact]
        public void ItHasSpecifiedEnvironment()
        {
            var env = new Dictionary<string, string>() {
                {"FOO", "BAR"},
                {"WASM", "IS"},
                {"VERY", "COOL"},
            };

            var wasi = new WasiBuilder()
                .WithEnvironmentVariables(env.Select(kvp => (kvp.Key, kvp.Value)))
                .Build(Fixture.Module.Store);

            using var instance = Fixture.Module.Instantiate(wasi);
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_environ_sizes_get(0, 4));
            Assert.Equal(env.Count, memory.ReadInt32(0));
            Assert.Equal(env.Sum(kvp => kvp.Key.Length + kvp.Value.Length + 2), memory.ReadInt32(4));
            Assert.Equal(0, inst.call_environ_get(0, 4 * env.Count));

            for (int i = 0; i < env.Count; ++i)
            {
                var kvp = memory.ReadNullTerminatedString(memory.ReadInt32(i * 4)).Split("=");
                Assert.Equal(env[kvp[0]], kvp[1]);
            }
        }

        [Fact]
        public void ItInheritsEnvironment()
        {
            var wasi = new WasiBuilder()
                .WithInheritedEnvironment()
                .Build(Fixture.Module.Store);

            using var instance = Fixture.Module.Instantiate(wasi);
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_environ_sizes_get(0, 4));
            Assert.Equal(Environment.GetEnvironmentVariables().Keys.Count, memory.ReadInt32(0));
        }

        [Fact]
        public void ItHasNoArgumentsByDefault()
        {
            using var instance = Fixture.Module.Instantiate(new Wasi(Fixture.Module.Store));
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_args_sizes_get(0, 4));
            Assert.Equal(0, memory.ReadInt32(0));
            Assert.Equal(0, memory.ReadInt32(4));
        }

        [Fact]
        public void ItHasSpecifiedArguments()
        {
            var args = new List<string>() {
                "WASM",
                "IS",
                "VERY",
                "COOL"
            };

            var wasi = new WasiBuilder()
                .WithArgs(args)
                .Build(Fixture.Module.Store);

            using var instance = Fixture.Module.Instantiate(wasi);
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_args_sizes_get(0, 4));
            Assert.Equal(args.Count, memory.ReadInt32(0));
            Assert.Equal(args.Sum(a => a.Length + 1), memory.ReadInt32(4));
            Assert.Equal(0, inst.call_args_get(0, 4 * args.Count));

            for (int i = 0; i < args.Count; ++i)
            {
                var arg = memory.ReadNullTerminatedString(memory.ReadInt32(i * 4));
                Assert.Equal(args[i], arg);
            }
        }

        [Fact]
        public void ItInheritsArguments()
        {
            var wasi = new WasiBuilder()
                .WithInheritedArgs()
                .Build(Fixture.Module.Store);

            using var instance = Fixture.Module.Instantiate(wasi);
            dynamic inst = instance;

            var memory = instance.Externs.Memories[0];

            Assert.Equal(0, inst.call_args_sizes_get(0, 4));
            Assert.Equal(Environment.GetCommandLineArgs().Length, memory.ReadInt32(0));
        }

        [Fact]
        public void ItSetsStdIn()
        {
            const string MESSAGE = "WASM IS VERY COOL";

            using (var file = new TempFile())
            {
                File.WriteAllText(file.Path, MESSAGE);

                var wasi = new WasiBuilder()
                    .WithStandardInput(file.Path)
                    .Build(Fixture.Module.Store);

                using var instance = Fixture.Module.Instantiate(wasi);
                dynamic inst = instance;

                var memory = instance.Externs.Memories[0];
                memory.WriteInt32(0, 8);
                memory.WriteInt32(4, MESSAGE.Length);

                Assert.Equal(0, inst.call_fd_read(0, 0, 1, 32));
                Assert.Equal(MESSAGE.Length, memory.ReadInt32(32));
                Assert.Equal(MESSAGE, memory.ReadString(8, MESSAGE.Length));
            }
        }

        [Theory]
        [InlineData(1)]
        [InlineData(2)]
        public void ItSetsStdOutAndStdErr(int fd)
        {
            const string MESSAGE = "WASM IS VERY COOL";

            using (var file = new TempFile())
            {
                var builder = new WasiBuilder();
                if (fd == 1)
                {
                    builder.WithStandardOutput(file.Path);
                }
                else if (fd == 2)
                {
                    builder.WithStandardError(file.Path);
                }

                var wasi = builder.Build(Fixture.Module.Store);

                using var instance = Fixture.Module.Instantiate(wasi);
                dynamic inst = instance;

                var memory = instance.Externs.Memories[0];
                memory.WriteInt32(0, 8);
                memory.WriteInt32(4, MESSAGE.Length);
                memory.WriteString(8, MESSAGE);

                Assert.Equal(0, inst.call_fd_write(fd, 0, 1, 32));
                Assert.Equal(MESSAGE.Length, memory.ReadInt32(32));
                Assert.Equal(0, inst.call_fd_close(fd));
                Assert.Equal(MESSAGE, File.ReadAllText(file.Path));
            }
        }

        [Fact]
        public void ItSetsPreopenDirectories()
        {
            const string MESSAGE = "WASM IS VERY COOL";

            using (var file = new TempFile())
            {
                var wasi = new WasiBuilder()
                    .WithPreopenedDirectory(Path.GetDirectoryName(file.Path), "/foo")
                    .Build(Fixture.Module.Store);

                using var instance = Fixture.Module.Instantiate(wasi);
                dynamic inst = instance;

                var memory = instance.Externs.Memories[0];
                var fileName = Path.GetFileName(file.Path);
                memory.WriteString(0, fileName);

                Assert.Equal(0, inst.call_path_open(
                        3,
                        0,
                        0,
                        fileName.Length,
                        0,
                        0x40 /* RIGHTS_FD_WRITE */,
                        0,
                        0,
                        64
                    )
                );

                var fileFd = (int) memory.ReadInt32(64);
                Assert.True(fileFd > 3);

                memory.WriteInt32(0, 8);
                memory.WriteInt32(4, MESSAGE.Length);
                memory.WriteString(8, MESSAGE);

                Assert.Equal(0, inst.call_fd_write(fileFd, 0, 1, 64));
                Assert.Equal(MESSAGE.Length, memory.ReadInt32(64));
                Assert.Equal(0, inst.call_fd_close(fileFd));
                Assert.Equal(MESSAGE, File.ReadAllText(file.Path));
            }
        }
    }
}

using System;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class MemoryImportBindingFixture : ModuleFixture
    {
        protected override string ModuleFileName => "MemoryImportBinding.wat";
    }

    public class MemoryImportBindingTests : IClassFixture<MemoryImportBindingFixture>
    {
        class MissingImportsHost : IHost
        {
            public Instance Instance { get; set; }
        }

        class MemoryIsStaticHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public static Memory x = new Memory(minimum: 1);
        }

        class MemoryIsNotReadOnlyHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public Memory x = new Memory(minimum: 1);
        }

        class NotAMemoryHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public readonly int x = 0;
        }

        class InvalidMinimumHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public readonly Memory Mem = new Memory(minimum: 2);
        }

        class InvalidMaximumHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public readonly Memory Mem = new Memory(maximum: 2);
        }

        class ValidHost : IHost
        {
            public Instance Instance { get; set; }

            [Import("mem")]
            public readonly Memory Mem = new Memory(minimum: 1);
        }

        public MemoryImportBindingTests(MemoryImportBindingFixture fixture)
        {
            Fixture = fixture;
        }

        private MemoryImportBindingFixture Fixture { get; set; }

        [Fact]
        public void ItFailsToInstantiateWithMissingImport()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new MissingImportsHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Failed to bind memory import 'mem': the host does not contain a memory field with a matching 'Import' attribute.");
        }

        [Fact]
        public void ItFailsToInstantiateWithStaticField()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new MemoryIsStaticHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'MemoryIsStaticHost.x' to WebAssembly import 'mem': field cannot be static.");
        }

        [Fact]
        public void ItFailsToInstantiateWithNonReadOnlyField()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new MemoryIsNotReadOnlyHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'MemoryIsNotReadOnlyHost.x' to WebAssembly import 'mem': field must be readonly.");
        }

        [Fact]
        public void ItFailsToInstantiateWithInvalidType()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new NotAMemoryHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'NotAMemoryHost.x' to WebAssembly import 'mem': field is expected to be of type 'Memory'.");
        }

        [Fact]
        public void ItFailsToInstantiateWhenMemoryHasInvalidMinimum()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new InvalidMinimumHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'InvalidMinimumHost.Mem' to WebAssembly import 'mem': Memory does not have the expected minimum of 1 page(s).");
        }

        [Fact]
        public void ItFailsToInstantiateWhenMemoryHasInvalidMaximum()
        {
            Action action = () => { using var instance = Fixture.Module.Instantiate(new InvalidMaximumHost()); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("Unable to bind 'InvalidMaximumHost.Mem' to WebAssembly import 'mem': Memory does not have the expected maximum of 4294967295 page(s).");
        }

        [Fact]
        public void ItBindsTheGlobalsCorrectly()
        {
            var host = new ValidHost();
            using dynamic instance = Fixture.Module.Instantiate(host);

            host.Mem.ReadString(0, 11).Should().Be("Hello World");
            int written = host.Mem.WriteString(0, "WebAssembly Rocks!");
            host.Mem.ReadString(0, written).Should().Be("WebAssembly Rocks!");

            host.Mem.ReadByte(20).Should().Be(1);
            host.Mem.WriteByte(20, 11);
            host.Mem.ReadByte(20).Should().Be(11);
            ((byte)instance.ReadByte()).Should().Be(11);

            host.Mem.ReadInt16(21).Should().Be(2);
            host.Mem.WriteInt16(21, 12);
            host.Mem.ReadInt16(21).Should().Be(12);
            ((short)instance.ReadInt16()).Should().Be(12);

            host.Mem.ReadInt32(23).Should().Be(3);
            host.Mem.WriteInt32(23, 13);
            host.Mem.ReadInt32(23).Should().Be(13);
            ((int)instance.ReadInt32()).Should().Be(13);

            host.Mem.ReadInt64(27).Should().Be(4);
            host.Mem.WriteInt64(27, 14);
            host.Mem.ReadInt64(27).Should().Be(14);
            ((long)instance.ReadInt64()).Should().Be(14);

            host.Mem.ReadSingle(35).Should().Be(5);
            host.Mem.WriteSingle(35, 15);
            host.Mem.ReadSingle(35).Should().Be(15);
            ((float)instance.ReadFloat32()).Should().Be(15);

            host.Mem.ReadDouble(39).Should().Be(6);
            host.Mem.WriteDouble(39, 16);
            host.Mem.ReadDouble(39).Should().Be(16);
            ((double)instance.ReadFloat64()).Should().Be(16);

            host.Mem.ReadIntPtr(48).Should().Be((IntPtr)7);
            host.Mem.WriteIntPtr(48, (IntPtr)17);
            host.Mem.ReadIntPtr(48).Should().Be((IntPtr)17);
            ((IntPtr)instance.ReadIntPtr()).Should().Be((IntPtr)17);
        }
    }
}

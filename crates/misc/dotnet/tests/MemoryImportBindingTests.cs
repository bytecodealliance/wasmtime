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
        public MemoryImportBindingTests(MemoryImportBindingFixture fixture)
        {
            Fixture = fixture;

            Fixture.Host.ClearDefinitions();
        }

        private MemoryImportBindingFixture Fixture { get; set; }

        [Fact]
        public void ItFailsToInstantiateWithMissingImport()
        {
            Action action = () => { using var instance = Fixture.Host.Instantiate(Fixture.Module); };

            action
                .Should()
                .Throw<WasmtimeException>()
                .WithMessage("unknown import: `::mem` has not been defined");
        }

        [Fact]
        public void ItBindsTheGlobalsCorrectly()
        {
            var mem = Fixture.Host.DefineMemory("", "mem");

            using dynamic instance = Fixture.Host.Instantiate(Fixture.Module);

            mem.ReadString(0, 11).Should().Be("Hello World");
            int written = mem.WriteString(0, "WebAssembly Rocks!");
            mem.ReadString(0, written).Should().Be("WebAssembly Rocks!");

            mem.ReadByte(20).Should().Be(1);
            mem.WriteByte(20, 11);
            mem.ReadByte(20).Should().Be(11);
            ((byte)instance.ReadByte()).Should().Be(11);

            mem.ReadInt16(21).Should().Be(2);
            mem.WriteInt16(21, 12);
            mem.ReadInt16(21).Should().Be(12);
            ((short)instance.ReadInt16()).Should().Be(12);

            mem.ReadInt32(23).Should().Be(3);
            mem.WriteInt32(23, 13);
            mem.ReadInt32(23).Should().Be(13);
            ((int)instance.ReadInt32()).Should().Be(13);

            mem.ReadInt64(27).Should().Be(4);
            mem.WriteInt64(27, 14);
            mem.ReadInt64(27).Should().Be(14);
            ((long)instance.ReadInt64()).Should().Be(14);

            mem.ReadSingle(35).Should().Be(5);
            mem.WriteSingle(35, 15);
            mem.ReadSingle(35).Should().Be(15);
            ((float)instance.ReadFloat32()).Should().Be(15);

            mem.ReadDouble(39).Should().Be(6);
            mem.WriteDouble(39, 16);
            mem.ReadDouble(39).Should().Be(16);
            ((double)instance.ReadFloat64()).Should().Be(16);

            mem.ReadIntPtr(48).Should().Be((IntPtr)7);
            mem.WriteIntPtr(48, (IntPtr)17);
            mem.ReadIntPtr(48).Should().Be((IntPtr)17);
            ((IntPtr)instance.ReadIntPtr()).Should().Be((IntPtr)17);
        }
    }
}

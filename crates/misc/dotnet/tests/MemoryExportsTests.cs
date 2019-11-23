using System;
using System.Collections.Generic;
using System.Linq;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class MemoryExportsFixture : ModuleFixture
    {
        protected override string ModuleFileName => "MemoryExports.wasm";
    }

    public class MemoryExportsTests : IClassFixture<MemoryExportsFixture>
    {
        public class Host : IHost
        {
            public Instance Instance { get; set; }
        }

        public MemoryExportsTests(MemoryExportsFixture fixture)
        {
            Fixture = fixture;
        }

        private MemoryExportsFixture Fixture { get; set; }

        [Theory]
        [MemberData(nameof(GetMemoryExports))]
        public void ItHasTheExpectedMemoryExports(string exportName, uint expectedMinimum, uint expectedMaximum)
        {
            var export = Fixture.Module.Exports.Memories.Where(m => m.Name == exportName).FirstOrDefault();
            export.Should().NotBeNull();
            export.Minimum.Should().Be(expectedMinimum);
            export.Maximum.Should().Be(expectedMaximum);
        }

        [Fact]
        public void ItHasTheExpectedNumberOfExportedTables()
        {
            GetMemoryExports().Count().Should().Be(Fixture.Module.Exports.Memories.Count);
        }

        [Fact]
        public void ItCreatesExternsForTheMemories()
        {
            var host = new Host();
            using (var instance = Fixture.Module.Instantiate(host))
            {
                instance.Externs.Memories.Count.Should().Be(1);

                var memory = instance.Externs.Memories[0];
                memory.ReadString(0, 11).Should().Be("Hello World");
                int written = memory.WriteString(0, "WebAssembly Rocks!");
                memory.ReadString(0, written).Should().Be("WebAssembly Rocks!");

                memory.ReadByte(20).Should().Be(1);
                memory.WriteByte(20, 11);
                memory.ReadByte(20).Should().Be(11);

                memory.ReadInt16(21).Should().Be(2);
                memory.WriteInt16(21, 12);
                memory.ReadInt16(21).Should().Be(12);

                memory.ReadInt32(23).Should().Be(3);
                memory.WriteInt32(23, 13);
                memory.ReadInt32(23).Should().Be(13);

                memory.ReadInt64(27).Should().Be(4);
                memory.WriteInt64(27, 14);
                memory.ReadInt64(27).Should().Be(14);

                memory.ReadSingle(35).Should().Be(5);
                memory.WriteSingle(35, 15);
                memory.ReadSingle(35).Should().Be(15);

                memory.ReadDouble(39).Should().Be(6);
                memory.WriteDouble(39, 16);
                memory.ReadDouble(39).Should().Be(16);

                memory.ReadIntPtr(48).Should().Be((IntPtr)7);
                memory.WriteIntPtr(48, (IntPtr)17);
                memory.ReadIntPtr(48).Should().Be((IntPtr)17);
            }
        }

        public static IEnumerable<object[]> GetMemoryExports()
        {
            yield return new object[] {
                "mem",
                1,
                2
            };
        }
    }
}

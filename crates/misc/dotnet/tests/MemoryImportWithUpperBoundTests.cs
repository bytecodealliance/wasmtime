using System;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class MemoryImportWithUpperBoundFixture : ModuleFixture
    {
        protected override string ModuleFileName => "MemoryImportWithUpperBound.wasm";
    }

    public class MemoryImportWithUpperBoundTests : IClassFixture<MemoryImportWithUpperBoundFixture>
    {
        public MemoryImportWithUpperBoundTests(MemoryImportWithUpperBoundFixture fixture)
        {
            Fixture = fixture;
        }

        private MemoryImportWithUpperBoundFixture Fixture { get; set; }

        [Fact]
        public void ItHasTheExpectedImport()
        {
            Fixture.Module.Imports.Memories.Count.Should().Be(1);

            var memory = Fixture.Module.Imports.Memories[0];

            memory.ModuleName.Should().Be("");
            memory.Name.Should().Be("mem");
            memory.Minimum.Should().Be(10);
            memory.Maximum.Should().Be(100);
        }
    }
}

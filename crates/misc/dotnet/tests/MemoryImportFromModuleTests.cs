using System;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class MemoryImportFromModuleFixture : ModuleFixture
    {
        protected override string ModuleFileName => "MemoryImportFromModule.wat";
    }

    public class MemoryImportFromModuleTests : IClassFixture<MemoryImportFromModuleFixture>
    {
        public MemoryImportFromModuleTests(MemoryImportFromModuleFixture fixture)
        {
            Fixture = fixture;
        }

        private MemoryImportFromModuleFixture Fixture { get; set; }

        [Fact]
        public void ItHasTheExpectedImport()
        {
            Fixture.Module.Imports.Memories.Count.Should().Be(1);

            var memory = Fixture.Module.Imports.Memories[0];

            memory.ModuleName.Should().Be("js");
            memory.Name.Should().Be("mem");
            memory.Minimum.Should().Be(1);
            memory.Maximum.Should().Be(2);
        }
    }
}

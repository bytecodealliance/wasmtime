using System;
using System.Collections.Generic;
using System.Linq;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class TableImportsFixture : ModuleFixture
    {
        protected override string ModuleFileName => "TableImports.wasm";
    }

    public class TableImportsTests : IClassFixture<TableImportsFixture>
    {
        public TableImportsTests(TableImportsFixture fixture)
        {
            Fixture = fixture;
        }

        private TableImportsFixture Fixture { get; set; }

        [Theory]
        [MemberData(nameof(GetTableImports))]
        public void ItHasTheExpectedTableImports(string importModule, string importName, ValueKind expectedKind, uint expectedMinimum, uint expectedMaximum)
        {
            var import = Fixture.Module.Imports.Tables.Where(f => f.ModuleName == importModule && f.Name == importName).FirstOrDefault();
            import.Should().NotBeNull();
            import.Kind.Should().Be(expectedKind);
            import.Minimum.Should().Be(expectedMinimum);
            import.Maximum.Should().Be(expectedMaximum);
        }

        [Fact]
        public void ItHasTheExpectedNumberOfExportedTables()
        {
            GetTableImports().Count().Should().Be(Fixture.Module.Imports.Tables.Count);
        }

        public static IEnumerable<object[]> GetTableImports()
        {
            yield return new object[] {
                "",
                "table1",
                ValueKind.FuncRef,
                10,
                uint.MaxValue
            };

            yield return new object[] {
                "",
                "table2",
                ValueKind.AnyRef,
                15,
                uint.MaxValue
            };

            yield return new object[] {
                "other",
                "table3",
                ValueKind.FuncRef,
                1,
                uint.MaxValue
            };
        }
    }
}

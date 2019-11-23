using System;
using System.Collections.Generic;
using System.Linq;
using FluentAssertions;
using Xunit;

namespace Wasmtime.Tests
{
    public class TableExportsFixture : ModuleFixture
    {
        protected override string ModuleFileName => "TableExports.wasm";
    }

    public class TableExportsTests : IClassFixture<TableExportsFixture>
    {
        public TableExportsTests(TableExportsFixture fixture)
        {
            Fixture = fixture;
        }

        private TableExportsFixture Fixture { get; set; }

        [Theory]
        [MemberData(nameof(GetTableExports))]
        public void ItHasTheExpectedTableExports(string exportName, ValueKind expectedKind, uint expectedMinimum, uint expectedMaximum)
        {
            var export = Fixture.Module.Exports.Tables.Where(f => f.Name == exportName).FirstOrDefault();
            export.Should().NotBeNull();
            export.Kind.Should().Be(expectedKind);
            export.Minimum.Should().Be(expectedMinimum);
            export.Maximum.Should().Be(expectedMaximum);
        }

        [Fact]
        public void ItHasTheExpectedNumberOfExportedTables()
        {
            GetTableExports().Count().Should().Be(Fixture.Module.Exports.Tables.Count);
        }

        public static IEnumerable<object[]> GetTableExports()
        {
            yield return new object[] {
                "table1",
                ValueKind.FuncRef,
                1,
                10
            };

            yield return new object[] {
                "table2",
                ValueKind.AnyRef,
                10,
                uint.MaxValue
            };

            yield return new object[] {
                "table3",
                ValueKind.FuncRef,
                100,
                1000
            };
        }
    }
}

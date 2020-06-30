//! Generating series of `table.get` and `table.set` operations.

use arbitrary::Arbitrary;
use std::fmt::Write;
use std::ops::Range;

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Arbitrary, Debug)]
pub struct TableOps {
    num_params: u8,
    table_size: u32,
    ops: Vec<TableOp>,
}

const NUM_PARAMS_RANGE: Range<u8> = 1..10;
const TABLE_SIZE_RANGE: Range<u32> = 1..100;
const MAX_OPS: usize = 1000;

impl TableOps {
    /// Get the number of parameters this module's "run" function takes.
    pub fn num_params(&self) -> u8 {
        let num_params = std::cmp::max(self.num_params, NUM_PARAMS_RANGE.start);
        let num_params = std::cmp::min(num_params, NUM_PARAMS_RANGE.end);
        num_params
    }

    /// Get the size of the table that this module uses.
    pub fn table_size(&self) -> u32 {
        let table_size = std::cmp::max(self.table_size, TABLE_SIZE_RANGE.start);
        let table_size = std::cmp::min(table_size, TABLE_SIZE_RANGE.end);
        table_size
    }

    /// Convert this into a WAT string.
    ///
    /// The module requires a single import: `(import "" "gc" (func))`. This
    /// should be a function to trigger GC.
    ///
    /// The single export of the module is a function "run" that takes
    /// `self.num_params()` parameters of type `externref`.
    ///
    /// The "run" function is guaranteed to terminate (no loops or recursive
    /// calls), but is not guaranteed to avoid traps (might access out-of-bounds
    /// of the table).
    pub fn to_wat_string(&self) -> String {
        let mut wat = "(module\n".to_string();

        // Import the GC function.
        wat.push_str("  (import \"\" \"gc\" (func))\n");

        // Define our table.
        wat.push_str("  (table $table ");
        write!(&mut wat, "{}", self.table_size()).unwrap();
        wat.push_str(" externref)\n");

        // Define the "run" function export.
        wat.push_str(r#"  (func (export "run") (param"#);
        for _ in 0..self.num_params() {
            wat.push_str(" externref");
        }
        wat.push_str(")\n");
        for op in self.ops.iter().take(MAX_OPS) {
            wat.push_str("    ");
            op.to_wat_string(&mut wat);
            wat.push('\n');
        }
        wat.push_str("  )\n");

        wat.push_str(")\n");
        wat
    }
}

#[derive(Arbitrary, Debug)]
pub(crate) enum TableOp {
    // `(call 0)`
    Gc,
    // `(drop (table.get x))`
    Get(u32),
    // `(table.set x (local.get y))`
    SetFromParam(u32, u8),
    // `(table.set x (table.get y))`
    SetFromGet(u32, u32),
}

impl TableOp {
    fn to_wat_string(&self, wat: &mut String) {
        match self {
            Self::Gc => {
                wat.push_str("(call 0)");
            }
            Self::Get(x) => {
                wat.push_str("(drop (table.get $table (i32.const ");
                write!(wat, "{}", x).unwrap();
                wat.push_str(")))");
            }
            Self::SetFromParam(x, y) => {
                wat.push_str("(table.set $table (i32.const ");
                write!(wat, "{}", x).unwrap();
                wat.push_str(") (local.get ");
                write!(wat, "{}", y).unwrap();
                wat.push_str("))");
            }
            Self::SetFromGet(x, y) => {
                wat.push_str("(table.set $table (i32.const ");
                write!(wat, "{}", x).unwrap();
                wat.push_str(") (table.get $table (i32.const ");
                write!(wat, "{}", y).unwrap();
                wat.push_str(")))");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wat_string() {
        let ops = TableOps {
            num_params: 2,
            table_size: 10,
            ops: vec![
                TableOp::Gc,
                TableOp::Get(0),
                TableOp::SetFromParam(1, 2),
                TableOp::SetFromGet(3, 4),
            ],
        };

        let expected = r#"
(module
  (import "" "gc" (func))
  (table $table 10 externref)
  (func (export "run") (param externref externref)
    (call 0)
    (drop (table.get $table (i32.const 0)))
    (table.set $table (i32.const 1) (local.get 2))
    (table.set $table (i32.const 3) (table.get $table (i32.const 4)))
  )
)
"#;
        let actual = ops.to_wat_string();
        assert_eq!(actual.trim(), expected.trim());
    }
}

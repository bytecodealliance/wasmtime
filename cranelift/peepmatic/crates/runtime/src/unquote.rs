//! Unquote operator definition.

peepmatic_traits::define_operator! {
    /// Compile-time unquote operators.
    ///
    /// These are used in the right-hand side to perform compile-time evaluation of
    /// constants matched on the left-hand side.
    #[allow(missing_docs)]
    UnquoteOperator {
        band => Band {
            parameters(iNN, iNN);
            result(iNN);
        }
        bor => Bor {
            parameters(iNN, iNN);
            result(iNN);
        }
        bxor => Bxor {
            parameters(iNN, iNN);
            result(iNN);
        }
        iadd => Iadd {
            parameters(iNN, iNN);
            result(iNN);
        }
        imul => Imul {
            parameters(iNN, iNN);
            result(iNN);
        }
        isub => Isub {
            parameters(iNN, iNN);
            result(iNN);
        }
        log2 => Log2 {
            parameters(iNN);
            result(iNN);
        }
        neg => Neg {
            parameters(iNN);
            result(iNN);
        }
    }
    parse_cfg(feature = "construct");
}

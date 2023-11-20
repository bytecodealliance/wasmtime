(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x7fffffffffffffff
	i64.const 1
	i64.shl
	i64.const 0xfffffffffffffffe
	call $assert_eq)
 (start $main))

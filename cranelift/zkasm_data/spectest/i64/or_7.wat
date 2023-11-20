(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0xf0f0ffff
	i64.const 0xfffff0f0
	i64.or
	i64.const 0xffffffff
	call $assert_eq)
 (start $main))

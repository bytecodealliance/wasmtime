(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x7f
	i64.extend8_s
	i64.const 127
	call $assert_eq)
 (start $main))

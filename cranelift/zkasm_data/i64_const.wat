(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 9223372036854775807
	i64.const 9223372036854775807
	call $assert_eq)
 (start $main))

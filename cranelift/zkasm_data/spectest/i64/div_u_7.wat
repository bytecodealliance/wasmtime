(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x8000000000000001
	i64.const 1000
	i64.div_u
	i64.const 0x20c49ba5e353f7
	call $assert_eq)
 (start $main))

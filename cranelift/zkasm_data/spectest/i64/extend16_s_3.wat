(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x8000
	i64.extend16_s
	i64.const -32768
	call $assert_eq)
 (start $main))

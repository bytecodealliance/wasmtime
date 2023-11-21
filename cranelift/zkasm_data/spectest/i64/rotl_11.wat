(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0xabd1234ef567809c
	i64.const 0x800000000000003f
	i64.rotl
	i64.const 0x55e891a77ab3c04e
	call $assert_eq)
 (start $main))

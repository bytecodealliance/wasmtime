(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x01234567_89abcd_00
	i64.extend8_s
	i64.const 0
	call $assert_eq)
 (start $main))

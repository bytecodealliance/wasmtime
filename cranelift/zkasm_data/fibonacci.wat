(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	(local $counter i32)
	(local $fp i32)
	(local $f i32)
	(local.set $counter (i32.const 0))
	(local.set $fp (i32.const 0))
	(local.set $f (i32.const 1))
	(block
	 (loop
		(local.get $fp)
		(local.get $f)
		(local.set $fp (local.get $f))
		(local.set $f (i32.add))
		(local.set $counter
		 (i32.add
			(local.get $counter)
			(i32.const 1)))
		(br_if 1
		 (i32.eq
			(local.get $counter)
			(i32.const 10)
		 )
		)
		(br 0)
	 )
	)
	(local.get $f)
	(i32.const 89)
	call $assert_eq)
(start $main))

(assert_fuel 0 (module))

(assert_fuel 1
  (module
    (func $f)
    (start $f)))

(assert_fuel 2
  (module
    (func $f
      i32.const 0
      drop
    )
    (start $f)))

(assert_fuel 1
  (module
    (func $f
      block
      end
    )
    (start $f)))

(assert_fuel 1
  (module
    (func $f
      unreachable
    )
    (start $f)))

(assert_fuel 7
  (module
    (func $f
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      unreachable
    )
    (start $f)))

(assert_fuel 1
  (module
    (func $f
      return
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      unreachable
    )
    (start $f)))

(assert_fuel 3
  (module
    (func $f
      i32.const 0
      if
        call $f
      end
    )
    (start $f)))

(assert_fuel 4
  (module
    (func $f
      i32.const 1
      if
        i32.const 0
        drop
      end
    )
    (start $f)))

(assert_fuel 4
  (module
    (func $f
      i32.const 1
      if
        i32.const 0
        drop
      else
        call $f
      end
    )
    (start $f)))

(assert_fuel 4
  (module
    (func $f
      i32.const 0
      if
        call $f
      else
        i32.const 0
        drop
      end
    )
    (start $f)))

(assert_fuel 3
  (module
    (func $f
      block
        i32.const 1
        br_if 0
        i32.const 0
        drop
      end
    )
    (start $f)))

(assert_fuel 4
  (module
    (func $f
      block
        i32.const 0
        br_if 0
        i32.const 0
        drop
      end
    )
    (start $f)))

;; count code before unreachable
(assert_fuel 2
  (module
    (func $f
      i32.const 0
      unreachable
    )
    (start $f)))

;; count code before return
(assert_fuel 2
  (module
    (func $f
      i32.const 0
      return
    )
    (start $f)))

;; cross-function fuel works
(assert_fuel 3
  (module
    (func $f
      call $other
    )
    (func $other)
    (start $f)))
(assert_fuel 5
  (module
    (func $f
      i32.const 0
      call $other
      i32.const 0
      drop
    )
    (func $other (param i32))
    (start $f)))
(assert_fuel 4
  (module
    (func $f
      call $other
      drop
    )
    (func $other (result i32)
      i32.const 0
    )
    (start $f)))
(assert_fuel 4
  (module
    (func $f
      i32.const 0
      call_indirect
    )
    (func $other)
    (table funcref (elem $other))
    (start $f)))

;; loops!
(assert_fuel 1
  (module
    (func $f
      loop
      end
    )
    (start $f)))
(assert_fuel 53 ;; 5 loop instructions, 10 iterations, 2 header instrs, 1 func
  (module
    (func $f
      (local i32)
      i32.const 10
      local.set 0

      loop
        local.get 0
        i32.const 1
        i32.sub
        local.tee 0
        br_if 0
      end
    )
    (start $f)))

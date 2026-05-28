(assert_fuel 0 (module))

(assert_fuel 3
  (module
    (func $f)
    (start $f)))

(assert_fuel 4
  (module
    (func $f
      i32.const 0
      drop
    )
    (start $f)))

(assert_fuel 3
  (module
    (func $f
      block
      end
    )
    (start $f)))

(assert_fuel 3
  (module
    (func $f
      unreachable
    )
    (start $f)))

(assert_fuel 9
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

(assert_fuel 3
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

(assert_fuel 5
  (module
    (func $f
      i32.const 0
      if
        call $f
      end
    )
    (start $f)))

(assert_fuel 6
  (module
    (func $f
      i32.const 1
      if
        i32.const 0
        drop
      end
    )
    (start $f)))

(assert_fuel 6
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

(assert_fuel 6
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

(assert_fuel 5
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

(assert_fuel 6
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
(assert_fuel 4
  (module
    (func $f
      i32.const 0
      unreachable
    )
    (start $f)))

;; count code before return
(assert_fuel 4
  (module
    (func $f
      i32.const 0
      return
    )
    (start $f)))

;; cross-function fuel works
(assert_fuel 5
  (module
    (func $f
      call $other
    )
    (func $other)
    (start $f)))
(assert_fuel 7
  (module
    (func $f
      i32.const 0
      call $other
      i32.const 0
      drop
    )
    (func $other (param i32))
    (start $f)))
(assert_fuel 6
  (module
    (func $f
      call $other
      drop
    )
    (func $other (result i32)
      i32.const 0
    )
    (start $f)))
(assert_fuel 6
  (module
    (func $f
      i32.const 0
      call_indirect
    )
    (func $other)
    (table funcref (elem $other))
    (start $f)))

;; loops!
(assert_fuel 3
  (module
    (func $f
      loop
      end
    )
    (start $f)))
(assert_fuel 55 ;; 5 loop instructions, 10 iterations, 2 header instrs, 1 func
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

(assert_fuel 107
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 100
      memory.copy
    )
    (start $f)))

(assert_fuel 107
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 100
      memory.fill
    )
    (start $f)))

(assert_fuel 27
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 20
      memory.init $d
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 107
  (module
    (table 100 funcref)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 100
      table.copy
    )
    (start $f)))

(assert_fuel 107
  (module
    (table 100 funcref)
    (func $f
      i32.const 0
      ref.null func
      i32.const 100
      table.fill
    )
    (start $f)))

(assert_fuel 106
  (module
    (table 0 funcref)
    (func $f
      ref.null func
      i32.const 100
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 27
  (module
    (table 20 funcref)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 20
      table.init $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 211
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 100)))
    (func $f
      global.get $a
      i32.const 0
      global.get $a
      i32.const 0
      i32.const 100
      array.copy $a $a
    )
    (start $f)))

(assert_fuel 210
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 100)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 100
      array.fill $a
    )
    (start $f)))

(assert_fuel 26
  (module
    (type $a (array (mut i8)))
    (func $f
      i32.const 0
      i32.const 20
      array.new_data $a $d
      drop
    )
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    (start $f)))

(assert_fuel 130
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 100)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 20
      array.init_data $a $d
    )
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    (start $f)))

(assert_fuel 26
  (module
    (type $a (array (mut funcref)))
    (func $f
      i32.const 0
      i32.const 20
      array.new_elem $a $e
      drop
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 130
  (module
    (type $a (array (mut funcref)))
    (global $a (ref $a) (array.new_default $a (i32.const 100)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 20
      array.init_elem $a $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 105
  (module
    (type $a (array (mut funcref)))
    (func $f
      i32.const 100
      array.new_default $a
      drop
    )
    (start $f)))

(assert_fuel 106
  (module
    (type $a (array (mut funcref)))
    (func $f
      ref.null func
      i32.const 100
      array.new $a
      drop
    )
    (start $f)))

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

;; A small constant size (size * cost <= 128, the `SMALL_BULK_OP_COST`
;; threshold) charges the size-proportional "variable" fuel up front, before
;; the op runs.
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

;; A large constant size (> 128) instead defers that variable fuel until
;; after the op has run.
(assert_fuel 207
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 200
      memory.copy
    )
    (start $f)))

(assert_fuel 207
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 200
      memory.fill
    )
    (start $f)))

(assert_fuel 207
  (module
    (memory 1)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 200
      memory.init $d
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 207
  (module
    (table 200 funcref)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 200
      table.copy
    )
    (start $f)))

(assert_fuel 207
  (module
    (table 200 funcref)
    (func $f
      i32.const 0
      ref.null func
      i32.const 200
      table.fill
    )
    (start $f)))

(assert_fuel 207
  (module
    (table 200 funcref)
    (func $f
      i32.const 0
      i32.const 0
      i32.const 200
      table.init $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 411
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 200)))
    (func $f
      global.get $a
      i32.const 0
      global.get $a
      i32.const 0
      i32.const 200
      array.copy $a $a
    )
    (start $f)))

(assert_fuel 410
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 200)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.fill $a
    )
    (start $f)))

(assert_fuel 206
  (module
    (type $a (array (mut i8)))
    (func $f
      i32.const 0
      i32.const 200
      array.new_data $a $d
      drop
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 410
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 200)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.init_data $a $d
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 206
  (module
    (type $a (array (mut funcref)))
    (func $f
      i32.const 0
      i32.const 200
      array.new_elem $a $e
      drop
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 410
  (module
    (type $a (array (mut funcref)))
    (global $a (ref $a) (array.new_default $a (i32.const 200)))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.init_elem $a $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 205
  (module
    (type $a (array (mut funcref)))
    (func $f
      i32.const 200
      array.new_default $a
      drop
    )
    (start $f)))

(assert_fuel 206
  (module
    (type $a (array (mut funcref)))
    (func $f
      ref.null func
      i32.const 200
      array.new $a
      drop
    )
    (start $f)))

;; A dynamic (runtime) size likewise defers the variable fuel until after
;; the op has run, whatever its magnitude.
(assert_fuel 57
  (module
    (memory 1)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      i32.const 0
      global.get $n
      memory.copy
    )
    (start $f)))

(assert_fuel 57
  (module
    (memory 1)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      i32.const 0
      global.get $n
      memory.fill
    )
    (start $f)))

(assert_fuel 57
  (module
    (memory 1)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      i32.const 0
      global.get $n
      memory.init $d
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 57
  (module
    (table 50 funcref)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      i32.const 0
      global.get $n
      table.copy
    )
    (start $f)))

(assert_fuel 57
  (module
    (table 50 funcref)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      ref.null func
      global.get $n
      table.fill
    )
    (start $f)))

(assert_fuel 57
  (module
    (table 50 funcref)
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      i32.const 0
      global.get $n
      table.init $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 111
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 50)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      global.get $a
      i32.const 0
      global.get $a
      i32.const 0
      global.get $n
      array.copy $a $a
    )
    (start $f)))

(assert_fuel 110
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 50)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      global.get $n
      array.fill $a
    )
    (start $f)))

(assert_fuel 56
  (module
    (type $a (array (mut i8)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      global.get $n
      array.new_data $a $d
      drop
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 110
  (module
    (type $a (array (mut i8)))
    (global $a (ref $a) (array.new_default $a (i32.const 50)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      global.get $n
      array.init_data $a $d
    )
    (start $f)
    (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")))

(assert_fuel 56
  (module
    (type $a (array (mut funcref)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      i32.const 0
      global.get $n
      array.new_elem $a $e
      drop
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 110
  (module
    (type $a (array (mut funcref)))
    (global $a (ref $a) (array.new_default $a (i32.const 50)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      global.get $a
      i32.const 0
      i32.const 0
      global.get $n
      array.init_elem $a $e
    )
    (start $f)
    (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f)))

(assert_fuel 55
  (module
    (type $a (array (mut funcref)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      global.get $n
      array.new_default $a
      drop
    )
    (start $f)))

(assert_fuel 56
  (module
    (type $a (array (mut funcref)))
    (global $n (mut i32) (i32.const 50))
    (func $f
      ref.null func
      global.get $n
      array.new $a
      drop
    )
    (start $f)))

;; Growing a memory or table charges a small constant size's variable fuel up
;; front, but defers a large or dynamic size's variable fuel until the grow has
;; succeeded. 
;;
;; The following tests cover the 12 cases of table/memory x small/large/dynamic
;; size x success/failure.
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

(assert_fuel 106
  (module
    (table 0 0 funcref)
    (func $f
      ref.null func
      i32.const 100
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 206
  (module
    (table 0 funcref)
    (func $f
      ref.null func
      i32.const 200
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 6
  (module
    (table 0 0 funcref)
    (func $f
      ref.null func
      i32.const 200
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 56
  (module
    (table 0 funcref)
    (global $d (mut i32) (i32.const 50))
    (func $f
      ref.null func
      global.get $d
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 6
  (module
    (table 0 0 funcref)
    (global $d (mut i32) (i32.const 50))
    (func $f
      ref.null func
      global.get $d
      table.grow
      drop
    )
    (start $f)))

(assert_fuel 105
  (module
    (memory 1)
    (func $f
      i32.const 100
      memory.grow
      drop
    )
    (start $f)))

(assert_fuel 105
  (module
    (memory 0 0)
    (func $f
      i32.const 100
      memory.grow
      drop
    )
    (start $f)))

(assert_fuel 205
  (module
    (memory 1)
    (func $f
      i32.const 200
      memory.grow
      drop
    )
    (start $f)))

(assert_fuel 5
  (module
    (memory 0 0)
    (func $f
      i32.const 200
      memory.grow
      drop
    )
    (start $f)))

(assert_fuel 55
  (module
    (memory 1)
    (global $d (mut i32) (i32.const 50))
    (func $f
      global.get $d
      memory.grow
      drop
    )
    (start $f)))

(assert_fuel 5
  (module
    (memory 0 0)
    (global $d (mut i32) (i32.const 50))
    (func $f
      global.get $d
      memory.grow
      drop
    )
    (start $f)))

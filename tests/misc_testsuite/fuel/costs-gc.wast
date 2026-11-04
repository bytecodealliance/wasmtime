;;! bulk_memory = true
;;! gc = true
;;! function_references = true
;;! exceptions = true
;;! flags = "-W fuel=1000000000"

;; Like `costs.wast`, but for the tests that need GC or typed function
;; references. Each test sets the store's fuel to 1000, instantiates a module
;; (usually running its start function), and checks exactly how much fuel is
;; left.
(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (import "wasmtime" "get-fuel" (func $get (result i64)))
  (export "set-fuel" (func $set))
  (export "get-fuel" (func $get)))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $t (func))
  (func $f
    ref.func $other
    call_ref $t
  )
  (func $other)
  (elem declare func $other)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $t (func))
  (func $f
    ref.func $other
    call_ref $t
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
  )
  (elem declare func $other)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 991))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $t (func))
  (tag $e)
  (func $f
    block $catch
      try_table (catch $e $catch)
        ref.func $other
        call_ref $t
      end
    end
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
    throw $e
  )
  (elem declare func $other)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 989))

;; Array bulk ops follow the same rules as the memory and table ops in
;; `costs.wast`. A small constant size (size * cost <= 128) charges the
;; size-proportional "variable" fuel up front.
(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 789))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 790))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut i8)))
  (func $f
    i32.const 0
    i32.const 20
    array.new_data $a $d
    drop
  )
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 974))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 870))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    i32.const 0
    i32.const 20
    array.new_elem $a $e
    drop
  )
  (start $f)
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 974))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 870))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    i32.const 100
    array.new_default $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 895))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    ref.null func
    i32.const 100
    array.new $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 894))

;; A large constant size (> 128) defers the variable fuel until after the op
;; has run.
(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 589))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 590))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut i8)))
  (func $f
    i32.const 0
    i32.const 200
    array.new_data $a $d
    drop
  )
  (start $f)
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 794))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 590))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    i32.const 0
    i32.const 200
    array.new_elem $a $e
    drop
  )
  (start $f)
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 794))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 590))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    i32.const 200
    array.new_default $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 795))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (func $f
    ref.null func
    i32.const 200
    array.new $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 794))

;; A dynamic (runtime) size likewise defers the variable fuel.
(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 889))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 890))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 944))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 890))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 944))

(invoke $Fuel "set-fuel" (i64.const 1000))
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
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 890))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (global $n (mut i32) (i32.const 50))
  (func $f
    global.get $n
    array.new_default $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 945))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (type $a (array (mut funcref)))
  (global $n (mut i32) (i32.const 50))
  (func $f
    ref.null func
    global.get $n
    array.new $a
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 944))

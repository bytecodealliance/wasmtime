;;! stack_switching = true
;; Tests using support for n-ary continuations
;; Uses a function as continuation that has 3 param and 5 return values
;; Uses tag that has 4 elements of payloads
;; All of these mix i32 and i64 values


(module
  (type $unit_to_unit (func))
  (type $unit_to_int (func (result i32)))
  (type $int_to_unit (func (param i32)))
  (type $int_to_int (func (param i32) (result i32)))


  ;; type of function f
  (type $f_t
        (func
         (param i64 i32 i64)
         (result i32 i64 i32 i64 i32)))
  (type $f_ct (cont $f_t))


  ;; type of the resumption we expect to see in our handler for $e
  (type $res_ft
        (func
         (param i64 i32 i64 i32)
         (result i32 i64 i32 i64 i32)))
  (type $res (cont $res_ft))

  ;; This is 10^10, which exceeds what can be stored in any 32 bit type
  (global $big i64 (i64.const 10_000_000_000))

  (tag $e
       (param i32 i64 i32 i64)
       (result i64 i32 i64 i32))



  (func $f (export "f")
        (param $x i64) (param $y i32) (param $z i64)
        (result i32 i64 i32 i64 i32)

    ;; value to stay on the stack as part of return values
    (i32.const 1)

    ;; values to be passed to $e
    (i32.const 10)
    (local.get $z)
    (local.get $y)
    (local.get $x)
    (suspend $e)
  )

  (func $test (export "test") (result i32 i64)
    (local $i64_acc i64)
    (local $i32_acc i32)
    (local $k (ref $res))
    (local.set $i64_acc (i64.const 0))
    (local.set $i32_acc (i32.const 0))


    (block $on_e (result i32 i64 i32 i64 (ref $res)) ;; lets call these values v1 v2 v3 v4 k
      (global.get $big)
      (i32.const 100)
      (i64.mul (global.get $big) (i64.const 10))
      (resume $f_ct (on $e $on_e) (cont.new $f_ct (ref.func $f)))
      (unreachable))
    ;; after on_e
    (local.set $k)
    (i32.const 1000)
    ;; We pass v2 v3 v4 123 as arguments to the continuation, leaving v1 on the stack
    (resume $res (local.get $k))
    ;; We now have v1 and the five return values of $f on the stack, i.e. [i32 i32 i64 i32 i64 i32]
    ;; Lets accumulate them
    ;;

    (local.set $i32_acc (i32.add (local.get $i32_acc)))
    (local.set $i64_acc (i64.add (local.get $i64_acc)))
    (local.set $i32_acc (i32.add (local.get $i32_acc)))
    (local.set $i64_acc (i64.add (local.get $i64_acc)))
    (local.set $i32_acc (i32.add (local.get $i32_acc)))
    (local.set $i32_acc (i32.add (local.get $i32_acc)))

    ;; ;; Set up return values
    (local.get $i32_acc)
    (local.get $i64_acc))

)

(assert_return (invoke "test") (i32.const 1111) (i64.const 110_000_000_000))

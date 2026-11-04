;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true
;;! consume_fuel = true

;; Fuel used inside a continuation is accounted for.
(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (import "wasmtime" "get-fuel" (func $get (result i64)))
  (export "set-fuel" (func $set))
  (export "get-fuel" (func $get)))

(module
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $t)

  (func $burn
    (local $i i32)
    (local.set $i (i32.const 50))
    (loop $l
      (suspend $t)
      (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
      (br_if $l)))

  (func (export "run")
    (local $k (ref null $ct))
    (local.set $k (cont.new $ct (ref.func $burn)))
    (loop $l
      (block $on_t (result (ref $ct))
        (resume $ct (on $t $on_t) (local.get $k))
        return)
      (local.set $k)
      (br $l)))

  (elem declare func $burn))

;; Both the 50 iterations in `$burn` and the 50 in `run` use fuel.
(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_return (invoke "run"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 9491))

;; Enough fuel for `run`'s loop alone, but not for `$burn` as well.
(invoke $Fuel "set-fuel" (i64.const 300))
(assert_trap (invoke "run") "all fuel consumed")

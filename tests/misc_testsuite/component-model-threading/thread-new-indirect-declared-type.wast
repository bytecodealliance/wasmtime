;;! component_model_async = true
;;! component_model_threading = true
;;! reference_types = true
;;! function_references = true
;;! gc = true

;; `thread.new-indirect` traps if the start function's type isn't exactly the
;; core function type named by the canonical built-in, using core wasm's type
;; canonicalization rules. Validation only requires that type to be
;; structurally `[i32] -> []`, so it may be non-final, have a supertype, or be
;; part of a larger rec group.
;;
;; Each `spawn-*` export below uses a `thread.new-indirect` with a different
;; declared type, and spawns the start function at the given table index. Only
;; the function whose type is the same as the declared type should be accepted.

(component definition $C
  (core module $libc
    (table (export "__indirect_function_table") 7 funcref))
  (core instance $libc (instantiate $libc))
  (alias core export $libc "__indirect_function_table" (core table $table))

  (core type $final (func (param i32)))
  (core type $non-final (sub (func (param i32))))
  (core rec (type $rec (func (param i32))) (type (struct)))
  (core type $super (sub (func (param i32))))
  (core type $sub (sub $super (func (param i32))))
  (core type $s (struct))
  (core rec
    (type $rec-outer (func (param i32)))
    (type (struct (field (ref $s)))))

  (core func $thread.new-indirect-final
    (canon thread.new-indirect $final (core table $table)))
  (core func $thread.new-indirect-non-final
    (canon thread.new-indirect $non-final (core table $table)))
  (core func $thread.new-indirect-rec
    (canon thread.new-indirect $rec (core table $table)))
  (core func $thread.new-indirect-sub
    (canon thread.new-indirect $sub (core table $table)))
  (core func $thread.new-indirect-rec-outer
    (canon thread.new-indirect $rec-outer (core table $table)))
  (core func $thread.yield-then-resume (canon thread.yield-then-resume))

  (core module $m
    (type $final (func (param i32)))
    (type $non-final (sub (func (param i32))))
    (rec (type $rec (func (param i32))) (type (struct)))
    (rec (type $rec-other (func (param i32))) (type (array i8)))
    (type $super (sub (func (param i32))))
    (type $sub (sub $super (func (param i32))))
    (type $s (struct))
    (rec
      (type $rec-outer (func (param i32)))
      (type (struct (field (ref $s)))))

    (import "" "thread.new-indirect-final"
      (func $thread.new-indirect-final (param i32 i32) (result i32)))
    (import "" "thread.new-indirect-non-final"
      (func $thread.new-indirect-non-final (param i32 i32) (result i32)))
    (import "" "thread.new-indirect-rec"
      (func $thread.new-indirect-rec (param i32 i32) (result i32)))
    (import "" "thread.new-indirect-sub"
      (func $thread.new-indirect-sub (param i32 i32) (result i32)))
    (import "" "thread.new-indirect-rec-outer"
      (func $thread.new-indirect-rec-outer (param i32 i32) (result i32)))
    (import "" "thread.yield-then-resume"
      (func $thread.yield-then-resume (param i32) (result i32)))
    (import "libc" "__indirect_function_table" (table $table 7 funcref))

    (global $ran (mut i32) (i32.const 0))

    (func $start-final (type $final) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-non-final (type $non-final) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-rec (type $rec) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-rec-other (type $rec-other) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-super (type $super) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-sub (type $sub) (param i32)
      (global.set $ran (local.get 0)))
    (func $start-rec-outer (type $rec-outer) (param i32)
      (global.set $ran (local.get 0)))
    (elem (table $table) (i32.const 0) func
      $start-final       ;; 0
      $start-non-final   ;; 1
      $start-rec         ;; 2
      $start-rec-other   ;; 3
      $start-super       ;; 4
      $start-sub         ;; 5
      $start-rec-outer)  ;; 6

    ;; Runs the new thread to completion and returns what it stored in `$ran`.
    (func $run (param $thread i32) (result i32)
      (global.set $ran (i32.const 0))
      (drop (call $thread.yield-then-resume (local.get $thread)))
      (global.get $ran))

    (func (export "spawn-final") (param i32) (result i32)
      (call $run
        (call $thread.new-indirect-final (local.get 0) (i32.const 42))))
    (func (export "spawn-non-final") (param i32) (result i32)
      (call $run
        (call $thread.new-indirect-non-final (local.get 0) (i32.const 42))))
    (func (export "spawn-rec") (param i32) (result i32)
      (call $run
        (call $thread.new-indirect-rec (local.get 0) (i32.const 42))))
    (func (export "spawn-sub") (param i32) (result i32)
      (call $run
        (call $thread.new-indirect-sub (local.get 0) (i32.const 42))))
    (func (export "spawn-rec-outer") (param i32) (result i32)
      (call $run
        (call $thread.new-indirect-rec-outer (local.get 0) (i32.const 42)))))

  (core instance $i (instantiate $m
    (with "" (instance
      (export "thread.new-indirect-final" (func $thread.new-indirect-final))
      (export "thread.new-indirect-non-final"
        (func $thread.new-indirect-non-final))
      (export "thread.new-indirect-rec" (func $thread.new-indirect-rec))
      (export "thread.new-indirect-sub" (func $thread.new-indirect-sub))
      (export "thread.new-indirect-rec-outer"
        (func $thread.new-indirect-rec-outer))
      (export "thread.yield-then-resume" (func $thread.yield-then-resume))))
    (with "libc" (instance $libc))))

  (func (export "spawn-final") async (param "i" u32) (result u32)
    (canon lift (core func $i "spawn-final")))
  (func (export "spawn-non-final") async (param "i" u32) (result u32)
    (canon lift (core func $i "spawn-non-final")))
  (func (export "spawn-rec") async (param "i" u32) (result u32)
    (canon lift (core func $i "spawn-rec")))
  (func (export "spawn-sub") async (param "i" u32) (result u32)
    (canon lift (core func $i "spawn-sub")))
  (func (export "spawn-rec-outer") async (param "i" u32) (result u32)
    (canon lift (core func $i "spawn-rec-outer")))
)

;; A final `(func (param i32))` matches only itself.
(component instance $C $C)
(assert_return (invoke "spawn-final" (u32.const 0)) (u32.const 42))
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 1)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 2)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 3)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 4)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 5)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-final" (u32.const 6)) "does not match")

;; A non-final type with no supertype matches the identical `$super`, but not a
;; subtype of it.
(component instance $C $C)
(assert_trap (invoke "spawn-non-final" (u32.const 0)) "does not match")
(component instance $C $C)
(assert_return (invoke "spawn-non-final" (u32.const 1)) (u32.const 42))
(component instance $C $C)
(assert_trap (invoke "spawn-non-final" (u32.const 2)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-non-final" (u32.const 3)) "does not match")
(component instance $C $C)
(assert_return (invoke "spawn-non-final" (u32.const 4)) (u32.const 42))
(component instance $C $C)
(assert_trap (invoke "spawn-non-final" (u32.const 5)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-non-final" (u32.const 6)) "does not match")

;; A rec group member matches the same member of a structurally identical rec
;; group, but not one of a different rec group.
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 0)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 1)) "does not match")
(component instance $C $C)
(assert_return (invoke "spawn-rec" (u32.const 2)) (u32.const 42))
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 3)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 4)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 5)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec" (u32.const 6)) "does not match")

;; A type with a supertype matches only the same subtype.
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 0)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 1)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 2)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 3)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 4)) "does not match")
(component instance $C $C)
(assert_return (invoke "spawn-sub" (u32.const 5)) (u32.const 42))
(component instance $C $C)
(assert_trap (invoke "spawn-sub" (u32.const 6)) "does not match")

;; A rec group member whose group refers to a type outside of the group.
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 0)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 1)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 2)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 3)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 4)) "does not match")
(component instance $C $C)
(assert_trap (invoke "spawn-rec-outer" (u32.const 5)) "does not match")
(component instance $C $C)
(assert_return (invoke "spawn-rec-outer" (u32.const 6)) (u32.const 42))

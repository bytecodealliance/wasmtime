;;! gc = true
;;! flags = """
;;!   -C inlining=y
;;!   -C cranelift-wasmtime-inlining-sum-size-threshold=4294967295
;;! """

;; Test for #13538
;;
;; With inlining forced on for every call that can be inlined, `g` reads an
;; entity imported from another instance, calls that instance's `writeit` to
;; change it, and reads it again. The second read must observe the write
;; (`123`), not the stale value from the first read (`0`).

;; Memories.
(component
  (core module $M
    (memory (export "mem") 1)
    (func (export "writeit") (param i32 i32)
      (i32.store (local.get 0) (local.get 1))))
  (core instance $m (instantiate $M))
  (core module $N
    (import "" "mem" (memory 1))
    (import "" "writeit" (func $writeit (param i32 i32)))
    (func (export "g") (result i32)
      (i32.load (i32.const 0))
      drop
      (call $writeit (i32.const 0) (i32.const 123))
      (i32.load (i32.const 0))))
  (core instance $n (instantiate $N
    (with "" (instance
      (export "mem" (memory $m "mem"))
      (export "writeit" (func $m "writeit"))))))
  (func (export "g") (result u32) (canon lift (core func $n "g"))))

(assert_return (invoke "g") (u32.const 123))

;; Globals.
(component
  (core module $M
    (global (export "glob") (mut i32) (i32.const 0))
    (func (export "writeit") (param i32)
      (global.set 0 (local.get 0))))
  (core instance $m (instantiate $M))
  (core module $N
    (import "" "glob" (global $glob (mut i32)))
    (import "" "writeit" (func $writeit (param i32)))
    (func (export "g") (result i32)
      (global.get $glob)
      drop
      (call $writeit (i32.const 123))
      (global.get $glob)))
  (core instance $n (instantiate $N
    (with "" (instance
      (export "glob" (global $m "glob"))
      (export "writeit" (func $m "writeit"))))))
  (func (export "g") (result u32) (canon lift (core func $n "g"))))

(assert_return (invoke "g") (u32.const 123))

;; Tables.
(component
  (core module $M
    (table (export "tab") 1 1 (ref i31) (ref.i31 (i32.const 0)))
    (func (export "writeit") (param i32)
      (table.set (i32.const 0) (ref.i31 (local.get 0)))))
  (core instance $m (instantiate $M))
  (core module $N
    (import "" "tab" (table 1 1 (ref i31)))
    (import "" "writeit" (func $writeit (param i32)))
    (func (export "g") (result i32)
      (drop (table.get (i32.const 0)))
      (call $writeit (i32.const 123))
      (i31.get_u (table.get (i32.const 0)))))
  (core instance $n (instantiate $N
    (with "" (instance
      (export "tab" (table $m "tab"))
      (export "writeit" (func $m "writeit"))))))
  (func (export "g") (result u32) (canon lift (core func $n "g"))))

(assert_return (invoke "g") (u32.const 123))

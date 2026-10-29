;;! multi_memory = true

;; Exercise filling the store's `lazy_lenders` buffer, which should
;; (transparently to the guest) trigger the `VMDeferredThread` to be forced.

(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))

    (core module $m
      (memory (export "mem") 1)

      ;; Bump allocator, keeping its cursor at address 0.
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (local.set $ret (i32.load (i32.const 0)))
        (if (i32.lt_u (local.get $ret) (i32.const 1024))
          (then (local.set $ret (i32.const 1024))))
        (i32.store (i32.const 0) (i32.add (local.get $ret) (local.get 3)))
        (local.get $ret))

      (func (export "consume") (param $ptr i32) (param $len i32) (result i32)
        (local.get $len))
    )
    (core instance $i (instantiate $m))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor)))
    (func (export "consume") (param "xs" (list (borrow $t))) (result u32)
      (canon lift (core func $i "consume")
        (memory (core memory $i "mem"))
        (realloc (core func $i "realloc"))))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "[constructor]t" (func (param "x" u32) (result (own $t))))
      (export "consume" (func (param "xs" (list (borrow $t))) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "[constructor]t" (func $ctor))
    (alias export $i "consume" (func $consume))

    (core module $mm
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (local.set $ret (i32.load (i32.const 0)))
        (if (i32.lt_u (local.get $ret) (i32.const 1024))
          (then (local.set $ret (i32.const 1024))))
        (i32.store (i32.const 0) (i32.add (local.get $ret) (local.get 3)))
        (local.get $ret))
    )
    (core instance $mi (instantiate $mm))

    (core func $ctor' (canon lower (func $ctor)
      (memory (core memory $mi "mem"))
      (realloc (core func $mi "realloc"))))
    (core func $consume' (canon lower (func $consume)
      (memory (core memory $mi "mem"))
      (realloc (core func $mi "realloc"))))
    (core func $drop (canon resource.drop $t))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "consume" (func $consume (param i32 i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (import "" "mem" (memory 1))

      ;; Lend one handle `$n` times in a single call, then drop it. The drop
      ;; only succeeds if every one of those lends was undone.
      (func (export "run") (param $n i32) (result i32)
        (local $h i32) (local $i i32) (local $v i32)
        (local.set $h (call $ctor (i32.const 9)))
        (block $done
          (loop $loop
            (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
            (i32.store
              (i32.add (i32.const 128) (i32.mul (local.get $i) (i32.const 4)))
              (local.get $h))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $loop)))
        (local.set $v (call $consume (i32.const 128) (local.get $n)))
        (call $drop (local.get $h))
        (local.get $v))
    )
    (core instance $i2 (instantiate $m (with "" (instance
      (export "ctor" (func $ctor'))
      (export "consume" (func $consume'))
      (export "drop" (func $drop))
      (export "mem" (memory $mi "mem"))))))

    (func (export "run") (param "n" u32) (result u32)
      (canon lift (core func $i2 "run")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

;; Comfortably within the buffer, so every lend is recorded lazily.
(assert_return (invoke "run" (u32.const 64)) (u32.const 64))

;; Past the buffer's capacity, so the lift switches to the forced path partway
;; through and the undo has to cover both halves.
(assert_return (invoke "run" (u32.const 1000)) (u32.const 1000))

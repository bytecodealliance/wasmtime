;;! gc = true
;;! exceptions = true

;; An exception object's two tag words each get their own alias region, which
;; must be distinct from each other and from every payload's region.

(component
  (core module $A
    ;; Two tags with identical payload signatures, so telling them apart at
    ;; the catch site depends entirely on the tag words.
    (tag $t0 (export "t0") (param i32 i64))
    (tag $t1 (export "t1") (param i32 i64))

    (func (export "throw0") (param i32 i64)
      (throw $t0 (local.get 0) (local.get 1)))
    (func (export "throw1") (param i32 i64)
      (throw $t1 (local.get 0) (local.get 1)))
  )

  (core instance $a (instantiate $A))

  (core module $B
    ;; Shift this module's type index space relative to $A's.
    (type $unused0 (struct (field f64)))
    (type $unused1 (array (mut i64)))

    (import "" "t0" (tag $t0 (param i32 i64)))
    (import "" "t1" (tag $t1 (param i32 i64)))
    (import "" "throw0" (func $throw0 (param i32 i64)))
    (import "" "throw1" (func $throw1 (param i32 i64)))

    (func $catch (param $throw-t1 i32) (param $p1-expected i64) (result i32)
      (local $which i32)
      (local $p0 i32)
      (local $p1 i64)
      (block $done
        (block $as-t0 (result i32 i64)
          (block $as-t1 (result i32 i64)
            (try_table (catch $t0 $as-t0) (catch $t1 $as-t1)
              (if (local.get $throw-t1)
                (then (call $throw1 (i32.const 0x22) (local.get $p1-expected)))
                (else (call $throw0 (i32.const 0x11) (local.get $p1-expected)))))
            (return (i32.const 0)))

          ;; Caught as `$t1`.
          (local.set $p1)
          (local.set $p0)
          (local.set $which (i32.const 2))
          (br $done))

        ;; Caught as `$t0`.
        (local.set $p1)
        (local.set $p0)
        (local.set $which (i32.const 1)))

      (if (i64.ne (local.get $p1) (local.get $p1-expected))
        (then (return (i32.const 0))))
      (i32.add (i32.mul (local.get $which) (i32.const 0x1000))
               (local.get $p0)))

    (func (export "catch-0") (result i32)
      (call $catch (i32.const 0) (i64.const 0x2222_3333_4444_5555)))

    (func (export "catch-1") (result i32)
      (call $catch (i32.const 1) (i64.const 0x6666_7777_8888_9999)))
  )

  (core instance $b (instantiate $B
    (with "" (instance
      (export "t0" (tag $a "t0"))
      (export "t1" (tag $a "t1"))
      (export "throw0" (func $a "throw0"))
      (export "throw1" (func $a "throw1"))))))

  (func (export "catch-0") (result u32)
    (canon lift (core func $b "catch-0")))
  (func (export "catch-1") (result u32)
    (canon lift (core func $b "catch-1")))
)

(assert_return (invoke "catch-0") (u32.const 0x1011))
(assert_return (invoke "catch-1") (u32.const 0x2022))

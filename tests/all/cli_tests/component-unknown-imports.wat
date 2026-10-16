;; A component whose imports are all undefined, to exercise
;; `-W unknown-imports-default`. Without that flag it cannot be instantiated.
;;
;; Two entry points:
;;
;; * `defaults`, invoked with `--invoke`, returns the defaults for every
;;   interface type that has one, so they can be checked against the printed
;;   output.
;;
;; * `wasi:cli/run`, which exercises the resource handling that is not visible
;;   in a return value: a `borrow` argument must be dropped by the stub, or the
;;   guest traps with "borrow handles still remain at the end of the call".
(component
  (type (;0;) (record (field "a" u32) (field "b" string)))
  (import "rec" (type $rec (eq 0)))
  (type (;2;) (variant (case "x" s64) (case "y")))
  (import "var" (type $var (eq 2)))
  (type (;4;) (flags "p" "q"))
  (import "fl" (type $fl (eq 4)))
  (type (;6;) (enum "e1" "e2"))
  (import "en" (type $en (eq 6)))

  (type $all (tuple u32 float64 string bool char (list u8)
                    (option u64) (result u32 (error string))
                    $rec $var $fl $en))

  (import "unknown:pkg/iface" (instance $iface
    (export "r" (type $r (sub resource)))
    (export "defaults" (func (result $all)))
    (export "make" (func (result (own $r))))
    (export "borrow-it" (func (param "self" (borrow $r)) (result u32)))
    (export "consume" (func (param "a" (own $r)) (result u32)))
    ;; Stubbed out as well, though nothing here calls them.
    (export "nested" (instance (export "baz" (func))))
  ))
  (alias export $iface "r" (type $r))
  (alias export $iface "defaults" (func $defaults))
  (alias export $iface "make" (func $make))
  (alias export $iface "borrow-it" (func $borrow-it))
  (alias export $iface "consume" (func $consume))

  (core module $libc
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
  )
  (core instance $libc (instantiate $libc))
  (alias core export $libc "mem" (core memory $mem))
  (alias core export $libc "realloc" (core func $realloc))

  (core func $defaults-l
    (canon lower (func $defaults) (memory $mem) (realloc (core func $realloc))))
  (core func $make-l (canon lower (func $make)))
  (core func $borrow-l (canon lower (func $borrow-it)))
  (core func $consume-l (canon lower (func $consume)))
  (core func $drop-r (canon resource.drop $r))

  (core module $m
    (import "" "defaults" (func $defaults (param i32)))
    (import "" "make" (func $make (result i32)))
    (import "" "borrow" (func $borrow (param i32) (result i32)))
    (import "" "consume" (func $consume (param i32) (result i32)))
    (import "" "drop" (func $drop (param i32)))

    (func (export "defaults") (result i32)
      (call $defaults (i32.const 256))
      i32.const 256)

    (func (export "run") (result i32)
      (local $handle i32)

      ;; A resource-returning import yields a usable handle, and passing it
      ;; back as a `borrow` must not leave the borrow dangling on return.
      (local.set $handle (call $make))
      (if (i32.ne (call $borrow (local.get $handle)) (i32.const 0))
        (then unreachable))
      (call $drop (local.get $handle))

      ;; An `own` argument is consumed by the stub; do it twice to be sure the
      ;; handle was really released.
      (if (i32.ne (call $consume (call $make)) (i32.const 0)) (then unreachable))
      (if (i32.ne (call $consume (call $make)) (i32.const 0)) (then unreachable))

      i32.const 0)
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "defaults" (func $defaults-l))
    (export "make" (func $make-l))
    (export "borrow" (func $borrow-l))
    (export "consume" (func $consume-l))
    (export "drop" (func $drop-r))))))

  (func (export "defaults") (result $all)
    (canon lift (core func $i "defaults") (memory $mem) (realloc (core func $realloc))))

  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)

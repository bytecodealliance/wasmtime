;;! component_model_memory64 = true
;;! memory64 = true
;;! multi_memory = true
;;! bulk_memory = true

;; Test latin1+utf16 strings passed between components whose memories have
;; different address types.

;; i64 caller -> i32 callee: utf16-tagged input whose code points all fit in
;; latin1 must arrive deflated to latin1; wide content stays utf16.
(component
  (component $C
    (core module $M
      (memory (export "mem") 1)
      (global $next (mut i32) (i32.const 1024))
      (func (export "realloc") (param $old i32) (param $osize i32) (param $align i32) (param $nsize i32) (result i32)
        (local $r i32)
        ;; shrinking realloc: keep the (8-aligned) allocation in place
        (if (i32.and (i32.ne (local.get $old) (i32.const 0))
                     (i32.le_u (local.get $nsize) (local.get $osize)))
          (then (return (local.get $old))))
        ;; fresh 8-aligned bump allocation
        (global.set $next (i32.and (i32.add (global.get $next) (i32.const 7)) (i32.const -8)))
        (local.set $r (global.get $next))
        (global.set $next (i32.add (global.get $next) (local.get $nsize)))
        ;; growing realloc must preserve the old contents
        (if (i32.ne (local.get $old) (i32.const 0))
          (then (memory.copy (local.get $r) (local.get $old) (local.get $osize))))
        (local.get $r))

      ;; response "\u{f8}k" latin1 = F8 6B (2 code units, untagged)
      (data (i32.const 64) "\f8\6b")
      ;; response "n\u{2603}" utf16 = 006E 2603 (2 code units, tagged)
      (data (i32.const 68) "\6e\00\03\26")

      ;; expect "AB" deflated to latin1 = 41 42 (2 code units, untagged)
      (func (export "deflate") (param $ptr i32) (param $tagged i32) (result i32)
        (if (i32.ne (local.get $tagged) (i32.const 2)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $ptr)) (i32.const 0x4241)) (then unreachable))
        (i32.store (i32.const 0) (i32.const 64))
        (i32.store (i32.const 4) (i32.const 2))
        (i32.const 0))

      ;; expect "\u{2603}" kept as utf16 = 2603 (1 code unit, i32-tagged)
      (func (export "keep-wide") (param $ptr i32) (param $tagged i32) (result i32)
        (if (i32.ne (local.get $tagged) (i32.const 0x80000001)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $ptr)) (i32.const 0x2603)) (then unreachable))
        (i32.store (i32.const 0) (i32.const 68))
        (i32.store (i32.const 4) (i32.const 0x80000002))
        (i32.const 0))
    )
    (core instance $m (instantiate $M))
    (func (export "deflate") (param "s" string) (result string)
      (canon lift (core func $m "deflate") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
    (func (export "keep-wide") (param "s" string) (result string)
      (canon lift (core func $m "keep-wide") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
  )

  (component $D
    (import "deflate" (func $deflate (param "s" string) (result string)))
    (import "keep-wide" (func $keep-wide (param "s" string) (result string)))
    (core module $Libc
      (memory (export "mem") i64 1)
      (global $next (mut i64) (i64.const 1024))
      (func (export "realloc") (param $old i64) (param $osize i64) (param $align i64) (param $nsize i64) (result i64)
        (local $r i64)
        (if (i32.and (i64.ne (local.get $old) (i64.const 0))
                     (i64.le_u (local.get $nsize) (local.get $osize)))
          (then (return (local.get $old))))
        (global.set $next (i64.and (i64.add (global.get $next) (i64.const 7)) (i64.const -8)))
        (local.set $r (global.get $next))
        (global.set $next (i64.add (global.get $next) (local.get $nsize)))
        (if (i64.ne (local.get $old) (i64.const 0))
          (then (memory.copy (local.get $r) (local.get $old) (local.get $osize))))
        (local.get $r))
    )
    (core instance $libc (instantiate $Libc))
    (core func $deflate' (canon lower (func $deflate) string-encoding=latin1+utf16
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core func $keep-wide' (canon lower (func $keep-wide) string-encoding=latin1+utf16
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core module $Main
      (import "" "mem" (memory i64 1))
      (import "" "deflate" (func $deflate (param i64 i64 i64)))
      (import "" "keep-wide" (func $keep-wide (param i64 i64 i64)))

      ;; argument "AB" over-encoded as utf16 = 0041 0042 (2 code units, tagged)
      (data (i64.const 16) "\41\00\42\00")
      ;; argument "\u{2603}" utf16 = 2603 (1 code unit, tagged)
      (data (i64.const 24) "\03\26")

      (func (export "run") (result i32)
        (local $p i64)
        (call $deflate (i64.const 16) (i64.const 0x8000000000000002) (i64.const 32))
        ;; expect "\u{f8}k" latin1 = F8 6B (2 code units, untagged)
        (local.set $p (i64.load (i64.const 32)))
        (if (i64.ne (i64.load (i64.const 40)) (i64.const 2)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $p)) (i32.const 0x6BF8)) (then unreachable))
        (call $keep-wide (i64.const 24) (i64.const 0x8000000000000001) (i64.const 32))
        ;; expect "n\u{2603}" utf16 = 006E 2603 (2 code units, i64-tagged)
        (local.set $p (i64.load (i64.const 32)))
        (if (i64.ne (i64.load (i64.const 40)) (i64.const 0x8000000000000002)) (then unreachable))
        (if (i32.ne (i32.load (local.get $p)) (i32.const 0x2603006E)) (then unreachable))
        (i32.const 42))
    )
    (core instance $main (instantiate $Main (with "" (instance
      (export "mem" (memory $libc "mem"))
      (export "deflate" (func $deflate'))
      (export "keep-wide" (func $keep-wide'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $main "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D
    (with "deflate" (func $c "deflate"))
    (with "keep-wide" (func $c "keep-wide"))
  ))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 42))

;; i32 caller -> i64 callee: the same strings in the other direction.
(component
  (component $C
    (core module $M
      (memory (export "mem") i64 1)
      (global $next (mut i64) (i64.const 1024))
      (func (export "realloc") (param $old i64) (param $osize i64) (param $align i64) (param $nsize i64) (result i64)
        (local $r i64)
        (if (i32.and (i64.ne (local.get $old) (i64.const 0))
                     (i64.le_u (local.get $nsize) (local.get $osize)))
          (then (return (local.get $old))))
        (global.set $next (i64.and (i64.add (global.get $next) (i64.const 7)) (i64.const -8)))
        (local.set $r (global.get $next))
        (global.set $next (i64.add (global.get $next) (local.get $nsize)))
        (if (i64.ne (local.get $old) (i64.const 0))
          (then (memory.copy (local.get $r) (local.get $old) (local.get $osize))))
        (local.get $r))

      ;; response "\u{f8}k" latin1 = F8 6B (2 code units, untagged)
      (data (i64.const 64) "\f8\6b")
      ;; response "n\u{2603}" utf16 = 006E 2603 (2 code units, tagged)
      (data (i64.const 68) "\6e\00\03\26")

      ;; expect "AB" deflated to latin1 = 41 42 (2 code units, untagged)
      (func (export "deflate") (param $ptr i64) (param $tagged i64) (result i64)
        (if (i64.ne (local.get $tagged) (i64.const 2)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $ptr)) (i32.const 0x4241)) (then unreachable))
        (i64.store (i64.const 0) (i64.const 64))
        (i64.store (i64.const 8) (i64.const 2))
        (i64.const 0))

      ;; expect "\u{2603}" kept as utf16 = 2603 (1 code unit, i64-tagged)
      (func (export "keep-wide") (param $ptr i64) (param $tagged i64) (result i64)
        (if (i64.ne (local.get $tagged) (i64.const 0x8000000000000001)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $ptr)) (i32.const 0x2603)) (then unreachable))
        (i64.store (i64.const 0) (i64.const 68))
        (i64.store (i64.const 8) (i64.const 0x8000000000000002))
        (i64.const 0))
    )
    (core instance $m (instantiate $M))
    (func (export "deflate") (param "s" string) (result string)
      (canon lift (core func $m "deflate") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
    (func (export "keep-wide") (param "s" string) (result string)
      (canon lift (core func $m "keep-wide") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
  )

  (component $D
    (import "deflate" (func $deflate (param "s" string) (result string)))
    (import "keep-wide" (func $keep-wide (param "s" string) (result string)))
    (core module $Libc
      (memory (export "mem") 1)
      (global $next (mut i32) (i32.const 1024))
      (func (export "realloc") (param $old i32) (param $osize i32) (param $align i32) (param $nsize i32) (result i32)
        (local $r i32)
        (if (i32.and (i32.ne (local.get $old) (i32.const 0))
                     (i32.le_u (local.get $nsize) (local.get $osize)))
          (then (return (local.get $old))))
        (global.set $next (i32.and (i32.add (global.get $next) (i32.const 7)) (i32.const -8)))
        (local.set $r (global.get $next))
        (global.set $next (i32.add (global.get $next) (local.get $nsize)))
        (if (i32.ne (local.get $old) (i32.const 0))
          (then (memory.copy (local.get $r) (local.get $old) (local.get $osize))))
        (local.get $r))
    )
    (core instance $libc (instantiate $Libc))
    (core func $deflate' (canon lower (func $deflate) string-encoding=latin1+utf16
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core func $keep-wide' (canon lower (func $keep-wide) string-encoding=latin1+utf16
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core module $Main
      (import "" "mem" (memory 1))
      (import "" "deflate" (func $deflate (param i32 i32 i32)))
      (import "" "keep-wide" (func $keep-wide (param i32 i32 i32)))

      ;; argument "AB" over-encoded as utf16 = 0041 0042 (2 code units, tagged)
      (data (i32.const 16) "\41\00\42\00")
      ;; argument "\u{2603}" utf16 = 2603 (1 code unit, tagged)
      (data (i32.const 24) "\03\26")

      (func (export "run") (result i32)
        (local $p i32)
        (call $deflate (i32.const 16) (i32.const 0x80000002) (i32.const 8))
        ;; expect "\u{f8}k" latin1 = F8 6B (2 code units, untagged)
        (local.set $p (i32.load (i32.const 8)))
        (if (i32.ne (i32.load (i32.const 12)) (i32.const 2)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $p)) (i32.const 0x6BF8)) (then unreachable))
        (call $keep-wide (i32.const 24) (i32.const 0x80000001) (i32.const 8))
        ;; expect "n\u{2603}" utf16 = 006E 2603 (2 code units, i32-tagged)
        (local.set $p (i32.load (i32.const 8)))
        (if (i32.ne (i32.load (i32.const 12)) (i32.const 0x80000002)) (then unreachable))
        (if (i32.ne (i32.load (local.get $p)) (i32.const 0x2603006E)) (then unreachable))
        (i32.const 43))
    )
    (core instance $main (instantiate $Main (with "" (instance
      (export "mem" (memory $libc "mem"))
      (export "deflate" (func $deflate'))
      (export "keep-wide" (func $keep-wide'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $main "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D
    (with "deflate" (func $c "deflate"))
    (with "keep-wide" (func $c "keep-wide"))
  ))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 43))

;; i32 utf8 caller -> i64 latin1+utf16 callee: latin1-representable input
;; arrives as latin1, and other input is inflated to i64-tagged utf16.
(component
  (component $C
    (core module $M
      (memory (export "mem") i64 1)
      (global $next (mut i64) (i64.const 1024))
      (func (export "realloc") (param $old i64) (param $osize i64) (param $align i64) (param $nsize i64) (result i64)
        (local $r i64)
        (if (i32.and (i64.ne (local.get $old) (i64.const 0))
                     (i64.le_u (local.get $nsize) (local.get $osize)))
          (then (return (local.get $old))))
        (global.set $next (i64.and (i64.add (global.get $next) (i64.const 7)) (i64.const -8)))
        (local.set $r (global.get $next))
        (global.set $next (i64.add (global.get $next) (local.get $nsize)))
        (if (i64.ne (local.get $old) (i64.const 0))
          (then (memory.copy (local.get $r) (local.get $old) (local.get $osize))))
        (local.get $r))

      ;; expect "AB" as latin1 = 41 42 (2 code units, untagged)
      (func (export "narrow") (param $ptr i64) (param $tagged i64)
        (if (i64.ne (local.get $tagged) (i64.const 2)) (then unreachable))
        (if (i32.ne (i32.load16_u (local.get $ptr)) (i32.const 0x4241)) (then unreachable)))

      ;; expect "a\u{2603}" as utf16 = 0061 2603 (2 code units, i64-tagged)
      (func (export "wide") (param $ptr i64) (param $tagged i64)
        (if (i64.ne (local.get $tagged) (i64.const 0x8000000000000002)) (then unreachable))
        (if (i32.ne (i32.load (local.get $ptr)) (i32.const 0x26030061)) (then unreachable)))
    )
    (core instance $m (instantiate $M))
    (func (export "narrow") (param "s" string)
      (canon lift (core func $m "narrow") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
    (func (export "wide") (param "s" string)
      (canon lift (core func $m "wide") string-encoding=latin1+utf16
        (memory (core memory $m "mem")) (realloc (core func $m "realloc"))))
  )

  (component $D
    (import "narrow" (func $narrow (param "s" string)))
    (import "wide" (func $wide (param "s" string)))
    (core module $Libc
      (memory (export "mem") 1)
    )
    (core instance $libc (instantiate $Libc))
    (core func $narrow' (canon lower (func $narrow) string-encoding=utf8
      (memory (core memory $libc "mem"))))
    (core func $wide' (canon lower (func $wide) string-encoding=utf8
      (memory (core memory $libc "mem"))))
    (core module $Main
      (import "" "mem" (memory 1))
      (import "" "narrow" (func $narrow (param i32 i32)))
      (import "" "wide" (func $wide (param i32 i32)))

      ;; argument "AB" utf8 = 41 42
      (data (i32.const 16) "AB")
      ;; argument "a\u{2603}" utf8 = 61 E2 98 83
      (data (i32.const 24) "a\u{2603}")

      (func (export "run") (result i32)
        (call $narrow (i32.const 16) (i32.const 2))
        (call $wide (i32.const 24) (i32.const 4))
        (i32.const 44))
    )
    (core instance $main (instantiate $Main (with "" (instance
      (export "mem" (memory $libc "mem"))
      (export "narrow" (func $narrow'))
      (export "wide" (func $wide'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $main "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D
    (with "narrow" (func $c "narrow"))
    (with "wide" (func $c "wide"))
  ))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 44))

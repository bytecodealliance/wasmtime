;;! gc = true
;;! exceptions = true
;;! simd = true

(module
  (type $a (array (mut i8)))
  (global $g (mut anyref) (ref.null any))
  (func (export "f") (param i32)
    (global.set $g (array.new_default $a (i32.sub (i32.const -1) (local.get 0))))
  )
)

;; Most of these will trap with either "allocation size too large" or "GC heap
;; out of memory", but some may succeed. Any of those results is fine, which is
;; why we do not `assert_return` or `assert_trap` here. What we mostly don't
;; want is to e.g. hit any integer overflow assertions in our free lists or bump
;; pointers.
(invoke "f" (i32.const 0))
(invoke "f" (i32.const 8))
(invoke "f" (i32.const 16))
(invoke "f" (i32.const 24))
(invoke "f" (i32.const 32))
(invoke "f" (i32.const 40))
(invoke "f" (i32.const 48))
(invoke "f" (i32.const 56))
(invoke "f" (i32.const 64))
(invoke "f" (i32.const 72))
(invoke "f" (i32.const 80))

;;! branch_hinting = true

;; Branch hints are advisory: with the proposal enabled a hinted module must
;; still produce the same results it would without hints. Both functions return
;; 10 when their argument is nonzero and 20 otherwise. The
;; `(@metadata.code.branch_hint ...)` annotation must immediately precede the
;; `if`/`br_if` it applies to.

(module
  (func (export "via_if") (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\00")
    if (result i32)
      i32.const 10
    else
      i32.const 20
    end)

  (func (export "via_br_if") (param i32) (result i32)
    (block $b (result i32)
      i32.const 10
      local.get 0
      (@metadata.code.branch_hint "\01")
      br_if $b
      drop
      i32.const 20)))

(assert_return (invoke "via_if" (i32.const 0)) (i32.const 20))
(assert_return (invoke "via_if" (i32.const 1)) (i32.const 10))
(assert_return (invoke "via_if" (i32.const 7)) (i32.const 10))
(assert_return (invoke "via_if" (i32.const -3)) (i32.const 10))

(assert_return (invoke "via_br_if" (i32.const 0)) (i32.const 20))
(assert_return (invoke "via_br_if" (i32.const 1)) (i32.const 10))
(assert_return (invoke "via_br_if" (i32.const 7)) (i32.const 10))
(assert_return (invoke "via_br_if" (i32.const -3)) (i32.const 10))

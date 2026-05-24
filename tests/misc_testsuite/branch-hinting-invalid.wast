;;! branch_hinting = true

;; A malformed `metadata.code.branch_hint` section must be ignored, not rejected:
;; the section is advisory and unvalidated, so a decode error discards it and
;; compilation still succeeds. Here the hint's reserved length byte is `\02`
;; instead of the required `\01`, so decoding the section fails.
;;
;; Section bytes: \01 = one function; \00 = func 0; \01 = one hint;
;;               \00 = func_offset 0; \02 = (invalid) hint length byte.

(module
  (@custom "metadata.code.branch_hint" (after code) "\01\00\01\00\02")
  (func (export "f") (param i32) (result i32)
    local.get 0
    if (result i32)
      i32.const 10
    else
      i32.const 20
    end))

(assert_return (invoke "f" (i32.const 0)) (i32.const 20))
(assert_return (invoke "f" (i32.const 1)) (i32.const 10))

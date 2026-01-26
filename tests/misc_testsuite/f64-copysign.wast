;; This would previously segfault or trap on x86-64 because the
;; `f64.copysign` sunk the `f64.load` and widened it to 128 bits
;; incorrectly.

(module
  ;; Define a linear memory with 1 page (64KiB)
  (memory 1)
  (export "f" (func 0))
  (func (result i32)
    ;; Push i32 constant 0 (destination address for the store)
    i32.const 0
    ;; Push f64 constant 0.0 (sign source for copysign)
    f64.const 0
    ;; Push i32 constant 32 (base address for the load)
    i32.const 32
    ;; Load f64 from memory at address (32 + 65491), with align=1
    f64.load offset=65491 align=1
    ;; Apply copysign: take magnitude from loaded f64 and sign from 0.0
    f64.copysign
    ;; Store f64 to memory at address 0, with align=1
    f64.store align=1
    ;; Return 0.
    i32.const 0
  )
)

(assert_return (invoke "f") (i32.const 0))

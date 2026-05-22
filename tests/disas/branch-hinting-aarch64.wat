;;! target = "aarch64"
;;! test = "compile"
;;! flags = "-Wbranch-hinting"

;; aarch64 honors branch hints too: the cold path is laid out after the hot
;; path in the generated machine code (the branch-hinting optimization lives in
;; target-independent Cranelift code, so it applies on every backend).

(module
  ;; condition likely false: the then-block is cold.
  (func $if_unlikely (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\00")
    if (result i32)
      i32.const 10
    else
      i32.const 20
    end
  )

  ;; condition likely true: the fallthrough is cold.
  (func $br_if_likely (param i32) (result i32)
    (block $b (result i32)
      i32.const 10
      local.get 0
      (@metadata.code.branch_hint "\01")
      br_if $b
      drop
      i32.const 20
    )
  )
)
;; wasm[0]::function[0]::if_unlikely:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       cbnz    w4, #0x18
;;    c: mov     w2, #0x14
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   18: mov     w2, #0xa
;;   1c: b       #0x10
;;
;; wasm[0]::function[1]::br_if_likely:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     w2, #0xa
;;       cbz     w4, #0x38
;;   30: ldp     x29, x30, [sp], #0x10
;;       ret
;;   38: mov     w2, #0x14
;;   3c: b       #0x30

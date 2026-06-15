;;! target = "aarch64"
;;! test = "compile"
;;! flags = "-C cranelift-has_dotprod=true"

;; `i32x4.relaxed_dot_i8x16_i7x16_add_s` with FEAT_DotProd: the dot-product tree
;; lowers to a single `sdot`.
(module
  (func (param v128 v128 v128) (result v128)
    local.get 0
    local.get 1
    local.get 2
    i32x4.relaxed_dot_i8x16_i7x16_add_s
  )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     v6.16b, v0.16b
;;       mov     v0.16b, v2.16b
;;       sdot    v0.4s, v6.16b, v1.16b
;;       ldp     x29, x30, [sp], #0x10
;;       ret

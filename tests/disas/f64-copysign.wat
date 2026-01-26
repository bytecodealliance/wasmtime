;;! target = "x86_64"
;;! test = "compile"
;;!flags = "-Ccranelift-has-avx"

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
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x38(%rdi), %rcx
;;       vmovsd  0xfff3(%rcx), %xmm4
;;       vxorpd  %xmm3, %xmm3, %xmm5
;;       movabsq $9223372036854775808, %r11
;;       vmovq   %r11, %xmm2
;;       vandnpd %xmm5, %xmm2, %xmm5
;;       vandpd  %xmm4, %xmm2, %xmm6
;;       vorpd   %xmm6, %xmm5, %xmm0
;;       vmovsd  %xmm0, (%rcx)
;;       xorl    %eax, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

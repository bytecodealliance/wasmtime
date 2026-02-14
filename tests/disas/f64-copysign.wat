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
;;       movq    0x38(%rdi), %rax
;;       vmovsd  0xfff3(%rax), %xmm0
;;       vxorpd  %xmm1, %xmm1, %xmm1
;;       movabsq $9223372036854775808, %r8
;;       vmovq   %r8, %xmm2
;;       vandnpd %xmm1, %xmm2, %xmm1
;;       vandpd  %xmm0, %xmm2, %xmm0
;;       vorpd   %xmm0, %xmm1, %xmm0
;;       vmovsd  %xmm0, (%rax)
;;       xorl    %eax, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

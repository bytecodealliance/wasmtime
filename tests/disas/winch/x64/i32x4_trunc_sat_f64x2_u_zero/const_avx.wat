;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.trunc_sat_f64x2_u_zero (v128.const f32x4 1 2 3 4))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x60
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x3c(%rip), %xmm0
;;       vxorpd  %xmm15, %xmm15, %xmm15
;;       vmaxpd  %xmm15, %xmm0, %xmm0
;;       vminpd  0x3a(%rip), %xmm0, %xmm0
;;       vroundpd $3, %xmm0, %xmm0
;;       vaddpd  0x3c(%rip), %xmm0, %xmm0
;;       vshufps $0x88, %xmm15, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   60: ud2
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: cmpb    $0, (%rdi)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)
;;   7e: addb    $0, (%rax)
;;   82: loopne  0x83

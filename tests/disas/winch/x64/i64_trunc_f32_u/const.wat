;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x5c(%rip), %xmm1
;;       movl    $0x5f000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm1
;;       jae     0x60
;;       jp      0x8c
;;   4f: cvttss2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0x84
;;   5e: ud2
;;       movaps  %xmm1, %xmm0
;;       subss   %xmm15, %xmm0
;;       cvttss2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x8e
;;   77: movabsq $9223372036854775808, %r11
;;       addq    %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   8a: ud2
;;   8c: ud2
;;   8e: ud2
;;   90: addb    %al, (%rax)

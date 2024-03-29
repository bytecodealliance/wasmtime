;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7e
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x55(%rip), %xmm1
;;       movl    $0x4f000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm1
;;       jae     0x5d
;;       jp      0x80
;;   4e: cvttss2si %xmm1, %eax
;;       cmpl    $0, %eax
;;       jge     0x78
;;   5b: ud2
;;       movaps  %xmm1, %xmm0
;;       subss   %xmm15, %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x82
;;   72: addl    $0x80000000, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7e: ud2
;;   80: ud2
;;   82: ud2
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)

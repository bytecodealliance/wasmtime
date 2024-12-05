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
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x54(%rip), %xmm1
;;       movl    $0x4f000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm1
;;       jae     0x5e
;;       jp      0x81
;;   4f: cvttss2si %xmm1, %eax
;;       cmpl    $0, %eax
;;       jge     0x79
;;   5c: ud2
;;       movaps  %xmm1, %xmm0
;;       subss   %xmm15, %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x83
;;   73: addl    $0x80000000, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7f: ud2
;;   81: ud2
;;   83: ud2
;;   85: addb    %al, (%rax)
;;   87: addb    %al, (%rax)

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x84
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x5c(%rip), %xmm1
;;       movabsq $0x41e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x63
;;       jp      0x86
;;   54: cvttsd2si %xmm1, %eax
;;       cmpl    $0, %eax
;;       jge     0x7e
;;   61: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x88
;;   78: addl    $0x80000000, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   84: ud2
;;   86: ud2
;;   88: ud2
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: addb    %al, (%rax)
;;   92: addb    %al, (%rax)
;;   94: addb    %al, (%rax)

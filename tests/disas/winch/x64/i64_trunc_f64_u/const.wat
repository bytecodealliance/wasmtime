;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x95
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x69(%rip), %xmm1
;;       movabsq $0x43e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x68
;;       jp      0x97
;;   57: cvttsd2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0x8c
;;   66: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x99
;;   7f: movabsq $9223372036854775808, %r11
;;       addq    %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   95: ud2
;;   97: ud2
;;   99: ud2
;;   9b: addb    %al, (%rax)
;;   9d: addb    %al, (%rax)
;;   9f: addb    %al, (%rax)
;;   a1: addb    %al, (%rax)
;;   a3: addb    %al, (%rax)
;;   a5: addb    %dh, %al

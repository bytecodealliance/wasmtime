;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x83
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x59(%rip), %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x7a
;;   46: ucomisd %xmm0, %xmm0
;;       jp      0x85
;;   50: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x87
;;   6a: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x89
;;   7a: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   83: ud2
;;   85: ud2
;;   87: ud2
;;   89: ud2
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
;;   8f: addb    %al, (%rax)
;;   91: addb    %al, (%rax)
;;   93: addb    %al, (%rax)
;;   95: addb    %dh, %al

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7a
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x55(%rip), %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x74
;;   40: ucomisd %xmm0, %xmm0
;;       jp      0x7c
;;   4a: movabsq $13970166044105375744, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jbe     0x7e
;;   64: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x80
;;   74: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)

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
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7c
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x55(%rip), %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x76
;;   42: ucomisd %xmm0, %xmm0
;;       jp      0x7e
;;   4c: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x80
;;   66: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x82
;;   76: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: ud2
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)

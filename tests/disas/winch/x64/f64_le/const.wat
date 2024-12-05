;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.le)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x24(%rip), %xmm0
;;       movsd   0x24(%rip), %xmm1
;;       ucomisd %xmm1, %xmm0
;;       movl    $0, %eax
;;       setae   %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %bl, -0x66666667(%rdx)
;;   5d: cltd
;;   5e: addl    %eax, -0x66(%rax)
;;   61: cltd
;;   62: cltd
;;   63: cltd
;;   64: cltd
;;   65: cltd
;;   66: int1

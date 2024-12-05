;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.max)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x3c(%rip), %xmm0
;;       movsd   0x3c(%rip), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jne     0x5f
;;       jp      0x55
;;   4c: andpd   %xmm0, %xmm1
;;       jmp     0x63
;;   55: addsd   %xmm0, %xmm1
;;       jp      0x63
;;   5f: maxsd   %xmm0, %xmm1
;;       movapd  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2
;;   6f: addb    %bl, -0x66666667(%rdx)
;;   75: cltd
;;   76: addl    %eax, -0x66(%rax)
;;   79: cltd
;;   7a: cltd
;;   7b: cltd
;;   7c: cltd
;;   7d: cltd
;;   7e: int1

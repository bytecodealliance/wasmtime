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
;;       ja      0x73
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x41(%rip), %xmm0
;;       movsd   0x41(%rip), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jne     0x62
;;       jp      0x58
;;   4f: andpd   %xmm0, %xmm1
;;       jmp     0x66
;;   58: addsd   %xmm0, %xmm1
;;       jp      0x66
;;   62: maxsd   %xmm0, %xmm1
;;       movapd  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   73: ud2
;;   75: addb    %al, (%rax)
;;   77: addb    %bl, -0x66666667(%rdx)
;;   7d: cltd
;;   7e: addl    %eax, -0x66(%rax)
;;   81: cltd
;;   82: cltd
;;   83: cltd
;;   84: cltd
;;   85: cltd
;;   86: int1

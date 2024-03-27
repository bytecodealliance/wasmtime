;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.eq)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x2d(%rip), %xmm0
;;       movsd   0x2d(%rip), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       movl    $0, %eax
;;       sete    %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andq    %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2
;;   5d: addb    %al, (%rax)
;;   5f: addb    %bl, -0x66666667(%rdx)
;;   65: cltd
;;   66: addl    %eax, -0x66(%rax)
;;   69: cltd
;;   6a: cltd
;;   6b: cltd
;;   6c: cltd
;;   6d: cltd
;;   6e: int1

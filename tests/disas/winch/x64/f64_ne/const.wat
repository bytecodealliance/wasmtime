;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.ne)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x61
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x31(%rip), %xmm0
;;       movsd   0x31(%rip), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       movl    $0, %eax
;;       setne   %al
;;       movl    $0, %r11d
;;       setp    %r11b
;;       orq     %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   61: ud2
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %bl, -0x66666667(%rdx)
;;   6d: cltd
;;   6e: addl    %eax, -0x66(%rax)
;;   71: cltd
;;   72: cltd
;;   73: cltd
;;   74: cltd
;;   75: cltd
;;   76: int1

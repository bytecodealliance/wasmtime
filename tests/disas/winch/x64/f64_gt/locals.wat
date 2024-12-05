;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.gt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7f
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       xorq    %r11, %r11
;;       movq    %r11, 8(%rsp)
;;       movq    %r11, (%rsp)
;;       movsd   0x47(%rip), %xmm0
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   0x41(%rip), %xmm0
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       movsd   8(%rsp), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       movl    $0, %eax
;;       seta    %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andq    %r11, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   7f: ud2
;;   81: addb    %al, (%rax)
;;   83: addb    %al, (%rax)
;;   85: addb    %al, (%rax)
;;   87: addb    %bl, -0x66666667(%rdx)
;;   8d: cltd
;;   8e: int1

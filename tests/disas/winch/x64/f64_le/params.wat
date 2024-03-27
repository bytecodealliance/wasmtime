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
        f64.le
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x71
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       xorl    %r11d, %r11d
;;       movq    %r11, 8(%rsp)
;;       movq    %r11, (%rsp)
;;       movsd   0x38(%rip), %xmm0
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   0x32(%rip), %xmm0
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       movsd   8(%rsp), %xmm1
;;       ucomisd %xmm1, %xmm0
;;       movl    $0, %eax
;;       setae   %al
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   71: ud2
;;   73: addb    %al, (%rax)
;;   75: addb    %al, (%rax)
;;   77: addb    %bl, -0x66666667(%rdx)
;;   7d: cltd
;;   7e: int1

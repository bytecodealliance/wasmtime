;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.div
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6b
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movss   0x32(%rip), %xmm0
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   0x2c(%rip), %xmm0
;;       movss   %xmm0, 8(%rsp)
;;       movss   8(%rsp), %xmm0
;;       movss   0xc(%rsp), %xmm1
;;       divss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   6b: ud2
;;   6d: addb    %al, (%rax)
;;   6f: addb    %cl, %ch
;;   71: int3

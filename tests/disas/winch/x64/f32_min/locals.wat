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
        f32.min
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x88
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movss   0x54(%rip), %xmm0
;;       movss   %xmm0, 4(%rsp)
;;       movss   0x4e(%rip), %xmm0
;;       movss   %xmm0, (%rsp)
;;       movss   (%rsp), %xmm0
;;       movss   4(%rsp), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x7b
;;       jp      0x71
;;   69: orps    %xmm0, %xmm1
;;       jmp     0x7f
;;   71: addss   %xmm0, %xmm1
;;       jp      0x7f
;;   7b: minss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   88: ud2
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: int     $0xcc

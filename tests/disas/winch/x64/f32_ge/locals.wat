;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.ge
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x79
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movss   0x44(%rip), %xmm0
;;       movss   %xmm0, 4(%rsp)
;;       movss   0x3e(%rip), %xmm0
;;       movss   %xmm0, (%rsp)
;;       movss   (%rsp), %xmm0
;;       movss   4(%rsp), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       movl    $0, %eax
;;       setae   %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andl    %r11d, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   79: ud2
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %cl, %ch
;;   81: int3

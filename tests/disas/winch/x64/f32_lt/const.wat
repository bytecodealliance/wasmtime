;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.lt)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x1c(%rip), %xmm0
;;       movss   0x1c(%rip), %xmm1
;;       ucomiss %xmm1, %xmm0
;;       movl    $0, %eax
;;       seta    %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
;;   50: int     $0xcc
;;   52: orb     $0x40, %al
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: int     $0xcc

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.gt)
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
;;       movss   0x31(%rip), %xmm0
;;       movss   0x31(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       movl    $0, %eax
;;       seta    %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andl    %r11d, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   61: ud2
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %cl, %ch
;;   69: int3
;;   6a: orb     $0x40, %al
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: int     $0xcc

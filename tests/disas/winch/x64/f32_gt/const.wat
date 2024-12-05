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
;;       ja      0x5b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x2c(%rip), %xmm0
;;       movss   0x2c(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       movl    $0, %eax
;;       seta    %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andl    %r11d, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2
;;   5d: addb    %al, (%rax)
;;   5f: addb    %cl, %ch
;;   61: int3
;;   62: orb     $0x40, %al
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: int     $0xcc

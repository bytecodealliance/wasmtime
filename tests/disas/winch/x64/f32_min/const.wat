;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.min)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x64
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x31(%rip), %xmm0
;;       movss   0x31(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x54
;;       jp      0x4e
;;   46: orps    %xmm0, %xmm1
;;       jmp     0x58
;;   4e: addss   %xmm0, %xmm1
;;       jp      0x58
;;   54: minss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   64: ud2
;;   66: addb    %al, (%rax)
;;   68: int     $0xcc
;;   6a: orb     $0x40, %al
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: int     $0xcc

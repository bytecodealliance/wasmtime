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
;;       ja      0x6a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x3c(%rip), %xmm0
;;       movss   0x3c(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x5d
;;       jp      0x53
;;   4b: orps    %xmm0, %xmm1
;;       jmp     0x61
;;   53: addss   %xmm0, %xmm1
;;       jp      0x61
;;   5d: minss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6a: ud2
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: int     $0xcc
;;   72: orb     $0x40, %al
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: int     $0xcc

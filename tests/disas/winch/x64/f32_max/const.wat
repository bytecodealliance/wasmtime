;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.max)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x70
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x41(%rip), %xmm0
;;       movss   0x41(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x60
;;       jp      0x56
;;   4e: andps   %xmm0, %xmm1
;;       jmp     0x64
;;   56: addss   %xmm0, %xmm1
;;       jp      0x64
;;   60: maxss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   70: ud2
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: int     $0xcc
;;   7a: orb     $0x40, %al
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: int     $0xcc

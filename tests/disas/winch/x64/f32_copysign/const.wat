;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const -1.1)
        (f32.const 2.2)
        (f32.copysign)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x65
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x31(%rip), %xmm0
;;       movss   0x31(%rip), %xmm1
;;       movl    $0x80000000, %r11d
;;       movd    %r11d, %xmm15
;;       andps   %xmm15, %xmm0
;;       andnps  %xmm1, %xmm15
;;       movaps  %xmm15, %xmm1
;;       orps    %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   65: ud2
;;   67: addb    %cl, %ch
;;   69: int3
;;   6a: orb     $0x40, %al
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: int     $0xcc

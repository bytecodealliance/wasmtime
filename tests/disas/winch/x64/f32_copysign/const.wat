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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x2d(%rip), %xmm0
;;   33: movss   0x2d(%rip), %xmm1
;;   3b: movl    $0x80000000, %r11d
;;   41: movd    %r11d, %xmm15
;;   46: andps   %xmm15, %xmm0
;;   4a: andnps  %xmm1, %xmm15
;;   4e: movaps  %xmm15, %xmm1
;;   52: orps    %xmm0, %xmm1
;;   55: movaps  %xmm1, %xmm0
;;   58: addq    $0x10, %rsp
;;   5c: popq    %rbp
;;   5d: retq
;;   5e: ud2
;;   60: int     $0xcc
;;   62: orb     $0x40, %al
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: int     $0xcc

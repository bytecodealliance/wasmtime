;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.div)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x48
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x1d(%rip), %xmm0
;;   33: movss   0x1d(%rip), %xmm1
;;   3b: divss   %xmm0, %xmm1
;;   3f: movaps  %xmm1, %xmm0
;;   42: addq    $0x10, %rsp
;;   46: popq    %rbp
;;   47: retq
;;   48: ud2
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: int     $0xcc
;;   52: orb     $0x40, %al
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: int     $0xcc

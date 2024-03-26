;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x89
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movsd   (%rsp), %xmm1
;;   39: movabsq $0x41e0000000000000, %r11
;;   43: movq    %r11, %xmm15
;;   48: ucomisd %xmm15, %xmm1
;;   4d: jae     0x68
;;   53: jp      0x8b
;;   59: cvttsd2si %xmm1, %eax
;;   5d: cmpl    $0, %eax
;;   60: jge     0x83
;;   66: ud2
;;   68: movaps  %xmm1, %xmm0
;;   6b: subsd   %xmm15, %xmm0
;;   70: cvttsd2si %xmm0, %eax
;;   74: cmpl    $0, %eax
;;   77: jl      0x8d
;;   7d: addl    $0x80000000, %eax
;;   83: addq    $0x18, %rsp
;;   87: popq    %rbp
;;   88: retq
;;   89: ud2
;;   8b: ud2
;;   8d: ud2

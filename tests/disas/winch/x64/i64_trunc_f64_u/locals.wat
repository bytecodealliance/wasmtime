;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x94
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movsd   (%rsp), %xmm1
;;   39: movabsq $0x43e0000000000000, %r11
;;   43: movq    %r11, %xmm15
;;   48: ucomisd %xmm15, %xmm1
;;   4d: jae     0x6a
;;   53: jp      0x96
;;   59: cvttsd2si %xmm1, %rax
;;   5e: cmpq    $0, %rax
;;   62: jge     0x8e
;;   68: ud2
;;   6a: movaps  %xmm1, %xmm0
;;   6d: subsd   %xmm15, %xmm0
;;   72: cvttsd2si %xmm0, %rax
;;   77: cmpq    $0, %rax
;;   7b: jl      0x98
;;   81: movabsq $9223372036854775808, %r11
;;   8b: addq    %r11, %rax
;;   8e: addq    $0x18, %rsp
;;   92: popq    %rbp
;;   93: retq
;;   94: ud2
;;   96: ud2
;;   98: ud2

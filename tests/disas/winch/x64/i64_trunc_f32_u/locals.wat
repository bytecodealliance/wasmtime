;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x90
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   4(%rsp), %xmm1
;;   3a: movl    $0x5f000000, %r11d
;;   40: movd    %r11d, %xmm15
;;   45: ucomiss %xmm15, %xmm1
;;   49: jae     0x66
;;   4f: jp      0x92
;;   55: cvttss2si %xmm1, %rax
;;   5a: cmpq    $0, %rax
;;   5e: jge     0x8a
;;   64: ud2
;;   66: movaps  %xmm1, %xmm0
;;   69: subss   %xmm15, %xmm0
;;   6e: cvttss2si %xmm0, %rax
;;   73: cmpq    $0, %rax
;;   77: jl      0x94
;;   7d: movabsq $9223372036854775808, %r11
;;   87: addq    %r11, %rax
;;   8a: addq    $0x18, %rsp
;;   8e: popq    %rbp
;;   8f: retq
;;   90: ud2
;;   92: ud2
;;   94: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
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
;;   15: ja      0x91
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm1
;;   36: movabsq $0x43e0000000000000, %r11
;;   40: movq    %r11, %xmm15
;;   45: ucomisd %xmm15, %xmm1
;;   4a: jae     0x67
;;   50: jp      0x93
;;   56: cvttsd2si %xmm1, %rax
;;   5b: cmpq    $0, %rax
;;   5f: jge     0x8b
;;   65: ud2
;;   67: movaps  %xmm1, %xmm0
;;   6a: subsd   %xmm15, %xmm0
;;   6f: cvttsd2si %xmm0, %rax
;;   74: cmpq    $0, %rax
;;   78: jl      0x95
;;   7e: movabsq $9223372036854775808, %r11
;;   88: addq    %r11, %rax
;;   8b: addq    $0x18, %rsp
;;   8f: popq    %rbp
;;   90: retq
;;   91: ud2
;;   93: ud2
;;   95: ud2

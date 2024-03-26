;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x89
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x5d(%rip), %xmm1
;;   33: movl    $0x5f000000, %r11d
;;   39: movd    %r11d, %xmm15
;;   3e: ucomiss %xmm15, %xmm1
;;   42: jae     0x5f
;;   48: jp      0x8b
;;   4e: cvttss2si %xmm1, %rax
;;   53: cmpq    $0, %rax
;;   57: jge     0x83
;;   5d: ud2
;;   5f: movaps  %xmm1, %xmm0
;;   62: subss   %xmm15, %xmm0
;;   67: cvttss2si %xmm0, %rax
;;   6c: cmpq    $0, %rax
;;   70: jl      0x8d
;;   76: movabsq $9223372036854775808, %r11
;;   80: addq    %r11, %rax
;;   83: addq    $0x10, %rsp
;;   87: popq    %rbp
;;   88: retq
;;   89: ud2
;;   8b: ud2
;;   8d: ud2
;;   8f: addb    %al, (%rax)

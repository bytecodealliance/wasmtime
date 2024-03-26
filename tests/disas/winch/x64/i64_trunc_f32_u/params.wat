;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result i64)
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
;;   15: ja      0x8e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm1
;;   38: movl    $0x5f000000, %r11d
;;   3e: movd    %r11d, %xmm15
;;   43: ucomiss %xmm15, %xmm1
;;   47: jae     0x64
;;   4d: jp      0x90
;;   53: cvttss2si %xmm1, %rax
;;   58: cmpq    $0, %rax
;;   5c: jge     0x88
;;   62: ud2
;;   64: movaps  %xmm1, %xmm0
;;   67: subss   %xmm15, %xmm0
;;   6c: cvttss2si %xmm0, %rax
;;   71: cmpq    $0, %rax
;;   75: jl      0x92
;;   7b: movabsq $9223372036854775808, %r11
;;   85: addq    %r11, %rax
;;   88: addq    $0x18, %rsp
;;   8c: popq    %rbp
;;   8d: retq
;;   8e: ud2
;;   90: ud2
;;   92: ud2

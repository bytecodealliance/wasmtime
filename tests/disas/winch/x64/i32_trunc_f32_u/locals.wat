;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x85
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   4(%rsp), %xmm1
;;   3a: movl    $0x4f000000, %r11d
;;   40: movd    %r11d, %xmm15
;;   45: ucomiss %xmm15, %xmm1
;;   49: jae     0x64
;;   4f: jp      0x87
;;   55: cvttss2si %xmm1, %eax
;;   59: cmpl    $0, %eax
;;   5c: jge     0x7f
;;   62: ud2
;;   64: movaps  %xmm1, %xmm0
;;   67: subss   %xmm15, %xmm0
;;   6c: cvttss2si %xmm0, %eax
;;   70: cmpl    $0, %eax
;;   73: jl      0x89
;;   79: addl    $0x80000000, %eax
;;   7f: addq    $0x18, %rsp
;;   83: popq    %rbp
;;   84: retq
;;   85: ud2
;;   87: ud2
;;   89: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result i32)
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
;;   15: ja      0x83
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm1
;;   38: movl    $0x4f000000, %r11d
;;   3e: movd    %r11d, %xmm15
;;   43: ucomiss %xmm15, %xmm1
;;   47: jae     0x62
;;   4d: jp      0x85
;;   53: cvttss2si %xmm1, %eax
;;   57: cmpl    $0, %eax
;;   5a: jge     0x7d
;;   60: ud2
;;   62: movaps  %xmm1, %xmm0
;;   65: subss   %xmm15, %xmm0
;;   6a: cvttss2si %xmm0, %eax
;;   6e: cmpl    $0, %eax
;;   71: jl      0x87
;;   77: addl    $0x80000000, %eax
;;   7d: addq    $0x18, %rsp
;;   81: popq    %rbp
;;   82: retq
;;   83: ud2
;;   85: ud2
;;   87: ud2

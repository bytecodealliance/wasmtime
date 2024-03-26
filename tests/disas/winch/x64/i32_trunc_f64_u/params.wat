;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i32)
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
;;   15: ja      0x86
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm1
;;   36: movabsq $0x41e0000000000000, %r11
;;   40: movq    %r11, %xmm15
;;   45: ucomisd %xmm15, %xmm1
;;   4a: jae     0x65
;;   50: jp      0x88
;;   56: cvttsd2si %xmm1, %eax
;;   5a: cmpl    $0, %eax
;;   5d: jge     0x80
;;   63: ud2
;;   65: movaps  %xmm1, %xmm0
;;   68: subsd   %xmm15, %xmm0
;;   6d: cvttsd2si %xmm0, %eax
;;   71: cmpl    $0, %eax
;;   74: jl      0x8a
;;   7a: addl    $0x80000000, %eax
;;   80: addq    $0x18, %rsp
;;   84: popq    %rbp
;;   85: retq
;;   86: ud2
;;   88: ud2
;;   8a: ud2

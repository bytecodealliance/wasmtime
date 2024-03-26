;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x8e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x65(%rip), %xmm1
;;   33: movabsq $0x43e0000000000000, %r11
;;   3d: movq    %r11, %xmm15
;;   42: ucomisd %xmm15, %xmm1
;;   47: jae     0x64
;;   4d: jp      0x90
;;   53: cvttsd2si %xmm1, %rax
;;   58: cmpq    $0, %rax
;;   5c: jge     0x88
;;   62: ud2
;;   64: movaps  %xmm1, %xmm0
;;   67: subsd   %xmm15, %xmm0
;;   6c: cvttsd2si %xmm0, %rax
;;   71: cmpq    $0, %rax
;;   75: jl      0x92
;;   7b: movabsq $9223372036854775808, %r11
;;   85: addq    %r11, %rax
;;   88: addq    $0x10, %rsp
;;   8c: popq    %rbp
;;   8d: retq
;;   8e: ud2
;;   90: ud2
;;   92: ud2
;;   94: addb    %al, (%rax)
;;   96: addb    %al, (%rax)
;;   98: addb    %al, (%rax)
;;   9a: addb    %al, (%rax)
;;   9c: addb    %al, (%rax)

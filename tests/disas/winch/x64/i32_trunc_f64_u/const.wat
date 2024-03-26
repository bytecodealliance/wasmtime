;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x83
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x5d(%rip), %xmm1
;;   33: movabsq $0x41e0000000000000, %r11
;;   3d: movq    %r11, %xmm15
;;   42: ucomisd %xmm15, %xmm1
;;   47: jae     0x62
;;   4d: jp      0x85
;;   53: cvttsd2si %xmm1, %eax
;;   57: cmpl    $0, %eax
;;   5a: jge     0x7d
;;   60: ud2
;;   62: movaps  %xmm1, %xmm0
;;   65: subsd   %xmm15, %xmm0
;;   6a: cvttsd2si %xmm0, %eax
;;   6e: cmpl    $0, %eax
;;   71: jl      0x87
;;   77: addl    $0x80000000, %eax
;;   7d: addq    $0x10, %rsp
;;   81: popq    %rbp
;;   82: retq
;;   83: ud2
;;   85: ud2
;;   87: ud2
;;   89: addb    %al, (%rax)
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
;;   8f: addb    %al, (%rax)
;;   91: addb    %al, (%rax)
;;   93: addb    %al, (%rax)
;;   95: addb    %dh, %al

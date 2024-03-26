;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x55(%rip), %xmm1
;;   33: movl    $0x4f000000, %r11d
;;   39: movd    %r11d, %xmm15
;;   3e: ucomiss %xmm15, %xmm1
;;   42: jae     0x5d
;;   48: jp      0x80
;;   4e: cvttss2si %xmm1, %eax
;;   52: cmpl    $0, %eax
;;   55: jge     0x78
;;   5b: ud2
;;   5d: movaps  %xmm1, %xmm0
;;   60: subss   %xmm15, %xmm0
;;   65: cvttss2si %xmm0, %eax
;;   69: cmpl    $0, %eax
;;   6c: jl      0x82
;;   72: addl    $0x80000000, %eax
;;   78: addq    $0x10, %rsp
;;   7c: popq    %rbp
;;   7d: retq
;;   7e: ud2
;;   80: ud2
;;   82: ud2
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)

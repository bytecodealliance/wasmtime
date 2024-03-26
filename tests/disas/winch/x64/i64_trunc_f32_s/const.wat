;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x75
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x4d(%rip), %xmm0
;;   33: cvttss2si %xmm0, %rax
;;   38: cmpq    $1, %rax
;;   3c: jno     0x6f
;;   42: ucomiss %xmm0, %xmm0
;;   45: jp      0x77
;;   4b: movl    $0xdf000000, %r11d
;;   51: movd    %r11d, %xmm15
;;   56: ucomiss %xmm15, %xmm0
;;   5a: jb      0x79
;;   60: xorpd   %xmm15, %xmm15
;;   65: ucomiss %xmm0, %xmm15
;;   69: jb      0x7b
;;   6f: addq    $0x10, %rsp
;;   73: popq    %rbp
;;   74: retq
;;   75: ud2
;;   77: ud2
;;   79: ud2
;;   7b: ud2
;;   7d: addb    %al, (%rax)
;;   7f: addb    %al, (%rax)

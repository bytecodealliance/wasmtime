;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x73
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x4d(%rip), %xmm0
;;   33: cvttss2si %xmm0, %eax
;;   37: cmpl    $1, %eax
;;   3a: jno     0x6d
;;   40: ucomiss %xmm0, %xmm0
;;   43: jp      0x75
;;   49: movl    $0xcf000000, %r11d
;;   4f: movd    %r11d, %xmm15
;;   54: ucomiss %xmm15, %xmm0
;;   58: jb      0x77
;;   5e: xorpd   %xmm15, %xmm15
;;   63: ucomiss %xmm0, %xmm15
;;   67: jb      0x79
;;   6d: addq    $0x10, %rsp
;;   71: popq    %rbp
;;   72: retq
;;   73: ud2
;;   75: ud2
;;   77: ud2
;;   79: ud2
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %al, (%rax)

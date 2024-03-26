;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result i32)
        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x78
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm0
;;   38: cvttss2si %xmm0, %eax
;;   3c: cmpl    $1, %eax
;;   3f: jno     0x72
;;   45: ucomiss %xmm0, %xmm0
;;   48: jp      0x7a
;;   4e: movl    $0xcf000000, %r11d
;;   54: movd    %r11d, %xmm15
;;   59: ucomiss %xmm15, %xmm0
;;   5d: jb      0x7c
;;   63: xorpd   %xmm15, %xmm15
;;   68: ucomiss %xmm0, %xmm15
;;   6c: jb      0x7e
;;   72: addq    $0x18, %rsp
;;   76: popq    %rbp
;;   77: retq
;;   78: ud2
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2

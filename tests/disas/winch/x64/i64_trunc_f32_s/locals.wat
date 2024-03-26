;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   4(%rsp), %xmm0
;;   3a: cvttss2si %xmm0, %rax
;;   3f: cmpq    $1, %rax
;;   43: jno     0x76
;;   49: ucomiss %xmm0, %xmm0
;;   4c: jp      0x7e
;;   52: movl    $0xdf000000, %r11d
;;   58: movd    %r11d, %xmm15
;;   5d: ucomiss %xmm15, %xmm0
;;   61: jb      0x80
;;   67: xorpd   %xmm15, %xmm15
;;   6c: ucomiss %xmm0, %xmm15
;;   70: jb      0x82
;;   76: addq    $0x18, %rsp
;;   7a: popq    %rbp
;;   7b: retq
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: ud2

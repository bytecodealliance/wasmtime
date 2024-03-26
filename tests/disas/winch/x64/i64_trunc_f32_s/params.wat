;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result i64)
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
;;   15: ja      0x7a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm0
;;   38: cvttss2si %xmm0, %rax
;;   3d: cmpq    $1, %rax
;;   41: jno     0x74
;;   47: ucomiss %xmm0, %xmm0
;;   4a: jp      0x7c
;;   50: movl    $0xdf000000, %r11d
;;   56: movd    %r11d, %xmm15
;;   5b: ucomiss %xmm15, %xmm0
;;   5f: jb      0x7e
;;   65: xorpd   %xmm15, %xmm15
;;   6a: ucomiss %xmm0, %xmm15
;;   6e: jb      0x80
;;   74: addq    $0x18, %rsp
;;   78: popq    %rbp
;;   79: retq
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: ud2

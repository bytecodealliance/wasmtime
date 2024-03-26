;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (result f64)
        (local.get 0)
        (f64.convert_i64_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x68
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    %rdx, (%rsp)
;;   30: movq    (%rsp), %rcx
;;   34: cmpq    $0, %rcx
;;   38: jl      0x48
;;   3e: cvtsi2sdq %rcx, %xmm0
;;   43: jmp     0x62
;;   48: movq    %rcx, %r11
;;   4b: shrq    $1, %r11
;;   4f: movq    %rcx, %rax
;;   52: andq    $1, %rax
;;   56: orq     %r11, %rax
;;   59: cvtsi2sdq %rax, %xmm0
;;   5e: addsd   %xmm0, %xmm0
;;   62: addq    $0x18, %rsp
;;   66: popq    %rbp
;;   67: retq
;;   68: ud2

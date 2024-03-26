;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        i64.const 1
        f64.convert_i64_u
        block
        end
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
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $1, %rcx
;;   32: cmpq    $0, %rcx
;;   36: jl      0x46
;;   3c: cvtsi2sdq %rcx, %xmm0
;;   41: jmp     0x60
;;   46: movq    %rcx, %r11
;;   49: shrq    $1, %r11
;;   4d: movq    %rcx, %rax
;;   50: andq    $1, %rax
;;   54: orq     %r11, %rax
;;   57: cvtsi2sdq %rax, %xmm0
;;   5c: addsd   %xmm0, %xmm0
;;   60: subq    $8, %rsp
;;   64: movsd   %xmm0, (%rsp)
;;   69: movsd   (%rsp), %xmm0
;;   6e: addq    $8, %rsp
;;   72: addq    $0x10, %rsp
;;   76: popq    %rbp
;;   77: retq
;;   78: ud2

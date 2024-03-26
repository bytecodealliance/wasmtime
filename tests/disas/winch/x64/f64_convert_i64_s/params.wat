;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (result f64)
        (local.get 0)
        (f64.convert_i64_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    %rdx, (%rsp)
;;   30: movq    (%rsp), %rax
;;   34: cvtsi2sdq %rax, %xmm0
;;   39: addq    $0x18, %rsp
;;   3d: popq    %rbp
;;   3e: retq
;;   3f: ud2

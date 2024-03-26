;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.xor)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x48
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movabsq $0x7ffffffffffffffe, %rax
;;   35: movabsq $0x7fffffffffffffff, %r11
;;   3f: xorq    %r11, %rax
;;   42: addq    $0x10, %rsp
;;   46: popq    %rbp
;;   47: retq
;;   48: ud2

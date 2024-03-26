;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i64.const 9223372036854775807)
        (i64.eqz)
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
;;   2b: movabsq $0x7fffffffffffffff, %rax
;;   35: cmpq    $0, %rax
;;   39: movl    $0, %eax
;;   3e: sete    %al
;;   42: addq    $0x10, %rsp
;;   46: popq    %rbp
;;   47: retq
;;   48: ud2

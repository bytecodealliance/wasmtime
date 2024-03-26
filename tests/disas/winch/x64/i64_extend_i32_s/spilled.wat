;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        i32.const 1
        i64.extend_i32_s
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
;;   15: ja      0x3b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: movslq  %eax, %rax
;;   33: pushq   %rax
;;   34: popq    %rax
;;   35: addq    $0x10, %rsp
;;   39: popq    %rbp
;;   3a: retq
;;   3b: ud2

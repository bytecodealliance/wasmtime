;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        i64.const 1
        i32.wrap_i64
        block
        end
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x14, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x48
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $1, %rax
;;   32: movl    %eax, %eax
;;   34: subq    $4, %rsp
;;   38: movl    %eax, (%rsp)
;;   3b: movl    (%rsp), %eax
;;   3e: addq    $4, %rsp
;;   42: addq    $0x10, %rsp
;;   46: popq    %rbp
;;   47: retq
;;   48: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i32.const 1)
        (i32.clz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       bsrl    %eax, %eax
;;       movl    $0, %r11d
;;       setne   %r11b
;;       negl    %eax
;;       addl    $0x20, %eax
;;       subl    %r11d, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4c: ud2

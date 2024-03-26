;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.rem_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $0, %rcx
;;   32: movq    $0, %rax
;;   39: cqto
;;   3b: cmpq    $-1, %rcx
;;   3f: jne     0x4f
;;   45: movl    $0, %edx
;;   4a: jmp     0x52
;;   4f: idivq   %rcx
;;   52: movq    %rdx, %rax
;;   55: addq    $0x10, %rsp
;;   59: popq    %rbp
;;   5a: retq
;;   5b: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.rem_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x60
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movq    %rdx, 8(%rsp)
;;   31: movq    %rcx, (%rsp)
;;   35: movq    (%rsp), %rcx
;;   39: movq    8(%rsp), %rax
;;   3e: cqto
;;   40: cmpq    $-1, %rcx
;;   44: jne     0x54
;;   4a: movl    $0, %edx
;;   4f: jmp     0x57
;;   54: idivq   %rcx
;;   57: movq    %rdx, %rax
;;   5a: addq    $0x20, %rsp
;;   5e: popq    %rbp
;;   5f: retq
;;   60: ud2

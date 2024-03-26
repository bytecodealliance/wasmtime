;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 1)
	(i32.const 0)
	(i32.rem_u)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x41
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0, %ecx
;;   30: movl    $1, %eax
;;   35: xorl    %edx, %edx
;;   37: divl    %ecx
;;   39: movl    %edx, %eax
;;   3b: addq    $0x10, %rsp
;;   3f: popq    %rbp
;;   40: retq
;;   41: ud2

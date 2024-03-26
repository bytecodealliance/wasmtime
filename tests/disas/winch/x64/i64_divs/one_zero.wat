;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0)
	(i64.div_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $0, %rcx
;;   32: movq    $1, %rax
;;   39: cmpq    $0, %rcx
;;   3d: je      0x50
;;   43: cqto
;;   45: idivq   %rcx
;;   48: addq    $0x10, %rsp
;;   4c: popq    %rbp
;;   4d: retq
;;   4e: ud2
;;   50: ud2

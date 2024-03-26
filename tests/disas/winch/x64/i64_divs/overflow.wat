;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
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
;;   15: ja      0x51
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $18446744073709551615, %rcx
;;   32: movabsq $9223372036854775808, %rax
;;   3c: cmpq    $0, %rcx
;;   40: je      0x53
;;   46: cqto
;;   48: idivq   %rcx
;;   4b: addq    $0x10, %rsp
;;   4f: popq    %rbp
;;   50: retq
;;   51: ud2
;;   53: ud2

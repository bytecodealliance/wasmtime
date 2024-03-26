;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 1)
	(i32.const 0)
	(i32.div_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x47
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0, %ecx
;;   30: movl    $1, %eax
;;   35: cmpl    $0, %ecx
;;   38: je      0x49
;;   3e: cltd
;;   3f: idivl   %ecx
;;   41: addq    $0x10, %rsp
;;   45: popq    %rbp
;;   46: retq
;;   47: ud2
;;   49: ud2

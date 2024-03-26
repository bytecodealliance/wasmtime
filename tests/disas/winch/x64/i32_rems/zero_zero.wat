;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 0)
	(i32.const 0)
	(i32.rem_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x53
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0, %ecx
;;   30: movl    $0, %eax
;;   35: cltd
;;   36: cmpl    $-1, %ecx
;;   39: jne     0x49
;;   3f: movl    $0, %edx
;;   44: jmp     0x4b
;;   49: idivl   %ecx
;;   4b: movl    %edx, %eax
;;   4d: addq    $0x10, %rsp
;;   51: popq    %rbp
;;   52: retq
;;   53: ud2

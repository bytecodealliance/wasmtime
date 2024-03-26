;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-if-else") (result i32)
    (if (result i32) (i32.const 1) (then (i32.const 2)) (else (loop (result i32) (i32.const 1))))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: testl   %eax, %eax
;;   32: je      0x42
;;   38: movl    $2, %eax
;;   3d: jmp     0x47
;;   42: movl    $1, %eax
;;   47: addq    $0x10, %rsp
;;   4b: popq    %rbp
;;   4c: retq
;;   4d: ud2

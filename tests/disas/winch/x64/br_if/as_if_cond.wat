;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %ecx
;;   34: movl    $1, %eax
;;   39: testl   %ecx, %ecx
;;   3b: jne     0x58
;;   41: testl   %eax, %eax
;;   43: je      0x53
;;   49: movl    $2, %eax
;;   4e: jmp     0x58
;;   53: movl    $3, %eax
;;   58: addq    $0x18, %rsp
;;   5c: popq    %rbp
;;   5d: retq
;;   5e: ud2

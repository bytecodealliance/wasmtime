;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy)
  (func (export "singular") (param i32) (result i32)
    (if (local.get 0) (then (nop)))
    (if (local.get 0) (then (nop)) (else (nop)))
    (if (result i32) (local.get 0) (then (i32.const 7)) (else (i32.const 8)))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x31
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: addq    $0x10, %rsp
;;   2f: popq    %rbp
;;   30: retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: movq    8(%rdi), %r11
;;   48: movq    (%r11), %r11
;;   4b: addq    $0x18, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0xa9
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    4(%rsp), %eax
;;   74: testl   %eax, %eax
;;   76: je      0x7c
;;   7c: movl    4(%rsp), %eax
;;   80: testl   %eax, %eax
;;   82: je      0x88
;;   88: movl    4(%rsp), %eax
;;   8c: testl   %eax, %eax
;;   8e: je      0x9e
;;   94: movl    $7, %eax
;;   99: jmp     0xa3
;;   9e: movl    $8, %eax
;;   a3: addq    $0x18, %rsp
;;   a7: popq    %rbp
;;   a8: retq
;;   a9: ud2

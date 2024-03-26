;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)

  (func (export "as-if-then") (param i32 i32) (result i32)
   (if (result i32)
    (local.get 0) (then (return (i32.const 3))) (else (local.get 1))
   )
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
;;   55: ja      0x92
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    %ecx, (%rsp)
;;   73: movl    4(%rsp), %eax
;;   77: testl   %eax, %eax
;;   79: je      0x89
;;   7f: movl    $3, %eax
;;   84: jmp     0x8c
;;   89: movl    (%rsp), %eax
;;   8c: addq    $0x18, %rsp
;;   90: popq    %rbp
;;   91: retq
;;   92: ud2

;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-test-operand") (param i32) (result i32)
    (i32.eqz
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 13))
        (else (call $dummy) (i32.const 0))
      )
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
;;   4b: addq    $0x20, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0xcd
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    4(%rsp), %eax
;;   74: testl   %eax, %eax
;;   76: je      0x9e
;;   7c: subq    $8, %rsp
;;   80: movq    %r14, %rdi
;;   83: movq    %r14, %rsi
;;   86: callq   0
;;   8b: addq    $8, %rsp
;;   8f: movq    0x10(%rsp), %r14
;;   94: movl    $0xd, %eax
;;   99: jmp     0xbb
;;   9e: subq    $8, %rsp
;;   a2: movq    %r14, %rdi
;;   a5: movq    %r14, %rsi
;;   a8: callq   0
;;   ad: addq    $8, %rsp
;;   b1: movq    0x10(%rsp), %r14
;;   b6: movl    $0, %eax
;;   bb: cmpl    $0, %eax
;;   be: movl    $0, %eax
;;   c3: sete    %al
;;   c7: addq    $0x18, %rsp
;;   cb: popq    %rbp
;;   cc: retq
;;   cd: ud2

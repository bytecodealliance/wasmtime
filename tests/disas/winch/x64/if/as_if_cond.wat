;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy)
  (func (export "as-if-condition") (param i32) (result i32)
    (if (result i32)
      (if (result i32) (local.get 0)
        (then (i32.const 1)) (else (i32.const 0))
      )
      (then (call $dummy) (i32.const 2))
      (else (call $dummy) (i32.const 3))
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
;;   55: ja      0xd8
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    4(%rsp), %eax
;;   74: testl   %eax, %eax
;;   76: je      0x86
;;   7c: movl    $1, %eax
;;   81: jmp     0x8b
;;   86: movl    $0, %eax
;;   8b: testl   %eax, %eax
;;   8d: je      0xb5
;;   93: subq    $8, %rsp
;;   97: movq    %r14, %rdi
;;   9a: movq    %r14, %rsi
;;   9d: callq   0
;;   a2: addq    $8, %rsp
;;   a6: movq    0x10(%rsp), %r14
;;   ab: movl    $2, %eax
;;   b0: jmp     0xd2
;;   b5: subq    $8, %rsp
;;   b9: movq    %r14, %rdi
;;   bc: movq    %r14, %rsi
;;   bf: callq   0
;;   c4: addq    $8, %rsp
;;   c8: movq    0x10(%rsp), %r14
;;   cd: movl    $3, %eax
;;   d2: addq    $0x18, %rsp
;;   d6: popq    %rbp
;;   d7: retq
;;   d8: ud2

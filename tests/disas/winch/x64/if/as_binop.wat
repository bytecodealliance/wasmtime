;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy)
  (func (export "as-binary-operand") (param i32 i32) (result i32)
    (i32.mul
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 3))
        (else (call $dummy) (i32.const -3))
      )
      (if (result i32) (local.get 1)
        (then (call $dummy) (i32.const 4))
        (else (call $dummy) (i32.const -5))
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
;;   55: ja      0x121
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    %ecx, (%rsp)
;;   73: movl    4(%rsp), %eax
;;   77: testl   %eax, %eax
;;   79: je      0xa1
;;   7f: subq    $8, %rsp
;;   83: movq    %r14, %rdi
;;   86: movq    %r14, %rsi
;;   89: callq   0
;;   8e: addq    $8, %rsp
;;   92: movq    0x10(%rsp), %r14
;;   97: movl    $3, %eax
;;   9c: jmp     0xbe
;;   a1: subq    $8, %rsp
;;   a5: movq    %r14, %rdi
;;   a8: movq    %r14, %rsi
;;   ab: callq   0
;;   b0: addq    $8, %rsp
;;   b4: movq    0x10(%rsp), %r14
;;   b9: movl    $0xfffffffd, %eax
;;   be: movl    (%rsp), %ecx
;;   c1: subq    $4, %rsp
;;   c5: movl    %eax, (%rsp)
;;   c8: testl   %ecx, %ecx
;;   ca: je      0xf2
;;   d0: subq    $4, %rsp
;;   d4: movq    %r14, %rdi
;;   d7: movq    %r14, %rsi
;;   da: callq   0
;;   df: addq    $4, %rsp
;;   e3: movq    0x14(%rsp), %r14
;;   e8: movl    $4, %eax
;;   ed: jmp     0x10f
;;   f2: subq    $4, %rsp
;;   f6: movq    %r14, %rdi
;;   f9: movq    %r14, %rsi
;;   fc: callq   0
;;  101: addq    $4, %rsp
;;  105: movq    0x14(%rsp), %r14
;;  10a: movl    $0xfffffffb, %eax
;;  10f: movl    (%rsp), %ecx
;;  112: addq    $4, %rsp
;;  116: imull   %eax, %ecx
;;  119: movl    %ecx, %eax
;;  11b: addq    $0x18, %rsp
;;  11f: popq    %rbp
;;  120: retq
;;  121: ud2

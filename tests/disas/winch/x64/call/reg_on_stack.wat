;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x24, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0xa4
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %r11d
;;   35: subq    $4, %rsp
;;   39: movl    %r11d, (%rsp)
;;   3d: subq    $4, %rsp
;;   41: movq    %r14, %rdi
;;   44: movq    %r14, %rsi
;;   47: movl    $1, %edx
;;   4c: callq   0
;;   51: addq    $4, %rsp
;;   55: movq    0x14(%rsp), %r14
;;   5a: subq    $4, %rsp
;;   5e: movl    %eax, (%rsp)
;;   61: movq    %r14, %rdi
;;   64: movq    %r14, %rsi
;;   67: movl    $1, %edx
;;   6c: callq   0
;;   71: movq    0x18(%rsp), %r14
;;   76: subq    $4, %rsp
;;   7a: movl    %eax, (%rsp)
;;   7d: movl    (%rsp), %ecx
;;   80: addq    $4, %rsp
;;   84: movl    (%rsp), %eax
;;   87: addq    $4, %rsp
;;   8b: testl   %ecx, %ecx
;;   8d: je      0x9c
;;   93: addq    $4, %rsp
;;   97: jmp     0x9e
;;   9c: ud2
;;   9e: addq    $0x18, %rsp
;;   a2: popq    %rbp
;;   a3: retq
;;   a4: ud2

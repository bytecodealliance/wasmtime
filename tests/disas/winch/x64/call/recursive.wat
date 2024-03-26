;;! target = "x86_64"
;;! test = "winch"

(module
  (func $fibonacci8 (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then
        ;; If n <= 1, return n (base case)
        (local.get $n)
      )
      (else
        ;; Else, return fibonacci(n - 1) + fibonacci(n - 2)
        (i32.add
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 1)) ;; Calculate n - 1
          )
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 2)) ;; Calculate n - 2
          )
        )
      )
    )
  )
  (export "fib" (func $fibonacci8))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0xbc
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %eax
;;   34: cmpl    $1, %eax
;;   37: movl    $0, %eax
;;   3c: setle   %al
;;   40: testl   %eax, %eax
;;   42: je      0x51
;;   48: movl    4(%rsp), %eax
;;   4c: jmp     0xb6
;;   51: movl    4(%rsp), %eax
;;   55: subl    $1, %eax
;;   58: subq    $4, %rsp
;;   5c: movl    %eax, (%rsp)
;;   5f: subq    $4, %rsp
;;   63: movq    %r14, %rdi
;;   66: movq    %r14, %rsi
;;   69: movl    4(%rsp), %edx
;;   6d: callq   0
;;   72: addq    $4, %rsp
;;   76: addq    $4, %rsp
;;   7a: movq    0x10(%rsp), %r14
;;   7f: movl    4(%rsp), %ecx
;;   83: subl    $2, %ecx
;;   86: subq    $4, %rsp
;;   8a: movl    %eax, (%rsp)
;;   8d: subq    $4, %rsp
;;   91: movl    %ecx, (%rsp)
;;   94: movq    %r14, %rdi
;;   97: movq    %r14, %rsi
;;   9a: movl    (%rsp), %edx
;;   9d: callq   0
;;   a2: addq    $4, %rsp
;;   a6: movq    0x14(%rsp), %r14
;;   ab: movl    (%rsp), %ecx
;;   ae: addq    $4, %rsp
;;   b2: addl    %eax, %ecx
;;   b4: movl    %ecx, %eax
;;   b6: addq    $0x18, %rsp
;;   ba: popq    %rbp
;;   bb: retq
;;   bc: ud2

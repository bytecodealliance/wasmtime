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
;; wasm[0]::function[0]::fibonacci8:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xbc
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       cmpl    $1, %eax
;;       movl    $0, %eax
;;       setle   %al
;;       testl   %eax, %eax
;;       je      0x51
;;   48: movl    4(%rsp), %eax
;;       jmp     0xb6
;;   51: movl    4(%rsp), %eax
;;       subl    $1, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    4(%rsp), %edx
;;       callq   0
;;       addq    $4, %rsp
;;       addq    $4, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    4(%rsp), %ecx
;;       subl    $2, %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    (%rsp), %edx
;;       callq   0
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   bc: ud2

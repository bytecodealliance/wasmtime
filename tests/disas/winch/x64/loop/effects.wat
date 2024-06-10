;;! target = "x86_64"
;;! test = "winch"
(module
  (func $fx (export "effects") (result i32)
    (local i32)
    (block
      (loop
        (local.set 0 (i32.const 1))
        (local.set 0 (i32.mul (local.get 0) (i32.const 3)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 5)))
        (local.set 0 (i32.mul (local.get 0) (i32.const 7)))
        (br 1)
        (local.set 0 (i32.mul (local.get 0) (i32.const 100)))
      )
    )
    (i32.eq (local.get 0) (i32.const -14))
  )
)
;; wasm[0]::function[0]::fx:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x74
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    $1, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       imull   $3, %eax, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       subl    $5, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       imull   $7, %eax, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       cmpl    $-0xe, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   74: ud2

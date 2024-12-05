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
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x76
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    $1, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       imull   $3, %eax, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       subl    $5, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       imull   $7, %eax, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       cmpl    $-0xe, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   76: ud2

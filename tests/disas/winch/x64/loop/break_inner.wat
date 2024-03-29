;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "break-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (br 1 (i32.const 0x8)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (loop (result i32) (br 2 (i32.const 0x10))))))))
    (local.get 0)
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x1c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xe2
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    $0, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $1, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $2, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $4, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $8, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $0x10, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   e2: ud2

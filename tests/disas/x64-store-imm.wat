;;! target = "x86_64"
;;! test = "compile"

(module
  (global $g (mut i32) (i32.const 0))

  (func $foo
    (global.set $g (i32.const 0))
    (global.set $g (i32.const 1))
    (global.set $g (i32.const -1))
    (global.set $g (i32.const -10))
    (global.set $g (i32.const 100000))
    (global.set $g (i32.const 0x8fff_ffff))
  )
)
;; wasm[0]::function[0]::foo:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0, 0x50(%rdi)
;;       movl    $1, 0x50(%rdi)
;;       movl    $0xffffffff, 0x50(%rdi)
;;       movl    $0xfffffff6, 0x50(%rdi)
;;       movl    $0x186a0, 0x50(%rdi)
;;       movl    $0x8fffffff, 0x50(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

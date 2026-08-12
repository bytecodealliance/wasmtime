;;! target = "x86_64"
;;! test = "compile"
;;! filter = "wasm[1]"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x20, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xd2
;;   39: subq    $0x20, %rsp
;;       movq    0x48(%rdi), %rdx
;;       movq    0xe8(%rdx), %rax
;;       movl    (%rax), %ecx
;;       testl   %ecx, %ecx
;;       je      0xd4
;;   52: movq    8(%rdx), %rdx
;;       movq    0x88(%rdx), %rsi
;;       leaq    (%rsp), %r8
;;       movq    %rsi, (%rsp)
;;       movl    $2, 8(%rsp)
;;       movl    $0, 0xc(%rsp)
;;       movl    $1, 0x10(%rsp)
;;       movl    0x80(%rdx), %edi
;;       movl    %edi, 0x14(%rsp)
;;       movl    $0, 0x80(%rdx)
;;       movl    0x84(%rdx), %r9d
;;       movl    %r9d, 0x18(%rsp)
;;       movl    $0, 0x84(%rdx)
;;       movq    %r8, 0x88(%rdx)
;;       movq    %rsi, 0x88(%rdx)
;;       movl    %edi, 0x80(%rdx)
;;       movl    %r9d, 0x84(%rdx)
;;       movl    %ecx, (%rax)
;;       movl    $0x4fc, %eax
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d2: ud2
;;   d4: ud2

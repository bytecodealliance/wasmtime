;;! target = "x86_64"
;;! test = 'compile'
;;! filter = "function"
;;! flags = "-C inlining=n"

;; Same as `direct-adapter-calls.wat` but shows full compilation down to x86_64
;; so that we can exercise our linker's ability to resolve relocations for
;; direct calls across Wasm modules.

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

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       leal    0x2a(%rdx), %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x10, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x4f
;;   39: movq    %rdi, %rsi
;;       movq    0x40(%rdi), %rdi
;;       movl    $0x4d2, %edx
;;       callq   0x60
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;
;; wasm[2]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x20, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x120
;;   79: subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r13, 8(%rsp)
;;       movq    0x78(%rdi), %rbx
;;       movl    (%rbx), %eax
;;       testl   $1, %eax
;;       je      0x10b
;;   97: movq    0x60(%rdi), %r13
;;       movq    %rdi, %rsi
;;       movl    (%r13), %eax
;;       testl   $2, %eax
;;       je      0xf9
;;   ad: movq    %rax, %r8
;;       andl    $0xfffffffd, %r8d
;;       movl    %r8d, (%r13)
;;       andl    $0xfffffffc, %eax
;;       movl    %eax, (%r13)
;;       orl     $1, %r8d
;;       movl    %r8d, (%r13)
;;       movq    0x40(%rsi), %rdi
;;       callq   0
;;       movl    (%rbx), %r10d
;;       movq    %r10, %rsi
;;       andl    $0xfffffffe, %esi
;;       movl    %esi, (%rbx)
;;       orl     $1, %r10d
;;       movl    %r10d, (%rbx)
;;       orl     $2, (%r13)
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r13
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   f9: movq    0x48(%rsi), %r8
;;   fd: movq    0x58(%rsi), %rdi
;;  101: movl    $0x12, %edx
;;  106: callq   *%r8
;;  109: ud2
;;  10b: movq    %rdi, %rsi
;;  10e: movq    0x48(%rsi), %r10
;;  112: movq    0x58(%rsi), %rdi
;;  116: movl    $0x18, %edx
;;  11b: callq   *%r10
;;  11e: ud2
;;  120: ud2

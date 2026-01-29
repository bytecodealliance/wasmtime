;;! target = "x86_64"
;;! test = 'compile'
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

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
;;       ja      0xf2
;;   79: subq    $0x10, %rsp
;;       movq    %r12, (%rsp)
;;       movq    0x78(%rdi), %r12
;;       movl    (%r12), %r10d
;;       testl   $1, %r10d
;;       je      0xdd
;;   96: movq    0x60(%rdi), %rsi
;;       movl    (%rsi), %r10d
;;       movq    %r10, %rax
;;       andl    $0xfffffffe, %eax
;;       movl    %eax, (%rsi)
;;       orl     $1, %r10d
;;       movl    %r10d, (%rsi)
;;       movq    %rdi, %rax
;;       movq    0x40(%rax), %rdi
;;       movq    %rax, %rsi
;;       callq   0
;;       movl    (%r12), %esi
;;       movq    %rsi, %rcx
;;       andl    $0xfffffffe, %ecx
;;       movl    %ecx, (%r12)
;;       orl     $1, %esi
;;       movl    %esi, (%r12)
;;       movq    (%rsp), %r12
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   dd: movq    %rdi, %rsi
;;   e0: movq    0x48(%rsi), %r9
;;   e4: movq    0x58(%rsi), %rdi
;;   e8: movl    $0x17, %edx
;;   ed: callq   *%r9
;;   f0: ud2
;;   f2: ud2

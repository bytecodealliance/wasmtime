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
;;       movq    0x18(%r10), %r10
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
;;       movq    0x18(%r10), %r10
;;       addq    $0x20, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xe4
;;   79: subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    0x78(%rdi), %rbx
;;       movl    (%rbx), %eax
;;       testl   $1, %eax
;;       je      0xd0
;;   92: movq    0x60(%rdi), %rax
;;       movl    (%rax), %ecx
;;       movq    %rcx, %rsi
;;       andl    $0xfffffffe, %esi
;;       movl    %esi, (%rax)
;;       orl     $1, %ecx
;;       movl    %ecx, (%rax)
;;       movq    %rdi, %rax
;;       movq    0x40(%rax), %rdi
;;       movq    %rax, %rsi
;;       callq   0
;;       movl    (%rbx), %ecx
;;       movq    %rcx, %rdx
;;       andl    $0xfffffffe, %edx
;;       movl    %edx, (%rbx)
;;       orl     $1, %ecx
;;       movl    %ecx, (%rbx)
;;       movq    (%rsp), %rbx
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d0: movq    %rdi, %rsi
;;   d3: movq    0x48(%rsi), %rax
;;   d7: movq    0x58(%rsi), %rdi
;;   db: movl    $0x17, %edx
;;   e0: callq   *%rax
;;   e2: ud2
;;   e4: ud2

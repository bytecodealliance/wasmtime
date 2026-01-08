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
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x14e
;;   79: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r14, 0x10(%rsp)
;;       movq    %r15, 0x18(%rsp)
;;       movq    0x78(%rdi), %r12
;;       movl    (%r12), %r8d
;;       testl   $1, %r8d
;;       je      0x13a
;;   a5: movq    0x60(%rdi), %rbx
;;       movl    (%rbx), %r9d
;;       testl   $2, %r9d
;;       je      0x126
;;   b9: andl    $0xfffffffd, %r9d
;;       movl    %r9d, (%rbx)
;;       movq    0x90(%rdi), %r14
;;       movq    %rdi, %r10
;;       movl    (%r14), %r15d
;;       movl    $0, (%r14)
;;       movl    (%rbx), %esi
;;       movq    %rsi, %rax
;;       andl    $0xfffffffe, %eax
;;       movl    %eax, (%rbx)
;;       orl     $1, %esi
;;       movl    %esi, (%rbx)
;;       movq    0x40(%r10), %rdi
;;       movq    %r10, %rsi
;;       callq   0
;;       movl    (%r12), %ecx
;;       movq    %rcx, %rdx
;;       andl    $0xfffffffe, %edx
;;       movl    %edx, (%r12)
;;       orl     $1, %ecx
;;       movl    %ecx, (%r12)
;;       orl     $2, (%rbx)
;;       movl    %r15d, (%r14)
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r12
;;       movq    0x10(%rsp), %r14
;;       movq    0x18(%rsp), %r15
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  126: movq    %rdi, %rsi
;;  129: movq    0x48(%rsi), %rax
;;  12d: movq    0x58(%rsi), %rdi
;;  131: movl    $0x12, %edx
;;  136: callq   *%rax
;;  138: ud2
;;  13a: movq    %rdi, %rsi
;;  13d: movq    0x48(%rsi), %rcx
;;  141: movq    0x58(%rsi), %rdi
;;  145: movl    $0x18, %edx
;;  14a: callq   *%rcx
;;  14c: ud2
;;  14e: ud2

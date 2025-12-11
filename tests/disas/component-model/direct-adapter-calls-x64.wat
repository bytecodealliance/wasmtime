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
;; wasm[2]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x169
;;   79: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, %rbx
;;       movq    %rdx, %r13
;;       movq    0x90(%rdi), %r12
;;       movl    (%r12), %ecx
;;       testl   $1, %ecx
;;       je      0x16b
;;   b3: movq    %rbx, %rdi
;;       movq    0x78(%rdi), %r15
;;       movl    (%r15), %r9d
;;       testl   $2, %r9d
;;       je      0x16d
;;   ca: andl    $0xfffffffd, (%r15)
;;       movq    %rbx, %rdi
;;       movq    0x48(%rdi), %r8
;;       movq    0x58(%rdi), %rdi
;;       movl    $2, %edx
;;       movl    $1, %r14d
;;       movq    %r14, %rcx
;;       movq    %rbx, %rsi
;;       callq   *%r8
;;       movq    %rax, (%rsp)
;;       movl    (%r15), %r11d
;;       movq    %r11, %rdi
;;       andl    $0xfffffffe, %edi
;;       movl    %edi, (%r15)
;;       orl     $1, %r11d
;;       movl    %r11d, (%r15)
;;       movq    0x40(%rbx), %rdi
;;       movq    %r13, %rdx
;;       movq    %rbx, %rsi
;;       callq   0
;;       movq    %rax, %r13
;;       movl    (%r12), %edi
;;       movq    %rdi, %rcx
;;       andl    $0xfffffffe, %ecx
;;       movl    %ecx, (%r12)
;;       orl     $1, %edi
;;       movl    %edi, (%r12)
;;       orl     $2, (%r15)
;;       movq    0x60(%rbx), %r9
;;       movq    0x70(%rbx), %rdi
;;       movq    (%rsp), %rcx
;;       movq    %r14, %rdx
;;       movq    %rbx, %rsi
;;       callq   *%r9
;;       movq    %r13, %rax
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  169: ud2
;;  16b: ud2
;;  16d: ud2

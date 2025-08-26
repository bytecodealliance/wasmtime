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
;; wasm[2]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x20, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xfe
;;   79: subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r14, 8(%rsp)
;;       movq    0x60(%rdi), %rbx
;;       movl    (%rbx), %r9d
;;       testl   $1, %r9d
;;       je      0x100
;;   9a: movq    0x48(%rdi), %r14
;;       movq    %rdi, %r11
;;       movl    (%r14), %esi
;;       testl   $2, %esi
;;       je      0x102
;;   b0: movl    (%r14), %ecx
;;       movq    %rcx, %rax
;;       andl    $0xfffffffd, %eax
;;       movl    %eax, (%r14)
;;       andl    $0xfffffffc, %ecx
;;       movl    %ecx, (%r14)
;;       orl     $1, %eax
;;       movl    %eax, (%r14)
;;       movq    %r11, %r10
;;       movq    0x40(%r10), %rdi
;;       movq    %r10, %rsi
;;       callq   0
;;       movl    (%rbx), %edx
;;       movq    %rdx, %r9
;;       andl    $0xfffffffe, %r9d
;;       movl    %r9d, (%rbx)
;;       orl     $1, %edx
;;       movl    %edx, (%rbx)
;;       orl     $2, (%r14)
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   fe: ud2
;;  100: ud2
;;  102: ud2

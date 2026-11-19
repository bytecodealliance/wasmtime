;;! target = "x86_64"
;;! test = "compile"
;;! filter = "wasm[1]::function"
;;! flags = "-Cinlining=y -Wconcurrency-support=y"

;; Like `sync-adapter-lazy-lend.wat`, but does its work in a loop so we can
;; check what gets LICM'd.

(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))

    (core module $M
      (func (export "get'") (param i32) (result i32)
        (local.get 0)
      )
    )
    (core instance $m (instantiate $M))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor))
    )
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $m "get'"))
    )
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "get" (func $get))

    (core func $get' (canon lower (func $get)))

    (core module $M
      (import "" "get" (func $get (param i32) (result i32)))
      (func (export "run") (param i32 i32) (result i32)
        loop
          (call $get (local.get 0))

          local.get 1
          i32.const 0
          i32.ne
          if
            local.get 1
            i32.const 1
            i32.sub
            local.set 1
            br 1
          end

          return
        end
        unreachable
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "get" (func $get'))))))

    (func (export "run") (param "x" u32) (param "n" u32) (result u32)
      (canon lift (core func $m "run"))
    )
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x90, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x2a1
;;   3c: subq    $0x80, %rsp
;;       movq    %rbx, 0x50(%rsp)
;;       movq    %r12, 0x58(%rsp)
;;       movq    %r13, 0x60(%rsp)
;;       movq    %r14, 0x68(%rsp)
;;       movq    %r15, 0x70(%rsp)
;;       movq    0x48(%rdi), %rbx
;;       movq    0x108(%rbx), %r8
;;       movq    8(%rbx), %r13
;;       leaq    (%rsp), %rsi
;;       movq    0x68(%rbx), %r15
;;       leal    -1(%rdx), %edi
;;       imulq   $0x18, %rdi, %r12
;;       movq    0x88(%r13), %r14
;;       movl    (%r8), %eax
;;       testl   %eax, %eax
;;       je      0x2a3
;;   8c: movq    0x80(%r13), %rax
;;       movq    %rax, (%rsp)
;;       movl    $0, 8(%rsp)
;;       movl    $1, 0xc(%rsp)
;;       movl    0x78(%r13), %eax
;;       movl    %eax, 0x10(%rsp)
;;       movl    $0, 0x78(%r13)
;;       movl    0x7c(%r13), %eax
;;       movl    %eax, 0x14(%rsp)
;;       movl    $0, 0x7c(%r13)
;;       movq    0x90(%r13), %r9
;;       movq    %r9, 0x18(%rsp)
;;       movq    %rsi, 0x80(%r13)
;;       cmpq    $8, %r9
;;       jae     0x1a6
;;   e4: movq    0x78(%r15), %rax
;;       cmpq    %rax, %rdi
;;       jae     0x1c4
;;   f1: movq    0x70(%r15), %rax
;;       movl    $0xffffffff, %r10d
;;       addl    (%rax, %r12), %r10d
;;       cmpl    $1, %r10d
;;       ja      0x1e2
;;  109: movl    4(%rax, %r12), %r10d
;;       cmpl    $2, %r10d
;;       jne     0x200
;;  118: movl    8(%rax, %r12), %eax
;;       movl    $2, (%r14, %r9, 8)
;;       movl    %edx, 4(%r14, %r9, 8)
;;       addq    $1, %r9
;;       movq    %r9, 0x90(%r13)
;;       movq    0x80(%r13), %r9
;;       cmpq    %rsi, %r9
;;       jne     0x250
;;  148: movq    (%rsp), %r9
;;       movq    %r9, 0x80(%r13)
;;       movq    0x18(%rsp), %r9
;;       movq    %r9, 0x90(%r13)
;;       movl    0x10(%rsp), %r9d
;;       movl    %r9d, 0x78(%r13)
;;       movl    0x14(%rsp), %r9d
;;       movl    %r9d, 0x7c(%r13)
;;       testl   %ecx, %ecx
;;       je      0x181
;;  179: subl    $1, %ecx
;;       jmp     0x81
;;  181: movq    0x50(%rsp), %rbx
;;       movq    0x58(%rsp), %r12
;;       movq    0x60(%rsp), %r13
;;       movq    0x68(%rsp), %r14
;;       movq    0x70(%rsp), %r15
;;       addq    $0x80, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  1a6: movq    %rdx, 0x20(%rsp)
;;  1ab: movq    %rcx, 0x30(%rsp)
;;  1b0: movq    %rdi, 0x38(%rsp)
;;  1b5: movq    %rsi, 0x40(%rsp)
;;  1ba: movq    %r8, 0x48(%rsp)
;;  1bf: jmp     0x219
;;  1c4: movq    %rdx, 0x20(%rsp)
;;  1c9: movq    %rcx, 0x30(%rsp)
;;  1ce: movq    %rdi, 0x38(%rsp)
;;  1d3: movq    %rsi, 0x40(%rsp)
;;  1d8: movq    %r8, 0x48(%rsp)
;;  1dd: jmp     0x219
;;  1e2: movq    %rdx, 0x20(%rsp)
;;  1e7: movq    %rcx, 0x30(%rsp)
;;  1ec: movq    %rdi, 0x38(%rsp)
;;  1f1: movq    %rsi, 0x40(%rsp)
;;  1f6: movq    %r8, 0x48(%rsp)
;;  1fb: jmp     0x219
;;  200: movq    %rdx, 0x20(%rsp)
;;  205: movq    %rcx, 0x30(%rsp)
;;  20a: movq    %rdi, 0x38(%rsp)
;;  20f: movq    %rsi, 0x40(%rsp)
;;  214: movq    %r8, 0x48(%rsp)
;;  219: movq    0x98(%rbx), %rcx
;;  220: movq    0x20(%rsp), %rdx
;;  225: movq    %rbx, %rsi
;;  228: movq    %r15, %rdi
;;  22b: callq   *%rcx
;;  22d: movq    %rax, 0x28(%rsp)
;;  232: movq    0x30(%rsp), %rcx
;;  237: movq    0x20(%rsp), %rdx
;;  23c: movq    0x40(%rsp), %rsi
;;  241: movq    0x38(%rsp), %rdi
;;  246: movq    0x48(%rsp), %r8
;;  24b: jmp     0x138
;;  250: movq    %rdx, 0x20(%rsp)
;;  255: movq    %rax, 0x28(%rsp)
;;  25a: movq    %rcx, 0x30(%rsp)
;;  25f: movq    %rdi, 0x38(%rsp)
;;  264: movq    %rsi, 0x40(%rsp)
;;  269: movq    %r8, 0x48(%rsp)
;;  26e: movq    0xb8(%rbx), %r11
;;  275: movq    %rbx, %rsi
;;  278: movq    %r15, %rdi
;;  27b: callq   *%r11
;;  27e: movq    0x28(%rsp), %rax
;;  283: movq    0x30(%rsp), %rcx
;;  288: movq    0x20(%rsp), %rdx
;;  28d: movq    0x40(%rsp), %rsi
;;  292: movq    0x38(%rsp), %rdi
;;  297: movq    0x48(%rsp), %r8
;;  29c: jmp     0x171
;;  2a1: ud2
;;  2a3: ud2

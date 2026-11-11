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
;;       movq    0x18(%r10), %r10
;;       addq    $0xb0, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x2a9
;;   3c: subq    $0xa0, %rsp
;;       movq    %rbx, 0x70(%rsp)
;;       movq    %r12, 0x78(%rsp)
;;       movq    %r13, 0x80(%rsp)
;;       movq    %r14, 0x88(%rsp)
;;       movq    %r15, 0x90(%rsp)
;;       movq    %rcx, 0x30(%rsp)
;;       movq    0x48(%rdi), %rax
;;       movq    0x108(%rax), %r12
;;       movq    %r12, 0x60(%rsp)
;;       movq    8(%rax), %r13
;;       movq    %rax, 0x68(%rsp)
;;       leaq    (%rsp), %r14
;;       movq    0x68(%rsp), %r8
;;       movq    %r14, 0x58(%rsp)
;;       movq    0x68(%r8), %rbx
;;       leal    -1(%rdx), %r14d
;;       movq    %rdx, 0x20(%rsp)
;;       imulq   $0x18, %r14, %r15
;;       movq    %r15, %rsi
;;       movq    %r14, 0x50(%rsp)
;;       movq    0x90(%r13), %r12
;;       movq    0x68(%rsp), %rax
;;       movq    %r12, 0x48(%rsp)
;;       movq    0x98(%rax), %r14
;;       movq    %r14, 0x40(%rsp)
;;       movq    0xb8(%rax), %r14
;;       movq    %r14, 0x38(%rsp)
;;       movq    0x60(%rsp), %rax
;;       movl    (%rax), %ecx
;;       testl   %ecx, %ecx
;;       je      0x2ab
;;   e2: movq    0x88(%r13), %rax
;;       movq    %rax, (%rsp)
;;       movl    $0, 8(%rsp)
;;       movl    $1, 0xc(%rsp)
;;       movl    0x80(%r13), %eax
;;       movl    %eax, 0x10(%rsp)
;;       movl    $0, 0x80(%r13)
;;       movl    0x84(%r13), %eax
;;       movl    %eax, 0x14(%rsp)
;;       movl    $0, 0x84(%r13)
;;       movq    0x98(%r13), %rcx
;;       movq    %rcx, 0x18(%rsp)
;;       movq    0x58(%rsp), %r15
;;       movq    %r15, 0x88(%r13)
;;       cmpq    $8, %rcx
;;       jae     0x247
;;  14b: movq    0x78(%rbx), %rax
;;       movq    0x50(%rsp), %rdx
;;       cmpq    %rax, %rdx
;;       jae     0x254
;;  15d: movq    0x70(%rbx), %rdx
;;       movl    $0xffffffff, %eax
;;       movq    %rsi, %r14
;;       addl    (%rdx, %r14), %eax
;;       cmpl    $1, %eax
;;       ja      0x261
;;  176: movl    4(%rdx, %r14), %eax
;;       cmpl    $2, %eax
;;       jne     0x26b
;;  184: movl    8(%rdx, %r14), %eax
;;       movq    0x48(%rsp), %r12
;;       movq    %rax, 0x28(%rsp)
;;       movl    $2, (%r12, %rcx, 8)
;;       movq    0x20(%rsp), %rax
;;       movl    %eax, 4(%r12, %rcx, 8)
;;       leaq    1(%rcx), %rsi
;;       movq    %rsi, 0x98(%r13)
;;       movq    0x88(%r13), %r9
;;       cmpq    %r15, %r9
;;       movq    %r15, 0x58(%rsp)
;;       jne     0x294
;;  1c5: movq    (%rsp), %rax
;;       movq    %rax, 0x88(%r13)
;;       movq    0x18(%rsp), %rax
;;       movq    %rax, 0x98(%r13)
;;       movl    0x10(%rsp), %eax
;;       movl    %eax, 0x80(%r13)
;;       movl    0x14(%rsp), %eax
;;       movl    %eax, 0x84(%r13)
;;       movq    0x30(%rsp), %rcx
;;       testl   %ecx, %ecx
;;       je      0x214
;;  1ff: subl    $1, %ecx
;;       movq    %r14, %rsi
;;       movq    %rcx, 0x30(%rsp)
;;       movq    %r12, 0x48(%rsp)
;;       jmp     0xd3
;;  214: movq    0x28(%rsp), %rax
;;       movq    0x70(%rsp), %rbx
;;       movq    0x78(%rsp), %r12
;;       movq    0x80(%rsp), %r13
;;       movq    0x88(%rsp), %r14
;;       movq    0x90(%rsp), %r15
;;       addq    $0xa0, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  247: movq    0x40(%rsp), %r12
;;  24c: movq    %rsi, %r14
;;  24f: jmp     0x270
;;  254: movq    0x40(%rsp), %r12
;;  259: movq    %rsi, %r14
;;  25c: jmp     0x270
;;  261: movq    0x40(%rsp), %r12
;;  266: jmp     0x270
;;  26b: movq    0x40(%rsp), %r12
;;  270: movq    0x20(%rsp), %rdx
;;  275: movq    0x68(%rsp), %rsi
;;  27a: movq    %rbx, %rdi
;;  27d: callq   *%r12
;;  280: movq    %r12, 0x40(%rsp)
;;  285: movq    0x48(%rsp), %r12
;;  28a: movq    %rax, 0x28(%rsp)
;;  28f: jmp     0x1b0
;;  294: movq    0x38(%rsp), %r15
;;  299: movq    0x68(%rsp), %rsi
;;  29e: movq    %rbx, %rdi
;;  2a1: callq   *%r15
;;  2a4: jmp     0x1f2
;;  2a9: ud2
;;  2ab: ud2

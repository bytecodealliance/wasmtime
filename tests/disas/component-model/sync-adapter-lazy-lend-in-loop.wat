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
;;       addq    $0xa0, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x25a
;;   3c: subq    $0x90, %rsp
;;       movq    %rbx, 0x60(%rsp)
;;       movq    %r12, 0x68(%rsp)
;;       movq    %r13, 0x70(%rsp)
;;       movq    %r14, 0x78(%rsp)
;;       movq    %r15, 0x80(%rsp)
;;       movq    %rcx, 0x30(%rsp)
;;       movq    0x48(%rdi), %rbx
;;       movq    0x108(%rbx), %r12
;;       movq    %r12, 0x50(%rsp)
;;       movq    8(%rbx), %r13
;;       leaq    (%rsp), %r14
;;       movq    %r14, 0x48(%rsp)
;;       movq    0x68(%rbx), %rax
;;       movq    %rax, %rsi
;;       leal    -1(%rdx), %r14d
;;       movq    %rdx, 0x20(%rsp)
;;       imulq   $0x18, %r14, %r15
;;       movq    %r14, 0x40(%rsp)
;;       movq    0x90(%r13), %r12
;;       movq    %r12, 0x38(%rsp)
;;       movq    0x50(%rsp), %rax
;;       movl    (%rax), %ecx
;;       testl   %ecx, %ecx
;;       je      0x25c
;;   b5: movq    0x88(%r13), %rax
;;       movq    %rax, (%rsp)
;;       movl    $0, 8(%rsp)
;;       movl    $1, 0xc(%rsp)
;;       movl    0x80(%r13), %eax
;;       movl    %eax, 0x10(%rsp)
;;       movl    $0, 0x80(%r13)
;;       movl    0x84(%r13), %eax
;;       movl    %eax, 0x14(%rsp)
;;       movl    $0, 0x84(%r13)
;;       movq    0x98(%r13), %rdx
;;       movq    %rdx, 0x18(%rsp)
;;       movq    0x48(%rsp), %r10
;;       movq    %r10, 0x88(%r13)
;;       cmpq    $8, %rdx
;;       jae     0x216
;;  11e: movq    %rsi, %r12
;;       movq    0x78(%r12), %rax
;;       movq    0x40(%rsp), %rcx
;;       cmpq    %rax, %rcx
;;       jae     0x221
;;  134: movq    0x70(%r12), %rcx
;;       movl    $0xffffffff, %eax
;;       movq    %r15, %r14
;;       addl    (%rcx, %r14), %eax
;;       cmpl    $1, %eax
;;       ja      0x224
;;  14e: movl    4(%rcx, %r14), %eax
;;       cmpl    $2, %eax
;;       jne     0x224
;;  15c: movq    0x20(%rsp), %r15
;;       movl    8(%rcx, %r14), %eax
;;       movq    0x38(%rsp), %rcx
;;       movq    %rax, 0x28(%rsp)
;;       movl    $2, (%rcx, %rdx, 8)
;;       movl    %r15d, 4(%rcx, %rdx, 8)
;;       leaq    1(%rdx), %rsi
;;       movq    %rsi, 0x98(%r13)
;;       movq    0x88(%r13), %r9
;;       movq    0x48(%rsp), %r10
;;       cmpq    %r10, %r9
;;       jne     0x245
;;  19c: movq    (%rsp), %rax
;;       movq    %rax, 0x88(%r13)
;;       movq    0x18(%rsp), %rax
;;       movq    %rax, 0x98(%r13)
;;       movl    0x10(%rsp), %eax
;;       movl    %eax, 0x80(%r13)
;;       movl    0x14(%rsp), %eax
;;       movl    %eax, 0x84(%r13)
;;       movq    0x30(%rsp), %rcx
;;       testl   %ecx, %ecx
;;       je      0x1e9
;;  1d6: subl    $1, %ecx
;;       movq    %r12, %rsi
;;       movq    %r14, %r15
;;       movq    %rcx, 0x30(%rsp)
;;       jmp     0xa6
;;  1e9: movq    0x28(%rsp), %rax
;;       movq    0x60(%rsp), %rbx
;;       movq    0x68(%rsp), %r12
;;       movq    0x70(%rsp), %r13
;;       movq    0x78(%rsp), %r14
;;       movq    0x80(%rsp), %r15
;;       addq    $0x90, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  216: movq    %rsi, %r12
;;  219: movq    %r15, %r14
;;  21c: jmp     0x224
;;  221: movq    %r15, %r14
;;  224: movq    0x98(%rbx), %rcx
;;  22b: movq    0x20(%rsp), %r15
;;  230: movq    %r15, %rdx
;;  233: movq    %rbx, %rsi
;;  236: movq    %r12, %rdi
;;  239: callq   *%rcx
;;  23b: movq    %rax, 0x28(%rsp)
;;  240: jmp     0x187
;;  245: movq    0xb8(%rbx), %r11
;;  24c: movq    %rbx, %rsi
;;  24f: movq    %r12, %rdi
;;  252: callq   *%r11
;;  255: jmp     0x1c9
;;  25a: ud2
;;  25c: ud2

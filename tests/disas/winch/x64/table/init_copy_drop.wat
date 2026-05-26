;;! target = "x86_64"
;;! test = "winch"

(module
  (type (func (result i32)))  ;; type #0
  (import "a" "ef0" (func (result i32)))    ;; index 0
  (import "a" "ef1" (func (result i32)))
  (import "a" "ef2" (func (result i32)))
  (import "a" "ef3" (func (result i32)))
  (import "a" "ef4" (func (result i32)))    ;; index 4
  (table $t0 30 30 funcref)
  (table $t1 30 30 funcref)
  (elem (table $t0) (i32.const 2) func 3 1 4 1)
  (elem funcref
    (ref.func 2) (ref.func 7) (ref.func 1) (ref.func 8))
  (elem (table $t0) (i32.const 12) func 7 5 2 3 6)
  (elem funcref
    (ref.func 5) (ref.func 9) (ref.func 2) (ref.func 7) (ref.func 6))
  (func (result i32) (i32.const 5))  ;; index 5
  (func (result i32) (i32.const 6))
  (func (result i32) (i32.const 7))
  (func (result i32) (i32.const 8))
  (func (result i32) (i32.const 9))  ;; index 9
  (func (export "test")
    (table.init $t0 1 (i32.const 7) (i32.const 0) (i32.const 4))
         (elem.drop 1)
         (table.init $t0 3 (i32.const 15) (i32.const 1) (i32.const 3))
         (elem.drop 3)
         (table.copy $t0 0 (i32.const 20) (i32.const 15) (i32.const 5))
         (table.copy $t0 0 (i32.const 21) (i32.const 29) (i32.const 1))
         (table.copy $t0 0 (i32.const 24) (i32.const 10) (i32.const 1))
         (table.copy $t0 0 (i32.const 13) (i32.const 11) (i32.const 4))
         (table.copy $t0 0 (i32.const 19) (i32.const 20) (i32.const 5)))
  (func (export "check") (param i32) (result i32)
    (call_indirect $t0 (type 0) (local.get 0)))
)
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $5, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7d
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $6, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7d: ud2
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xbd
;;   9c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $7, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   bd: ud2
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xfd
;;   dc: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $8, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   fd: ud2
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x13d
;;  11c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $9, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  13d: ud2
;;
;; wasm[0]::function[10]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa2b
;;  15c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x10af
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x10da
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rcx
;;       popq    %rdx
;;       movl    $4, %ebx
;;       movl    $0, %esi
;;       movl    $7, %edi
;;       movl    %ebx, %ebx
;;       movl    %esi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa2d
;;  1ca: cmpl    %edx, %r8d
;;       ja      0xa2f
;;  1d3: movl    %edi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa31
;;  1df: cmpl    %edx, %r8d
;;       ja      0xa33
;;  1e8: movl    %esi, %esi
;;       imulq   $0x10, %rsi, %rsi
;;       addq    %rsi, %rax
;;       cmpq    $0, %rbx
;;       je      0x24f
;;  1fe: movq    (%rax), %rax
;;       addq    $0x10, %rax
;;       movl    %edi, %ecx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %r8
;;       cmpq    %r8, %rcx
;;       jae     0xa35
;;  21c: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r9
;;       addq    %r11, %rdx
;;       cmpq    %r8, %rcx
;;       cmovaeq %r9, %rdx
;;       orq     $1, %rcx
;;       movq    %rcx, (%rdx)
;;       addl    $1, %edi
;;       jmp     0x1f4
;;  24f: movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x1105
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x10af
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x10da
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rcx
;;       popq    %rdx
;;       movl    $3, %ebx
;;       movl    $1, %edi
;;       movl    $0xf, %r8d
;;       movl    %ebx, %ebx
;;       movl    %edi, %r9d
;;       addl    %ebx, %r9d
;;       jb      0xa37
;;  2bd: cmpl    %edx, %r9d
;;       ja      0xa39
;;  2c6: movl    %r8d, %r9d
;;       addl    %ebx, %r9d
;;       jb      0xa3b
;;  2d2: cmpl    %edx, %r9d
;;       ja      0xa3d
;;  2db: movl    %edi, %edi
;;       imulq   $0x10, %rdi, %rdi
;;       addq    %rdi, %rax
;;       cmpq    $0, %rbx
;;       je      0x344
;;  2f1: movq    (%rax), %rax
;;       addq    $0x10, %rax
;;       movl    %r8d, %ecx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %r9
;;       cmpq    %r9, %rcx
;;       jae     0xa3f
;;  310: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r10
;;       addq    %r11, %rdx
;;       cmpq    %r9, %rcx
;;       cmovaeq %r10, %rdx
;;       orq     $1, %rcx
;;       movq    %rcx, (%rdx)
;;       addl    $1, %r8d
;;       jmp     0x2e7
;;  344: movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x1105
;;       movq    8(%rsp), %r14
;;       movl    $5, %eax
;;       movl    $0xf, %ecx
;;       movl    $0x14, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %r8
;;       addq    %rax, %r8
;;       jb      0xa41
;;  381: cmpq    %rbx, %r8
;;       ja      0xa43
;;  38a: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %r8
;;       addq    %rax, %r8
;;       jb      0xa45
;;  3a0: cmpq    %rbx, %r8
;;       ja      0xa47
;;  3a9: cmpq    %rcx, %rdx
;;       jbe     0x3d2
;;  3b2: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x3d7
;;  3d2: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x4b2
;;  3e1: movq    %rcx, %r8
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %r8
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa49
;;  3fe: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r8
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %r8, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x45a
;;  428: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x114d
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x460
;;  45a: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %r8
;;       movq    0xd8(%r8), %r9
;;       cmpq    %r9, %rbx
;;       jae     0xa4b
;;  478: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rbx
;;       cmovaeq %r10, %r8
;;       orq     $1, %rax
;;       movq    %rax, (%r8)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x3d7
;;  4b2: movl    $1, %eax
;;       movl    $0x1d, %ecx
;;       movl    $0x15, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %r8
;;       addq    %rax, %r8
;;       jb      0xa4d
;;  4dd: cmpq    %rbx, %r8
;;       ja      0xa4f
;;  4e6: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %r8
;;       addq    %rax, %r8
;;       jb      0xa51
;;  4fc: cmpq    %rbx, %r8
;;       ja      0xa53
;;  505: cmpq    %rcx, %rdx
;;       jbe     0x52e
;;  50e: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x533
;;  52e: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x60e
;;  53d: movq    %rcx, %r8
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %r8
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa55
;;  55a: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r8
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %r8, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x5b6
;;  584: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x114d
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x5bc
;;  5b6: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %r8
;;       movq    0xd8(%r8), %r9
;;       cmpq    %r9, %rbx
;;       jae     0xa57
;;  5d4: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rbx
;;       cmovaeq %r10, %r8
;;       orq     $1, %rax
;;       movq    %rax, (%r8)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x533
;;  60e: movl    $1, %eax
;;       movl    $0xa, %ecx
;;       movl    $0x18, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %r8
;;       addq    %rax, %r8
;;       jb      0xa59
;;  639: cmpq    %rbx, %r8
;;       ja      0xa5b
;;  642: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %r8
;;       addq    %rax, %r8
;;       jb      0xa5d
;;  658: cmpq    %rbx, %r8
;;       ja      0xa5f
;;  661: cmpq    %rcx, %rdx
;;       jbe     0x68a
;;  66a: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x68f
;;  68a: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x76a
;;  699: movq    %rcx, %r8
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %r8
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa61
;;  6b6: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r8
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %r8, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x712
;;  6e0: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x114d
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x718
;;  712: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %r8
;;       movq    0xd8(%r8), %r9
;;       cmpq    %r9, %rbx
;;       jae     0xa63
;;  730: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rbx
;;       cmovaeq %r10, %r8
;;       orq     $1, %rax
;;       movq    %rax, (%r8)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x68f
;;  76a: movl    $4, %eax
;;       movl    $0xb, %ecx
;;       movl    $0xd, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %r8
;;       addq    %rax, %r8
;;       jb      0xa65
;;  795: cmpq    %rbx, %r8
;;       ja      0xa67
;;  79e: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %r8
;;       addq    %rax, %r8
;;       jb      0xa69
;;  7b4: cmpq    %rbx, %r8
;;       ja      0xa6b
;;  7bd: cmpq    %rcx, %rdx
;;       jbe     0x7e6
;;  7c6: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x7eb
;;  7e6: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x8c6
;;  7f5: movq    %rcx, %r8
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %r8
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa6d
;;  812: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r8
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %r8, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x86e
;;  83c: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x114d
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x874
;;  86e: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %r8
;;       movq    0xd8(%r8), %r9
;;       cmpq    %r9, %rbx
;;       jae     0xa6f
;;  88c: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rbx
;;       cmovaeq %r10, %r8
;;       orq     $1, %rax
;;       movq    %rax, (%r8)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x7eb
;;  8c6: movl    $5, %eax
;;       movl    $0x14, %ecx
;;       movl    $0x13, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %r8
;;       addq    %rax, %r8
;;       jb      0xa71
;;  8f1: cmpq    %rbx, %r8
;;       ja      0xa73
;;  8fa: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %r8
;;       addq    %rax, %r8
;;       jb      0xa75
;;  910: cmpq    %rbx, %r8
;;       ja      0xa77
;;  919: cmpq    %rcx, %rdx
;;       jbe     0x942
;;  922: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x947
;;  942: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0xa22
;;  951: movq    %rcx, %r8
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %r8
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa79
;;  96e: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %r8
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %r8, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x9ca
;;  998: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x114d
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x9d0
;;  9ca: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %r8
;;       movq    0xd8(%r8), %r9
;;       cmpq    %r9, %rbx
;;       jae     0xa7b
;;  9e8: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rbx
;;       cmovaeq %r10, %r8
;;       orq     $1, %rax
;;       movq    %rax, (%r8)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x947
;;  a22: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  a2b: ud2
;;  a2d: ud2
;;  a2f: ud2
;;  a31: ud2
;;  a33: ud2
;;  a35: ud2
;;  a37: ud2
;;  a39: ud2
;;  a3b: ud2
;;  a3d: ud2
;;  a3f: ud2
;;  a41: ud2
;;  a43: ud2
;;  a45: ud2
;;  a47: ud2
;;  a49: ud2
;;  a4b: ud2
;;  a4d: ud2
;;  a4f: ud2
;;  a51: ud2
;;  a53: ud2
;;  a55: ud2
;;  a57: ud2
;;  a59: ud2
;;  a5b: ud2
;;  a5d: ud2
;;  a5f: ud2
;;  a61: ud2
;;  a63: ud2
;;  a65: ud2
;;  a67: ud2
;;  a69: ud2
;;  a6b: ud2
;;  a6d: ud2
;;  a6f: ud2
;;  a71: ud2
;;  a73: ud2
;;  a75: ud2
;;  a77: ud2
;;  a79: ud2
;;  a7b: ud2
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb86
;;  a9c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xb88
;;  ae1: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xb45
;;  b0b: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x114d
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0xb4b
;;  b45: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0xb8a
;;  b54: movq    0x28(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0xb8c
;;  b66: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %rbx
;;       movq    8(%rcx), %rdx
;;       movq    %rbx, %rdi
;;       movq    %r14, %rsi
;;       callq   *%rdx
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  b86: ud2
;;  b88: ud2
;;  b8a: ud2
;;  b8c: ud2

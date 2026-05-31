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
;;       ja      0xa31
;;  15c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x10bf
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x10ea
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
;;       jb      0xa33
;;  1ca: cmpl    %edx, %r8d
;;       ja      0xa35
;;  1d3: movl    %edi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa37
;;  1df: cmpl    %ecx, %r8d
;;       ja      0xa39
;;  1e8: movl    %esi, %esi
;;       imulq   $0x10, %rsi, %rsi
;;       addq    %rsi, %rax
;;       cmpq    $0, %rbx
;;       je      0x256
;;  1fe: movq    (%rax), %rcx
;;       addq    $0x10, %rax
;;       movl    %edi, %edx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %r8
;;       cmpq    %r8, %rdx
;;       jae     0xa3b
;;  21c: movq    %rdx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r9
;;       addq    %r11, %rsi
;;       cmpq    %r8, %rdx
;;       cmovaeq %r9, %rsi
;;       orq     $1, %rcx
;;       movq    %rcx, (%rsi)
;;       addl    $1, %edi
;;       subq    $1, %rbx
;;       jmp     0x1f4
;;  256: movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x1115
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x10bf
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x10ea
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rcx
;;       popq    %rdx
;;       movl    $3, %ebx
;;       movl    $1, %esi
;;       movl    $0xf, %edi
;;       movl    %ebx, %ebx
;;       movl    %esi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa3d
;;  2c3: cmpl    %edx, %r8d
;;       ja      0xa3f
;;  2cc: movl    %edi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa41
;;  2d8: cmpl    %ecx, %r8d
;;       ja      0xa43
;;  2e1: movl    %esi, %esi
;;       imulq   $0x10, %rsi, %rsi
;;       addq    %rsi, %rax
;;       cmpq    $0, %rbx
;;       je      0x34f
;;  2f7: movq    (%rax), %rcx
;;       addq    $0x10, %rax
;;       movl    %edi, %edx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %r8
;;       cmpq    %r8, %rdx
;;       jae     0xa45
;;  315: movq    %rdx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r9
;;       addq    %r11, %rsi
;;       cmpq    %r8, %rdx
;;       cmovaeq %r9, %rsi
;;       orq     $1, %rcx
;;       movq    %rcx, (%rsi)
;;       addl    $1, %edi
;;       subq    $1, %rbx
;;       jmp     0x2ed
;;  34f: movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x1115
;;       movq    8(%rsp), %r14
;;       movl    $5, %eax
;;       movl    $0xf, %ecx
;;       movl    $0x14, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa47
;;  38c: cmpq    %rbx, %rsi
;;       ja      0xa49
;;  395: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa4b
;;  3ab: cmpq    %rbx, %rsi
;;       ja      0xa4d
;;  3b4: cmpq    %rcx, %rdx
;;       jbe     0x3dd
;;  3bd: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x3e2
;;  3dd: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x4bc
;;  3ec: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa4f
;;  408: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x464
;;  432: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x1188
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x46a
;;  464: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa51
;;  482: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x3e2
;;  4bc: movl    $1, %eax
;;       movl    $0x1d, %ecx
;;       movl    $0x15, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa53
;;  4e7: cmpq    %rbx, %rsi
;;       ja      0xa55
;;  4f0: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa57
;;  506: cmpq    %rbx, %rsi
;;       ja      0xa59
;;  50f: cmpq    %rcx, %rdx
;;       jbe     0x538
;;  518: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x53d
;;  538: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x617
;;  547: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa5b
;;  563: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x5bf
;;  58d: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x1188
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x5c5
;;  5bf: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa5d
;;  5dd: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x53d
;;  617: movl    $1, %eax
;;       movl    $0xa, %ecx
;;       movl    $0x18, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa5f
;;  642: cmpq    %rbx, %rsi
;;       ja      0xa61
;;  64b: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa63
;;  661: cmpq    %rbx, %rsi
;;       ja      0xa65
;;  66a: cmpq    %rcx, %rdx
;;       jbe     0x693
;;  673: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x698
;;  693: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x772
;;  6a2: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa67
;;  6be: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x71a
;;  6e8: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x1188
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x720
;;  71a: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa69
;;  738: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x698
;;  772: movl    $4, %eax
;;       movl    $0xb, %ecx
;;       movl    $0xd, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa6b
;;  79d: cmpq    %rbx, %rsi
;;       ja      0xa6d
;;  7a6: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa6f
;;  7bc: cmpq    %rbx, %rsi
;;       ja      0xa71
;;  7c5: cmpq    %rcx, %rdx
;;       jbe     0x7ee
;;  7ce: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x7f3
;;  7ee: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x8cd
;;  7fd: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa73
;;  819: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x875
;;  843: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x1188
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x87b
;;  875: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa75
;;  893: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x7f3
;;  8cd: movl    $5, %eax
;;       movl    $0x14, %ecx
;;       movl    $0x13, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa77
;;  8f8: cmpq    %rbx, %rsi
;;       ja      0xa79
;;  901: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa7b
;;  917: cmpq    %rbx, %rsi
;;       ja      0xa7d
;;  920: cmpq    %rcx, %rdx
;;       jbe     0x949
;;  929: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x94e
;;  949: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0xa28
;;  958: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa7f
;;  974: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x9d0
;;  99e: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x1188
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x9d6
;;  9d0: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa81
;;  9ee: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x94e
;;  a28: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
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
;;  a7d: ud2
;;  a7f: ud2
;;  a81: ud2
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb96
;;  aac: movq    %rdi, %r14
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
;;       jae     0xb98
;;  af1: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xb55
;;  b1b: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x1188
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0xb5b
;;  b55: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0xb9a
;;  b64: movq    0x28(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0xb9c
;;  b76: pushq   %rax
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
;;  b96: ud2
;;  b98: ud2
;;  b9a: ud2
;;  b9c: ud2

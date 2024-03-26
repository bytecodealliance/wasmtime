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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x36
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $5, %eax
;;   30: addq    $0x10, %rsp
;;   34: popq    %rbp
;;   35: retq
;;   36: ud2
;;
;; wasm[0]::function[6]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: movq    8(%rdi), %r11
;;   48: movq    (%r11), %r11
;;   4b: addq    $0x10, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0x76
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x10, %rsp
;;   62: movq    %rdi, 8(%rsp)
;;   67: movq    %rsi, (%rsp)
;;   6b: movl    $6, %eax
;;   70: addq    $0x10, %rsp
;;   74: popq    %rbp
;;   75: retq
;;   76: ud2
;;
;; wasm[0]::function[7]:
;;   80: pushq   %rbp
;;   81: movq    %rsp, %rbp
;;   84: movq    8(%rdi), %r11
;;   88: movq    (%r11), %r11
;;   8b: addq    $0x10, %r11
;;   92: cmpq    %rsp, %r11
;;   95: ja      0xb6
;;   9b: movq    %rdi, %r14
;;   9e: subq    $0x10, %rsp
;;   a2: movq    %rdi, 8(%rsp)
;;   a7: movq    %rsi, (%rsp)
;;   ab: movl    $7, %eax
;;   b0: addq    $0x10, %rsp
;;   b4: popq    %rbp
;;   b5: retq
;;   b6: ud2
;;
;; wasm[0]::function[8]:
;;   c0: pushq   %rbp
;;   c1: movq    %rsp, %rbp
;;   c4: movq    8(%rdi), %r11
;;   c8: movq    (%r11), %r11
;;   cb: addq    $0x10, %r11
;;   d2: cmpq    %rsp, %r11
;;   d5: ja      0xf6
;;   db: movq    %rdi, %r14
;;   de: subq    $0x10, %rsp
;;   e2: movq    %rdi, 8(%rsp)
;;   e7: movq    %rsi, (%rsp)
;;   eb: movl    $8, %eax
;;   f0: addq    $0x10, %rsp
;;   f4: popq    %rbp
;;   f5: retq
;;   f6: ud2
;;
;; wasm[0]::function[9]:
;;  100: pushq   %rbp
;;  101: movq    %rsp, %rbp
;;  104: movq    8(%rdi), %r11
;;  108: movq    (%r11), %r11
;;  10b: addq    $0x10, %r11
;;  112: cmpq    %rsp, %r11
;;  115: ja      0x136
;;  11b: movq    %rdi, %r14
;;  11e: subq    $0x10, %rsp
;;  122: movq    %rdi, 8(%rsp)
;;  127: movq    %rsi, (%rsp)
;;  12b: movl    $9, %eax
;;  130: addq    $0x10, %rsp
;;  134: popq    %rbp
;;  135: retq
;;  136: ud2
;;
;; wasm[0]::function[10]:
;;  140: pushq   %rbp
;;  141: movq    %rsp, %rbp
;;  144: movq    8(%rdi), %r11
;;  148: movq    (%r11), %r11
;;  14b: addq    $0x10, %r11
;;  152: cmpq    %rsp, %r11
;;  155: ja      0x2ad
;;  15b: movq    %rdi, %r14
;;  15e: subq    $0x10, %rsp
;;  162: movq    %rdi, 8(%rsp)
;;  167: movq    %rsi, (%rsp)
;;  16b: movq    %r14, %rdi
;;  16e: movl    $0, %esi
;;  173: movl    $1, %edx
;;  178: movl    $7, %ecx
;;  17d: movl    $0, %r8d
;;  183: movl    $4, %r9d
;;  189: callq   0xaed
;;  18e: movq    8(%rsp), %r14
;;  193: movq    %r14, %rdi
;;  196: movl    $1, %esi
;;  19b: callq   0xb36
;;  1a0: movq    8(%rsp), %r14
;;  1a5: movq    %r14, %rdi
;;  1a8: movl    $0, %esi
;;  1ad: movl    $3, %edx
;;  1b2: movl    $0xf, %ecx
;;  1b7: movl    $1, %r8d
;;  1bd: movl    $3, %r9d
;;  1c3: callq   0xaed
;;  1c8: movq    8(%rsp), %r14
;;  1cd: movq    %r14, %rdi
;;  1d0: movl    $3, %esi
;;  1d5: callq   0xb36
;;  1da: movq    8(%rsp), %r14
;;  1df: movq    %r14, %rdi
;;  1e2: movl    $0, %esi
;;  1e7: movl    $0, %edx
;;  1ec: movl    $0x14, %ecx
;;  1f1: movl    $0xf, %r8d
;;  1f7: movl    $5, %r9d
;;  1fd: callq   0xb75
;;  202: movq    8(%rsp), %r14
;;  207: movq    %r14, %rdi
;;  20a: movl    $0, %esi
;;  20f: movl    $0, %edx
;;  214: movl    $0x15, %ecx
;;  219: movl    $0x1d, %r8d
;;  21f: movl    $1, %r9d
;;  225: callq   0xb75
;;  22a: movq    8(%rsp), %r14
;;  22f: movq    %r14, %rdi
;;  232: movl    $0, %esi
;;  237: movl    $0, %edx
;;  23c: movl    $0x18, %ecx
;;  241: movl    $0xa, %r8d
;;  247: movl    $1, %r9d
;;  24d: callq   0xb75
;;  252: movq    8(%rsp), %r14
;;  257: movq    %r14, %rdi
;;  25a: movl    $0, %esi
;;  25f: movl    $0, %edx
;;  264: movl    $0xd, %ecx
;;  269: movl    $0xb, %r8d
;;  26f: movl    $4, %r9d
;;  275: callq   0xb75
;;  27a: movq    8(%rsp), %r14
;;  27f: movq    %r14, %rdi
;;  282: movl    $0, %esi
;;  287: movl    $0, %edx
;;  28c: movl    $0x13, %ecx
;;  291: movl    $0x14, %r8d
;;  297: movl    $5, %r9d
;;  29d: callq   0xb75
;;  2a2: movq    8(%rsp), %r14
;;  2a7: addq    $0x10, %rsp
;;  2ab: popq    %rbp
;;  2ac: retq
;;  2ad: ud2
;;
;; wasm[0]::function[11]:
;;  2b0: pushq   %rbp
;;  2b1: movq    %rsp, %rbp
;;  2b4: movq    8(%rdi), %r11
;;  2b8: movq    (%r11), %r11
;;  2bb: addq    $0x20, %r11
;;  2c2: cmpq    %rsp, %r11
;;  2c5: ja      0x39d
;;  2cb: movq    %rdi, %r14
;;  2ce: subq    $0x18, %rsp
;;  2d2: movq    %rdi, 0x10(%rsp)
;;  2d7: movq    %rsi, 8(%rsp)
;;  2dc: movl    %edx, 4(%rsp)
;;  2e0: movl    4(%rsp), %r11d
;;  2e5: subq    $4, %rsp
;;  2e9: movl    %r11d, (%rsp)
;;  2ed: movl    (%rsp), %ecx
;;  2f0: addq    $4, %rsp
;;  2f4: movq    %r14, %rdx
;;  2f7: movl    0xf0(%rdx), %ebx
;;  2fd: cmpl    %ebx, %ecx
;;  2ff: jae     0x39f
;;  305: movl    %ecx, %r11d
;;  308: imulq   $8, %r11, %r11
;;  30c: movq    0xe8(%rdx), %rdx
;;  313: movq    %rdx, %rsi
;;  316: addq    %r11, %rdx
;;  319: cmpl    %ebx, %ecx
;;  31b: cmovaeq %rsi, %rdx
;;  31f: movq    (%rdx), %rax
;;  322: testq   %rax, %rax
;;  325: jne     0x359
;;  32b: subq    $4, %rsp
;;  32f: movl    %ecx, (%rsp)
;;  332: subq    $4, %rsp
;;  336: movq    %r14, %rdi
;;  339: movl    $0, %esi
;;  33e: movl    4(%rsp), %edx
;;  342: callq   0xbbe
;;  347: addq    $4, %rsp
;;  34b: addq    $4, %rsp
;;  34f: movq    0x10(%rsp), %r14
;;  354: jmp     0x35d
;;  359: andq    $0xfffffffffffffffe, %rax
;;  35d: testq   %rax, %rax
;;  360: je      0x3a1
;;  366: movq    0x40(%r14), %r11
;;  36a: movl    (%r11), %ecx
;;  36d: movl    0x18(%rax), %edx
;;  370: cmpl    %edx, %ecx
;;  372: jne     0x3a3
;;  378: pushq   %rax
;;  379: popq    %rcx
;;  37a: movq    0x20(%rcx), %rbx
;;  37e: movq    0x10(%rcx), %rdx
;;  382: subq    $8, %rsp
;;  386: movq    %rbx, %rdi
;;  389: movq    %r14, %rsi
;;  38c: callq   *%rdx
;;  38e: addq    $8, %rsp
;;  392: movq    0x10(%rsp), %r14
;;  397: addq    $0x18, %rsp
;;  39b: popq    %rbp
;;  39c: retq
;;  39d: ud2
;;  39f: ud2
;;  3a1: ud2
;;  3a3: ud2

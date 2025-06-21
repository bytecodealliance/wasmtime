;;! target = 'x86_64'
;;! test = 'compile'

(module
  (func $if_b20 (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )
  (func $select_b20 (param i32 i32 i32) (result i32)
    local.get 1
    local.get 2
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    select
  )
  (func $eqz_b20 (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    i32.eqz
  )

  (func $if_b40 (param i64) (result i64)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.const 0
    i64.ne
    if (result i64)
      i64.const 100
    else
      i64.const 400
    end
  )
  (func $select_b40 (param i64 i64 i64) (result i64)
    local.get 1
    local.get 2
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.const 0
    i64.ne
    select
  )
  (func $eqz_b40 (param i64) (result i32)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.eqz
  )

  (func $if_bit32 (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )
  (func $select_bit32 (param i32 i32 i32 i32) (result i32)
    local.get 2
    local.get 3
    (i32.and (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
    select
  )
  (func $eqz_bit32 (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
    i32.eqz
  )

  (func $if_bit64 (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
    i64.const 0
    i64.ne
    if (result i64)
      i64.const 100
    else
      i64.const 200
    end
  )
  (func $select_bit64 (param i64 i64 i64 i64) (result i64)
    local.get 2
    local.get 3
    (i64.and (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
    i64.const 0
    i64.ne
    select
  )
  (func $eqz_bit64 (param i64 i64) (result i32)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
    i64.eqz
  )
)
;; wasm[0]::function[0]::if_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       jne     0x1a
;;   10: movl    $0xc8, %eax
;;       jmp     0x1f
;;   1a: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::select_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       movq    %r8, %rax
;;       cmovnel %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::eqz_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::if_b40:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     $0x28, %rdx
;;       jb      0x99
;;   8f: movl    $0x190, %eax
;;       jmp     0x9e
;;   99: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::select_b40:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     $0x28, %rdx
;;       movq    %r8, %rax
;;       cmovbq  %rcx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]::eqz_b40:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     $0x28, %rdx
;;       setae   %r8b
;;       movzbl  %r8b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]::if_bit32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btl     %ecx, %edx
;;       jb      0x117
;;  10d: movl    $0xc8, %eax
;;       jmp     0x11c
;;  117: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]::select_bit32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btl     %ecx, %edx
;;       movq    %r9, %rax
;;       cmovbl  %r8d, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]::eqz_bit32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btl     %ecx, %edx
;;       setae   %r9b
;;       movzbl  %r9b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]::if_bit64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     %rcx, %rdx
;;       jb      0x198
;;  18e: movl    $0xc8, %eax
;;       jmp     0x19d
;;  198: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[10]::select_bit64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     %rcx, %rdx
;;       movq    %r9, %rax
;;       cmovbq  %r8, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[11]::eqz_bit64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       btq     %rcx, %rdx
;;       setae   %r9b
;;       movzbl  %r9b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

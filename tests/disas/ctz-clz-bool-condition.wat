;;! target = 'x86_64'
;;! test = 'compile'

;; End-to-end check that boolean-context comparisons of `ctz`/`clz` against
;; zero collapse to the corresponding bit test (LSB / sign), per the egraph
;; rewrites in `cranelift/codegen/src/opts/icmp.isle`.
;;
;; Layout per operator/width: three consumers (`if`, `select`, `eqz`) over
;; the explicit `(ctz/clz x) == 0` and `(ctz/clz x) != 0` icmp shapes, plus
;; the wasm-natural `if (ctz/clz x)` form (no icmp interposed) which is what
;; non-Rust frontends like Motoko's `moc` emit.

(module
  ;; ----- ctz, i32 -------------------------------------------------------

  (func $if_ctz_eq0_i32 (param i32) (result i32)
    (i32.eq (i32.ctz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_ne0_i32 (param i32) (result i32)
    (i32.ne (i32.ctz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_bare_i32 (param i32) (result i32)
    (i32.ctz (local.get 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $select_ctz_eq0_i32 (param i32 i32 i32) (result i32)
    local.get 1 local.get 2
    (i32.eq (i32.ctz (local.get 0)) (i32.const 0))
    select)
  (func $eqz_ctz_eq0_i32 (param i32) (result i32)
    (i32.eq (i32.ctz (local.get 0)) (i32.const 0))
    i32.eqz)

  ;; ----- ctz, i64 -------------------------------------------------------

  (func $if_ctz_eq0_i64 (param i64) (result i32)
    (i64.eq (i64.ctz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_ne0_i64 (param i64) (result i32)
    (i64.ne (i64.ctz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  ;; Wasm-natural shape: `i64.ctz` produces i64, narrowed via `i32.wrap_i64`
  ;; before `if`. This is exactly what moc emits for the EOP compactness
  ;; discriminator.
  (func $if_ctz_bare_i64 (param i64) (result i32)
    (i64.ctz (local.get 0)) i32.wrap_i64
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $select_ctz_eq0_i64 (param i64 i32 i32) (result i32)
    local.get 1 local.get 2
    (i64.eq (i64.ctz (local.get 0)) (i64.const 0))
    select)

  ;; ----- clz, i32 (sign-bit tests) --------------------------------------

  (func $if_clz_eq0_i32 (param i32) (result i32)
    (i32.eq (i32.clz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_clz_ne0_i32 (param i32) (result i32)
    (i32.ne (i32.clz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_clz_bare_i32 (param i32) (result i32)
    (i32.clz (local.get 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $select_clz_eq0_i32 (param i32 i32 i32) (result i32)
    local.get 1 local.get 2
    (i32.eq (i32.clz (local.get 0)) (i32.const 0))
    select)

  ;; ----- clz, i64 -------------------------------------------------------

  (func $if_clz_eq0_i64 (param i64) (result i32)
    (i64.eq (i64.clz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_clz_ne0_i64 (param i64) (result i32)
    (i64.ne (i64.clz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)

  ;; ----- negative test: numeric comparison must NOT collapse ------------
  ;; `ctz(x) == 4` is an arithmetic test on the count, not a boolean
  ;; context, so the egraph should leave it alone.
  (func $if_ctz_eq4_i32 (param i32) (result i32)
    (i32.eq (i32.ctz (local.get 0)) (i32.const 4))
    if (result i32) i32.const 100 else i32.const 200 end)
)
;; wasm[0]::function[0]::if_ctz_eq0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $1, %edx
;;       jne     0x1a
;;   10: movl    $0xc8, %eax
;;       jmp     0x1f
;;   1a: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::if_ctz_ne0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $1, %edx
;;       je      0x5a
;;   50: movl    $0xc8, %eax
;;       jmp     0x5f
;;   5a: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::if_ctz_bare_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $1, %edx
;;       je      0x9a
;;   90: movl    $0xc8, %eax
;;       jmp     0x9f
;;   9a: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::select_ctz_eq0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $1, %edx
;;       movq    %r8, %rax
;;       cmovnel %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::eqz_ctz_eq0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $1, %edx
;;       sete    %sil
;;       movzbl  %sil, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]::if_ctz_eq0_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testq   $1, %rdx
;;       jne     0x11b
;;  111: movl    $0xc8, %eax
;;       jmp     0x120
;;  11b: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]::if_ctz_ne0_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testq   $1, %rdx
;;       je      0x15b
;;  151: movl    $0xc8, %eax
;;       jmp     0x160
;;  15b: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]::if_ctz_bare_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0x40, %esi
;;       bsfq    %rdx, %r9
;;       cmoveq  %rsi, %r9
;;       testl   %r9d, %r9d
;;       jne     0x1a4
;;  19a: movl    $0xc8, %eax
;;       jmp     0x1a9
;;  1a4: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]::select_ctz_eq0_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testq   $1, %rdx
;;       movq    %r8, %rax
;;       cmovnel %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]::if_clz_eq0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   %edx, %edx
;;       jl      0x1f6
;;  1ec: movl    $0xc8, %eax
;;       jmp     0x1fb
;;  1f6: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[10]::if_clz_ne0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   %edx, %edx
;;       jge     0x216
;;  20c: movl    $0xc8, %eax
;;       jmp     0x21b
;;  216: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[11]::if_clz_bare_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   %edx, %edx
;;       jge     0x236
;;  22c: movl    $0xc8, %eax
;;       jmp     0x23b
;;  236: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[12]::select_clz_eq0_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   %edx, %edx
;;       movq    %r8, %rax
;;       cmovll  %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[13]::if_clz_eq0_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testq   %rdx, %rdx
;;       jl      0x277
;;  26d: movl    $0xc8, %eax
;;       jmp     0x27c
;;  277: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[14]::if_clz_ne0_i64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testq   %rdx, %rdx
;;       jge     0x2b7
;;  2ad: movl    $0xc8, %eax
;;       jmp     0x2bc
;;  2b7: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[15]::if_ctz_eq4_i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0x20, %esi
;;       bsfl    %edx, %r9d
;;       cmovel  %esi, %r9d
;;       cmpl    $4, %r9d
;;       je      0x305
;;  2fb: movl    $0xc8, %eax
;;       jmp     0x30a
;;  305: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

;;! target = "x86_64"

(module
  (func $main (result i32)
    (local $var i32)
    (call $product (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.div_u))

  (func $product (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.mul))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 48c7042400000000     	mov	qword ptr [rsp], 0
;;   10:	 4883ec10             	sub	rsp, 0x10
;;   14:	 bf14000000           	mov	edi, 0x14
;;   19:	 be50000000           	mov	esi, 0x50
;;   1e:	 e800000000           	call	0x23
;;   23:	 4883c410             	add	rsp, 0x10
;;   27:	 b902000000           	mov	ecx, 2
;;   2c:	 894c2404             	mov	dword ptr [rsp + 4], ecx
;;   30:	 50                   	push	rax
;;   31:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   36:	 4153                 	push	r11
;;   38:	 59                   	pop	rcx
;;   39:	 58                   	pop	rax
;;   3a:	 31d2                 	xor	edx, edx
;;   3c:	 f7f1                 	div	ecx
;;   3e:	 4883c408             	add	rsp, 8
;;   42:	 5d                   	pop	rbp
;;   43:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b0424               	mov	eax, dword ptr [rsp]
;;   12:	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;   16:	 0fafc8               	imul	ecx, eax
;;   19:	 4889c8               	mov	rax, rcx
;;   1c:	 4883c408             	add	rsp, 8
;;   20:	 5d                   	pop	rbp
;;   21:	 c3                   	ret	

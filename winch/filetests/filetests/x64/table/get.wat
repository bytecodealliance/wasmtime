;;! target = "x86_64"
(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
  )
)


;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   14:	 4c89f2               	mov	rdx, r14
;;   17:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   1a:	 39d9                 	cmp	ecx, ebx
;;   1c:	 0f8350000000         	jae	0x72
;;   22:	 4189cb               	mov	r11d, ecx
;;   25:	 4d6bdb08             	imul	r11, r11, 8
;;   29:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   2d:	 4889d6               	mov	rsi, rdx
;;   30:	 4c01da               	add	rdx, r11
;;   33:	 39d9                 	cmp	ecx, ebx
;;   35:	 480f43d6             	cmovae	rdx, rsi
;;   39:	 488b02               	mov	rax, qword ptr [rdx]
;;   3c:	 4885c0               	test	rax, rax
;;   3f:	 0f8523000000         	jne	0x68
;;   45:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   49:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   4d:	 4156                 	push	r14
;;   4f:	 51                   	push	rcx
;;   50:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   55:	 be00000000           	mov	esi, 0
;;   5a:	 8b1424               	mov	edx, dword ptr [rsp]
;;   5d:	 ffd3                 	call	rbx
;;   5f:	 4883c410             	add	rsp, 0x10
;;   63:	 e904000000           	jmp	0x6c
;;   68:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   6c:	 4883c410             	add	rsp, 0x10
;;   70:	 5d                   	pop	rbp
;;   71:	 c3                   	ret	
;;   72:	 0f0b                 	ud2	

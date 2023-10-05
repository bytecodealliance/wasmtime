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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   16:	 4153                 	push	r11
;;   18:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   1c:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   20:	 5b                   	pop	rbx
;;   21:	 4d89f1               	mov	r9, r14
;;   24:	 458b5150             	mov	r10d, dword ptr [r9 + 0x50]
;;   28:	 4439d3               	cmp	ebx, r10d
;;   2b:	 0f8342000000         	jae	0x73
;;   31:	 4189db               	mov	r11d, ebx
;;   34:	 4d6bdb08             	imul	r11, r11, 8
;;   38:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   3c:	 4d89cc               	mov	r12, r9
;;   3f:	 4d01d9               	add	r9, r11
;;   42:	 4439d3               	cmp	ebx, r10d
;;   45:	 4d0f43cc             	cmovae	r9, r12
;;   49:	 4d8b01               	mov	r8, qword ptr [r9]
;;   4c:	 4c89c0               	mov	rax, r8
;;   4f:	 4d85c0               	test	r8, r8
;;   52:	 0f8511000000         	jne	0x69
;;   58:	 4c89f7               	mov	rdi, r14
;;   5b:	 be00000000           	mov	esi, 0
;;   60:	 89da                 	mov	edx, ebx
;;   62:	 ffd1                 	call	rcx
;;   64:	 e904000000           	jmp	0x6d
;;   69:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   6d:	 4883c410             	add	rsp, 0x10
;;   71:	 5d                   	pop	rbp
;;   72:	 c3                   	ret	
;;   73:	 0f0b                 	ud2	

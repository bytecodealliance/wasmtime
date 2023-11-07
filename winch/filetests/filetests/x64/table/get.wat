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
;;   11:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   15:	 4c89f2               	mov	rdx, r14
;;   18:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   1b:	 39d9                 	cmp	ecx, ebx
;;   1d:	 0f8350000000         	jae	0x73
;;   23:	 4189cb               	mov	r11d, ecx
;;   26:	 4d6bdb08             	imul	r11, r11, 8
;;   2a:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   2e:	 4889d6               	mov	rsi, rdx
;;   31:	 4c01da               	add	rdx, r11
;;   34:	 39d9                 	cmp	ecx, ebx
;;   36:	 480f43d6             	cmovae	rdx, rsi
;;   3a:	 488b02               	mov	rax, qword ptr [rdx]
;;   3d:	 4885c0               	test	rax, rax
;;   40:	 0f8523000000         	jne	0x69
;;   46:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   4a:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   4e:	 4156                 	push	r14
;;   50:	 51                   	push	rcx
;;   51:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   56:	 be00000000           	mov	esi, 0
;;   5b:	 8b1424               	mov	edx, dword ptr [rsp]
;;   5e:	 ffd3                 	call	rbx
;;   60:	 4883c410             	add	rsp, 0x10
;;   64:	 e904000000           	jmp	0x6d
;;   69:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   6d:	 4883c410             	add	rsp, 0x10
;;   71:	 5d                   	pop	rbp
;;   72:	 c3                   	ret	
;;   73:	 0f0b                 	ud2	

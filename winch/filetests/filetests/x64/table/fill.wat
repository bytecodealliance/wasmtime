;;! target = "x86_64"
(module
  (type $t0 (func))
  (func $f1 (type $t0))
  (func $f2 (type $t0))
  (func $f3 (type $t0))

  ;; Define two tables of funcref
  (table $t1 3 funcref)
  (table $t2 10 funcref)

  ;; Initialize table $t1 with functions $f1, $f2, $f3
  (elem (i32.const 0) $f1 $f2 $f3)

  ;; Function to fill table $t1 using a function reference from table $t2
  (func (export "fill") (param $i i32) (param $r i32) (param $n i32)
    (local $ref funcref)
    (local.set $ref (table.get $t1 (local.get $r)))
    (table.fill $t2 (local.get $i) (local.get $ref) (local.get $n))
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
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
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
;;    4:	 4883ec20             	sub	rsp, 0x20
;;    8:	 897c241c             	mov	dword ptr [rsp + 0x1c], edi
;;    c:	 89742418             	mov	dword ptr [rsp + 0x18], esi
;;   10:	 89542414             	mov	dword ptr [rsp + 0x14], edx
;;   14:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   19:	 448b5c2418           	mov	r11d, dword ptr [rsp + 0x18]
;;   1e:	 4153                 	push	r11
;;   20:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   24:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   28:	 5b                   	pop	rbx
;;   29:	 4d89f1               	mov	r9, r14
;;   2c:	 458b5150             	mov	r10d, dword ptr [r9 + 0x50]
;;   30:	 4439d3               	cmp	ebx, r10d
;;   33:	 0f8384000000         	jae	0xbd
;;   39:	 4189db               	mov	r11d, ebx
;;   3c:	 4d6bdb08             	imul	r11, r11, 8
;;   40:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   44:	 4d89cc               	mov	r12, r9
;;   47:	 4d01d9               	add	r9, r11
;;   4a:	 4439d3               	cmp	ebx, r10d
;;   4d:	 4d0f43cc             	cmovae	r9, r12
;;   51:	 4d8b01               	mov	r8, qword ptr [r9]
;;   54:	 4c89c0               	mov	rax, r8
;;   57:	 4d85c0               	test	r8, r8
;;   5a:	 0f8511000000         	jne	0x71
;;   60:	 4c89f7               	mov	rdi, r14
;;   63:	 be00000000           	mov	esi, 0
;;   68:	 89da                 	mov	edx, ebx
;;   6a:	 ffd1                 	call	rcx
;;   6c:	 e904000000           	jmp	0x75
;;   71:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   75:	 488944240c           	mov	qword ptr [rsp + 0xc], rax
;;   7a:	 448b5c241c           	mov	r11d, dword ptr [rsp + 0x1c]
;;   7f:	 4153                 	push	r11
;;   81:	 4c8b5c2414           	mov	r11, qword ptr [rsp + 0x14]
;;   86:	 4153                 	push	r11
;;   88:	 448b5c2424           	mov	r11d, dword ptr [rsp + 0x24]
;;   8d:	 4153                 	push	r11
;;   8f:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   93:	 498b4368             	mov	rax, qword ptr [r11 + 0x68]
;;   97:	 4883ec08             	sub	rsp, 8
;;   9b:	 4c89f7               	mov	rdi, r14
;;   9e:	 be01000000           	mov	esi, 1
;;   a3:	 8b542418             	mov	edx, dword ptr [rsp + 0x18]
;;   a7:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   ac:	 448b442408           	mov	r8d, dword ptr [rsp + 8]
;;   b1:	 ffd0                 	call	rax
;;   b3:	 4883c420             	add	rsp, 0x20
;;   b7:	 4883c420             	add	rsp, 0x20
;;   bb:	 5d                   	pop	rbp
;;   bc:	 c3                   	ret	
;;   bd:	 0f0b                 	ud2	

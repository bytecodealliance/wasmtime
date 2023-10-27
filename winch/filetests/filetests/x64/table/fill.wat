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
;;   19:	 8b4c2418             	mov	ecx, dword ptr [rsp + 0x18]
;;   1d:	 4c89f2               	mov	rdx, r14
;;   20:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   23:	 39d9                 	cmp	ecx, ebx
;;   25:	 0f8381000000         	jae	0xac
;;   2b:	 4189cb               	mov	r11d, ecx
;;   2e:	 4d6bdb08             	imul	r11, r11, 8
;;   32:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   36:	 4889d6               	mov	rsi, rdx
;;   39:	 4c01da               	add	rdx, r11
;;   3c:	 39d9                 	cmp	ecx, ebx
;;   3e:	 480f43d6             	cmovae	rdx, rsi
;;   42:	 488b02               	mov	rax, qword ptr [rdx]
;;   45:	 4885c0               	test	rax, rax
;;   48:	 0f8523000000         	jne	0x71
;;   4e:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   52:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   56:	 4156                 	push	r14
;;   58:	 51                   	push	rcx
;;   59:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   5e:	 be00000000           	mov	esi, 0
;;   63:	 8b1424               	mov	edx, dword ptr [rsp]
;;   66:	 ffd3                 	call	rbx
;;   68:	 4883c410             	add	rsp, 0x10
;;   6c:	 e904000000           	jmp	0x75
;;   71:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   75:	 488944240c           	mov	qword ptr [rsp + 0xc], rax
;;   7a:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   7e:	 498b4368             	mov	rax, qword ptr [r11 + 0x68]
;;   82:	 4156                 	push	r14
;;   84:	 4883ec08             	sub	rsp, 8
;;   88:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   8d:	 be01000000           	mov	esi, 1
;;   92:	 8b54242c             	mov	edx, dword ptr [rsp + 0x2c]
;;   96:	 488b4c241c           	mov	rcx, qword ptr [rsp + 0x1c]
;;   9b:	 448b442424           	mov	r8d, dword ptr [rsp + 0x24]
;;   a0:	 ffd0                 	call	rax
;;   a2:	 4883c410             	add	rsp, 0x10
;;   a6:	 4883c420             	add	rsp, 0x20
;;   aa:	 5d                   	pop	rbp
;;   ab:	 c3                   	ret	
;;   ac:	 0f0b                 	ud2	

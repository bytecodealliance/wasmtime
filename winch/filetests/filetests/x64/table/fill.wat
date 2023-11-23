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
;;   14:	 c744241000000000     	mov	dword ptr [rsp + 0x10], 0
;;   1c:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   25:	 4c893424             	mov	qword ptr [rsp], r14
;;   29:	 8b4c2418             	mov	ecx, dword ptr [rsp + 0x18]
;;   2d:	 4c89f2               	mov	rdx, r14
;;   30:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   33:	 39d9                 	cmp	ecx, ebx
;;   35:	 0f83b5000000         	jae	0xf0
;;   3b:	 4189cb               	mov	r11d, ecx
;;   3e:	 4d6bdb08             	imul	r11, r11, 8
;;   42:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   46:	 4889d6               	mov	rsi, rdx
;;   49:	 4c01da               	add	rdx, r11
;;   4c:	 39d9                 	cmp	ecx, ebx
;;   4e:	 480f43d6             	cmovae	rdx, rsi
;;   52:	 488b02               	mov	rax, qword ptr [rdx]
;;   55:	 4885c0               	test	rax, rax
;;   58:	 0f8532000000         	jne	0x90
;;   5e:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   62:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   66:	 4156                 	push	r14
;;   68:	 4883ec04             	sub	rsp, 4
;;   6c:	 890c24               	mov	dword ptr [rsp], ecx
;;   6f:	 4883ec04             	sub	rsp, 4
;;   73:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   78:	 be00000000           	mov	esi, 0
;;   7d:	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;   81:	 ffd3                 	call	rbx
;;   83:	 4883c404             	add	rsp, 4
;;   87:	 4883c40c             	add	rsp, 0xc
;;   8b:	 e904000000           	jmp	0x94
;;   90:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   94:	 488944240c           	mov	qword ptr [rsp + 0xc], rax
;;   99:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   9d:	 498b4368             	mov	rax, qword ptr [r11 + 0x68]
;;   a1:	 4156                 	push	r14
;;   a3:	 448b5c2424           	mov	r11d, dword ptr [rsp + 0x24]
;;   a8:	 4883ec04             	sub	rsp, 4
;;   ac:	 44891c24             	mov	dword ptr [rsp], r11d
;;   b0:	 4c8b5c2418           	mov	r11, qword ptr [rsp + 0x18]
;;   b5:	 4153                 	push	r11
;;   b7:	 448b5c2428           	mov	r11d, dword ptr [rsp + 0x28]
;;   bc:	 4883ec04             	sub	rsp, 4
;;   c0:	 44891c24             	mov	dword ptr [rsp], r11d
;;   c4:	 4883ec08             	sub	rsp, 8
;;   c8:	 488b7c2418           	mov	rdi, qword ptr [rsp + 0x18]
;;   cd:	 be01000000           	mov	esi, 1
;;   d2:	 8b542414             	mov	edx, dword ptr [rsp + 0x14]
;;   d6:	 488b4c240c           	mov	rcx, qword ptr [rsp + 0xc]
;;   db:	 448b442408           	mov	r8d, dword ptr [rsp + 8]
;;   e0:	 ffd0                 	call	rax
;;   e2:	 4883c408             	add	rsp, 8
;;   e6:	 4883c418             	add	rsp, 0x18
;;   ea:	 4883c420             	add	rsp, 0x20
;;   ee:	 5d                   	pop	rbp
;;   ef:	 c3                   	ret	
;;   f0:	 0f0b                 	ud2	

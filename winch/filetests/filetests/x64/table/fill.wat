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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870a000000         	ja	0x22
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   22:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870a000000         	ja	0x22
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   22:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870a000000         	ja	0x22
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   22:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87e8000000         	ja	0x100
;;   18:	 897c241c             	mov	dword ptr [rsp + 0x1c], edi
;;      	 89742418             	mov	dword ptr [rsp + 0x18], esi
;;      	 89542414             	mov	dword ptr [rsp + 0x14], edx
;;      	 c744241000000000     	mov	dword ptr [rsp + 0x10], 0
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c2418             	mov	ecx, dword ptr [rsp + 0x18]
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f83b7000000         	jae	0x102
;;   4b:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8532000000         	jne	0xa0
;;   6e:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;      	 be00000000           	mov	esi, 0
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0xa4
;;   a0:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 488944240c           	mov	qword ptr [rsp + 0xc], rax
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4368             	mov	rax, qword ptr [r11 + 0x68]
;;      	 4156                 	push	r14
;;      	 448b5c2424           	mov	r11d, dword ptr [rsp + 0x24]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4c8b5c2418           	mov	r11, qword ptr [rsp + 0x18]
;;      	 4153                 	push	r11
;;      	 448b5c2428           	mov	r11d, dword ptr [rsp + 0x28]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4883ec08             	sub	rsp, 8
;;      	 488b7c2418           	mov	rdi, qword ptr [rsp + 0x18]
;;      	 be01000000           	mov	esi, 1
;;      	 8b542414             	mov	edx, dword ptr [rsp + 0x14]
;;      	 488b4c240c           	mov	rcx, qword ptr [rsp + 0xc]
;;      	 448b442408           	mov	r8d, dword ptr [rsp + 8]
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4883c418             	add	rsp, 0x18
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  100:	 0f0b                 	ud2	
;;  102:	 0f0b                 	ud2	

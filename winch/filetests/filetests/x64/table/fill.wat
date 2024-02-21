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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x31
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x31
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x31
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8704010000         	ja	0x122
;;   1e:	 4883ec28             	sub	rsp, 0x28
;;      	 48897c2420           	mov	qword ptr [rsp + 0x20], rdi
;;      	 4889742418           	mov	qword ptr [rsp + 0x18], rsi
;;      	 89542414             	mov	dword ptr [rsp + 0x14], edx
;;      	 894c2410             	mov	dword ptr [rsp + 0x10], ecx
;;      	 448944240c           	mov	dword ptr [rsp + 0xc], r8d
;;      	 c744240800000000     	mov	dword ptr [rsp + 8], 0
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 448b5c2410           	mov	r11d, dword ptr [rsp + 0x10]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f83b9000000         	jae	0x124
;;   6b:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8533000000         	jne	0xc1
;;   8e:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742420           	mov	r14, qword ptr [rsp + 0x20]
;;      	 e904000000           	jmp	0xc5
;;   c1:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4889442404           	mov	qword ptr [rsp + 4], rax
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4368             	mov	rax, qword ptr [r11 + 0x68]
;;      	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4c8b5c2408           	mov	r11, qword ptr [rsp + 8]
;;      	 4153                 	push	r11
;;      	 448b5c2418           	mov	r11d, dword ptr [rsp + 0x18]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c89f7               	mov	rdi, r14
;;      	 be01000000           	mov	esi, 1
;;      	 8b542414             	mov	edx, dword ptr [rsp + 0x14]
;;      	 488b4c240c           	mov	rcx, qword ptr [rsp + 0xc]
;;      	 448b442408           	mov	r8d, dword ptr [rsp + 8]
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 4c8b742420           	mov	r14, qword ptr [rsp + 0x20]
;;      	 4883c428             	add	rsp, 0x28
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  122:	 0f0b                 	ud2	
;;  124:	 0f0b                 	ud2	

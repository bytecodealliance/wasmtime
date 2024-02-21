;;! target = "x86_64"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8742000000         	ja	0x60
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48891424             	mov	qword ptr [rsp], rdx
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b50             	mov	rbx, qword ptr [r11 + 0x50]
;;      	 4c8b1c24             	mov	r11, qword ptr [rsp]
;;      	 4153                 	push	r11
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba0a000000           	mov	edx, 0xa
;;      	 488b0c24             	mov	rcx, qword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c408             	add	rsp, 8
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   60:	 0f0b                 	ud2	

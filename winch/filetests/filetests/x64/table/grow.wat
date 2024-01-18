;;! target = "x86_64"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8739000000         	ja	0x51
;;   18:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b50             	mov	rbx, qword ptr [r11 + 0x50]
;;      	 4156                 	push	r14
;;      	 4c8b5c2410           	mov	r11, qword ptr [rsp + 0x10]
;;      	 4153                 	push	r11
;;      	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;      	 be00000000           	mov	esi, 0
;;      	 ba0a000000           	mov	edx, 0xa
;;      	 488b0c24             	mov	rcx, qword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c410             	add	rsp, 0x10
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   51:	 0f0b                 	ud2	

;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i32)
        (local.get 0)
        (local.get 1)
        (i64.ge_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8734000000         	ja	0x52
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4889542408           	mov	qword ptr [rsp + 8], rdx
;;      	 48890c24             	mov	qword ptr [rsp], rcx
;;      	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 4839c1               	cmp	rcx, rax
;;      	 b900000000           	mov	ecx, 0
;;      	 400f9dc1             	setge	cl
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   52:	 0f0b                 	ud2	

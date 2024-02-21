;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
        (i32.clz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872d000000         	ja	0x4b
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b801000000           	mov	eax, 1
;;      	 0fbdc0               	bsr	eax, eax
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f95c3             	setne	r11b
;;      	 f7d8                 	neg	eax
;;      	 83c020               	add	eax, 0x20
;;      	 4429d8               	sub	eax, r11d
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4b:	 0f0b                 	ud2	

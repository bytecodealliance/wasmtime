;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.nearest)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x30
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100514000000     	movsd	xmm0, qword ptr [rip + 0x14]
;;      	 660f3a0bc000         	roundsd	xmm0, xmm0, 0
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   30:	 0f0b                 	ud2	
;;   32:	 0000                 	add	byte ptr [rax], al
;;   34:	 0000                 	add	byte ptr [rax], al
;;   36:	 0000                 	add	byte ptr [rax], al

;;! target = "x86_64"

(module
    (func (result f64)
        f64.const 1.0
        i64.reinterpret_f64
        drop
        f64.const 1.0
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8723000000         	ja	0x3e
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100515000000     	movsd	xmm0, qword ptr [rip + 0x15]
;;      	 66480f7ec0           	movq	rax, xmm0
;;      	 f20f100508000000     	movsd	xmm0, qword ptr [rip + 8]
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3e:	 0f0b                 	ud2	
;;   40:	 0000                 	add	byte ptr [rax], al
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 0000                 	add	byte ptr [rax], al

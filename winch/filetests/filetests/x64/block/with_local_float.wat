;;! target = "x86_64"

(module
  (func (export "") (param f32) (result f32)
    local.get 0
    block
    end
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c31c000000       	add	r11, 0x1c
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8734000000         	ja	0x52
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f30f11442404         	movss	dword ptr [rsp + 4], xmm0
;;      	 f3440f107c2404       	movss	xmm15, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 f3440f113c24         	movss	dword ptr [rsp], xmm15
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   52:	 0f0b                 	ud2	

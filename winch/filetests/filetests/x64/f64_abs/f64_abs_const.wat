;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.abs)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;   14:	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;   1e:	 664d0f6efb           	movq	xmm15, r11
;;   23:	 66410f54c7           	andpd	xmm0, xmm15
;;   28:	 4883c408             	add	rsp, 8
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
;;   2e:	 0000                 	add	byte ptr [rax], al

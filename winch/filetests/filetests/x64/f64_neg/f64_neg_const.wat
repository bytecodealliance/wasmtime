;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.neg)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;   14:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   1e:	 664d0f6efb           	movq	xmm15, r11
;;   23:	 66410f57c7           	xorpd	xmm0, xmm15
;;   28:	 4883c408             	add	rsp, 8
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
;;   2e:	 0000                 	add	byte ptr [rax], al

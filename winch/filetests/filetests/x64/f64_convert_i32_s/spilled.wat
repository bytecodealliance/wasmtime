;;! target = "x86_64"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_s
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 f20f2ac0             	cvtsi2sd	xmm0, eax
;;   15:	 4883ec08             	sub	rsp, 8
;;   19:	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;   1e:	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;   23:	 4883c408             	add	rsp, 8
;;   27:	 4883c408             	add	rsp, 8
;;   2b:	 5d                   	pop	rbp
;;   2c:	 c3                   	ret	

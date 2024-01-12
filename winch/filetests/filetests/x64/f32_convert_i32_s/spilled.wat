;;! target = "x86_64"

(module
    (func (result f32)
        i32.const 1
        f32.convert_i32_s
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 f30f2ac0             	cvtsi2ss	xmm0, eax
;;   15:	 4883ec04             	sub	rsp, 4
;;   19:	 f30f110424           	movss	dword ptr [rsp], xmm0
;;   1e:	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;   23:	 4883c404             	add	rsp, 4
;;   27:	 4883c408             	add	rsp, 8
;;   2b:	 5d                   	pop	rbp
;;   2c:	 c3                   	ret	

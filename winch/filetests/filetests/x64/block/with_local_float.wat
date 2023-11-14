;;! target = "x86_64"

(module
  (func (export "") (param f32) (result f32)
    local.get 0
    block
    end
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f3440f107c240c       	movss	xmm15, dword ptr [rsp + 0xc]
;;   19:	 4883ec04             	sub	rsp, 4
;;   1d:	 f3440f113c24         	movss	dword ptr [rsp], xmm15
;;   23:	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;   28:	 4883c404             	add	rsp, 4
;;   2c:	 4883c410             	add	rsp, 0x10
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	

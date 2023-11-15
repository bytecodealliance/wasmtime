;;! target = "x86_64"

(module
  (func (export "") (param i32)
    local.get 0
    block
    end
    local.set 0
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   15:	 4883ec04             	sub	rsp, 4
;;   19:	 44891c24             	mov	dword ptr [rsp], r11d
;;   1d:	 8b0424               	mov	eax, dword ptr [rsp]
;;   20:	 4883c404             	add	rsp, 4
;;   24:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   28:	 4883c410             	add	rsp, 0x10
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	

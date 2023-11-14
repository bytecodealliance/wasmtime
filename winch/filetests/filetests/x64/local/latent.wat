;;! target = "x86_64"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    local.set 0
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   16:	 4153                 	push	r11
;;   18:	 b801000000           	mov	eax, 1
;;   1d:	 89442414             	mov	dword ptr [rsp + 0x14], eax
;;   21:	 58                   	pop	rax
;;   22:	 4883c410             	add	rsp, 0x10
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	

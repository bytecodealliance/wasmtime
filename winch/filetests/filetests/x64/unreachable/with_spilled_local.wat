;;! target = "x86_64"

(module
  (func (export "")
    (local i32)
    local.get 0
    block
    end
    unreachable
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   1a:	 4883ec04             	sub	rsp, 4
;;   1e:	 44891c24             	mov	dword ptr [rsp], r11d
;;   22:	 0f0b                 	ud2	
;;   24:	 4883c410             	add	rsp, 0x10
;;   28:	 5d                   	pop	rbp
;;   29:	 c3                   	ret	

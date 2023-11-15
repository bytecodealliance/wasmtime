;;! target = "x86_64"

(module
  (func (export "")
    (local i32)
    local.get 0
    if
      local.get 0
      block
      end
      unreachable
    else
      nop
    end
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   19:	 85c0                 	test	eax, eax
;;   1b:	 0f840f000000         	je	0x30
;;   21:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   26:	 4883ec04             	sub	rsp, 4
;;   2a:	 44891c24             	mov	dword ptr [rsp], r11d
;;   2e:	 0f0b                 	ud2	
;;   30:	 4883c410             	add	rsp, 0x10
;;   34:	 5d                   	pop	rbp
;;   35:	 c3                   	ret	

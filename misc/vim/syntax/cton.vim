" Vim syntax file
" Language:     Cretonne
" Maintainer:   Jakob Stoklund Olesen <stoklund@2pi.dk
" Last Change:  Jun 16, 2017

if version < 600
  syntax clear
elseif exists("b:current_syntax")
  finish
endif

" Disable spell checking even in comments.
" They tend to refer to weird stuff like assembler mnemonics anyway.
syn spell notoplevel

syn keyword ctonHeader test isa set
syn keyword ctonDecl function jump_table incoming_arg outgoing_arg spill_slot local emergency_slot
syn keyword ctonFilecheck check sameln nextln unordered not regex contained

syn match ctonType  /\<\([bif]\d\+\(x\d\+\)\?\)\|[if]flags\>/
syn match ctonEntity /\<\(v\|ss\|jt\|fn\|sig\)\d\+\>/
syn match ctonLabel /\<ebb\d+\>/
syn match ctonName /%\w\+\>/

syn match ctonNumber /-\?\<[0-9_]\+\>/
syn match ctonNumber /-\?\<0x[0-9a-fA-F_]\+\(\.[0-9a-fA-F_]*\)\?\(p[+-]\?\d\+\)\?\>/
syn match ctonHexSeq /#\x\+\>/
syn match ctonSourceLoc /@[0-9a-f]\+\>/

syn region ctonCommentLine start=";" end="$" contains=ctonFilecheck

hi def link ctonHeader        Keyword
hi def link ctonDecl          Keyword
hi def link ctonType          Type
hi def link ctonEntity        Identifier
hi def link ctonLabel         Label
hi def link ctonName          String
hi def link ctonNumber        Number
hi def link ctonHexSeq        Number
hi def link ctonCommentLine   Comment
hi def link ctonFilecheck     SpecialComment
hi def link ctonSourceLoc     LineNr

let b:current_syntax = "cton"

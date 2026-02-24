(require "./std.scm")

; This is a test file for messing around with feldspar
; It automatically connects to groq for now.

(set-model groq "openai/gpt-oss-20b" (lookup-env "GROQ_KEY"))

(define history (new-history))
(define chat (new-chat history))

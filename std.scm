;; Feldspar Standard Library

;; Model API URLs (base URLs - genai appends the path)
(define OPENAI_URL "https://api.openai.com/v1/")
(define ANTHROPIC_URL "https://api.anthropic.com/v1/")
(define OLLAMA_URL "http://localhost:11434/v1/")
(define GROQ_URL "https://api.groq.com/openai/v1/")

(define (list-providers)
  (displayln "openai    - OpenAI API")
  (displayln "anthropic - Anthropic API")
  (displayln "ollama    - Local Ollama")
  (displayln "groq      - Groq API")
  (displayln "custom <url>"))

;; Configure model
;; Usage:
;;   (set-model openai "gpt-4o" (lookup-env "OPENAI_API_KEY"))
;;   (set-model anthropic "claude-sonnet-4-20250514" (lookup-env "ANTHROPIC_API_KEY"))
;;   (set-model ollama "llama3" "")
;;   (set-model groq "llama-3.1-70b-versatile" (lookup-env "GROQ_API_KEY"))
;;   (set-model custom "openai" "https://api.example.com/v1" "model-name" "token")
(define-syntax set-model
  (syntax-rules (openai anthropic ollama groq custom)
    [(set-model openai model token)
     (configure-model OPENAI_URL token model "openai")]
    [(set-model anthropic model token)
     (configure-model ANTHROPIC_URL token model "anthropic")]
    [(set-model ollama model token)
     (configure-model OLLAMA_URL token model "ollama")]
    [(set-model groq model token)
     (configure-model GROQ_URL token model "groq")]
    [(set-model custom adapter url model token)
     (configure-model url token model adapter)]))

;; ============================================================
;; Chat with persistent history
;; ============================================================

;; A chat session uses a boxed history for mutability
;; Usage:
;;   (define my-history (new-history))
;;   (define chat (new-chat my-history))
;;   (chat "Hello!")
;;   (chat "Follow up")
;;   (unbox my-history)  ;; Get the history list
;;   (clear-history my-history)
(define (new-chat history-box)
  (lambda (user-message)
    (let ((response (prompt (unbox history-box) user-message)))
      (set-box! history-box
                (append (unbox history-box)
                        (list (list "user" user-message)
                              (list "assistant" response))))
      response)))

;; Create a new history box
(define (new-history)
  (box '()))

;; Clear a history box
(define (clear-history history-box)
  (set-box! history-box '())
  "History cleared")

;; Pretty print a history box
(define (pretty-history history-box)
  (for-each
    (lambda (entry)
      (let ((role (car entry))
            (content (cadr entry)))
        (displayln (string-append
          (if (equal? role "user") "You: " "AI: ")
          content))
        (displayln "")))
    (unbox history-box)))

(provide
  list-providers
  set-model
  new-chat
  new-history
  clear-history
  pretty-history)

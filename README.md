# Feldspar
s expression based agent runtime  

The pitch is really quite simple. I want this to be possible.

Instead of an AGENT.md I want AGENT.scm

```scheme
(use-feldspar)

(model anthropic "opus-4.6" (load-env "CLAUDE_TOKEN"))
(defined history (new-history))
(define chat (new-chat history))

(define research-summary
  (create-tool
    :name "research"
    :input ((topic . string))
    :output (summary . string)
    :desc "Research a topic, and summarize it"
    :tool (-> web-search summarize)))

(register-tool research)
```

Then to launch a session where the agent has access to the defined tools.

```bash
feldspar chat
```

Or a repl ...

```
$ feldspar ./AGENT.scm
> (chat "tell me about kangaroos")
=> "Kangaroos are ..."
```

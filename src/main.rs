use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fmt::{self, Display, Write};
use std::fs;
use std::{cell::RefCell, env};
use steel::steel_vm::engine::Engine;
use steel::steel_vm::register_fn::RegisterFn;
use steel_derive::Steel;

struct ModelConfig {
    url: String,
    token: String,
    model: String,
    adapter: String,
}

#[derive(Clone, Debug, PartialEq, Steel)]
enum ToolSchema {
    Number,
    String,
    Bool,
}

impl fmt::Display for ToolSchema {
    fn fmt(self: &Self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ToolSchema::Number => write!(f, "<number>"),
            ToolSchema::String => write!(f, "<string>"),
            ToolSchema::Bool => write!(f, "<bool>"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Steel)]
struct Tool {
    name: String,
    description: String,
    schema: Vec<(String, ToolSchema)>,
    handler: String,
}

impl Tool {
    pub fn new(
        name: String,
        desc: String,
        schema: Vec<(String, ToolSchema)>,
        handler: String,
    ) -> Tool {
        return Tool {
            name,
            description: desc,
            schema,
            handler,
        };
    }

    pub fn describe(&self) -> String {
        let mut d = String::new();
        writeln!(d, "Name: {}", self.name).unwrap();
        writeln!(d, "Description: {}", self.description).unwrap();
        let mut pretty_schema = String::new();
        for (k, t) in self.schema.clone() {
            writeln!(pretty_schema, "{{\"{}\": {} }},", k, t).unwrap();
        }
        writeln!(d, "Schema: [\n {}\n]", self.description).unwrap();
        writeln!(d, "Description: {}", self.description).unwrap();
        return d;
    }
}

fn register_std_tool(engine: &mut Engine) {
    // Tool types and functions
    engine
        .register_type::<Tool>("tool")
        .register_fn("tool/new", Tool::new)
        .register_fn("tool/describe", Tool::describe);

    engine
        .register_type::<ToolSchema>("tool-schema")
        .register_fn("tool-schema/string", || ToolSchema::String)
        .register_fn("tool-schema/number", || ToolSchema::Number)
        .register_fn("tool-schema/bool", || ToolSchema::Bool);
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            url: "https://api.openai.com/v1".to_string(),
            token: String::new(),
            model: "gpt-4o-mini".to_string(),
            adapter: "openai".to_string(),
        }
    }
}

thread_local! {
    static TOKIO_RT: RefCell<tokio::runtime::Runtime> = RefCell::new(
        tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")
    );
    // I think this shouldn't be like this
    static MODEL_CONFIG: RefCell<ModelConfig> = RefCell::new(ModelConfig::default());
}

fn adapter_from_string(s: &str) -> AdapterKind {
    match s {
        "openai" => AdapterKind::OpenAI,
        "anthropic" => AdapterKind::Anthropic,
        "ollama" => AdapterKind::Ollama,
        "gemini" => AdapterKind::Gemini,
        "groq" => AdapterKind::Groq,
        "cohere" => AdapterKind::Cohere,
        _ => AdapterKind::OpenAI, // default to OpenAI-compatible
    }
}

/// Prompt with history. Takes a list of (role, content) pairs as Vec<Vec<String>>
/// where each inner vec is ["user"|"assistant"|"system", "content"]
fn prompt(history: Vec<Vec<String>>, user_prompt: String) -> String {
    let (url, token, model, adapter) = MODEL_CONFIG.with(|config| {
        let c = config.borrow();
        (
            c.url.clone(),
            c.token.clone(),
            c.model.clone(),
            c.adapter.clone(),
        )
    });

    let adapter_kind = adapter_from_string(&adapter);

    // Create resolver for custom endpoint
    let target_resolver =
        ServiceTargetResolver::from_resolver_fn(move |service_target: ServiceTarget| {
            Ok(ServiceTarget {
                endpoint: Endpoint::from_owned(url.clone()),
                auth: AuthData::from_single(token.clone()),
                model: ModelIden::new(adapter_kind, service_target.model.model_name),
            })
        });

    let client = Client::builder()
        .with_service_target_resolver(target_resolver)
        .build();

    TOKIO_RT.with(|rt| {
        rt.borrow().block_on(async {
            // Build messages from history
            let mut messages: Vec<ChatMessage> = history
                .iter()
                .filter_map(|entry| {
                    if entry.len() >= 2 {
                        let role = &entry[0];
                        let content = &entry[1];
                        match role.as_str() {
                            "user" => Some(ChatMessage::user(content)),
                            "assistant" => Some(ChatMessage::assistant(content)),
                            "system" => Some(ChatMessage::system(content)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();

            // Add the new user prompt
            messages.push(ChatMessage::user(user_prompt));

            let chat_req = ChatRequest::new(messages);

            match client.exec_chat(&model, chat_req, None).await {
                Ok(response) => response
                    .content_text_as_str()
                    .unwrap_or("No response")
                    .to_string(),
                Err(e) => format!("Error: {}", e),
            }
        })
    })
}

fn configure_model(url: String, token: String, model: String, adapter: String) {
    MODEL_CONFIG.with(|config| {
        let mut config = config.borrow_mut();
        config.url = url.clone();
        config.token = token.clone();
        config.model = model.clone();
        config.adapter = adapter.clone();
    });
    println!(
        "Configured: adapter={}, model={}, url={}",
        adapter, model, url
    );
}

fn lookup_env(v: String) -> Result<String, String> {
    match env::var(&v) {
        Ok(s) => Ok(s),
        Err(e) => Err(e.to_string()),
    }
}

fn register_std_io(engine: &mut Engine) {
    engine.register_fn("lookup-env", lookup_env);
}

fn register_std_chat(engine: &mut Engine) {
    engine.register_fn("prompt", prompt);
    engine.register_fn("configure-model", configure_model);
}

fn init() -> Engine {
    let mut engine = Engine::new_sandboxed();

    register_std_chat(&mut engine);
    register_std_io(&mut engine);
    register_std_tool(&mut engine);

    println!("Type :help for commands\n");

    engine
}

fn print_help() {
    println!("Commands:");
    println!("  :help         (:h)  Show this help");
    println!("  :load <file>  (:l)  Load a .scm file");
    println!("  :quit         (:q)  Exit the REPL");
    println!("Functions:");
    println!("  (chat <string>)                                    Prompt the AI");
}

fn handle_command(cmd: &str, engine: &mut Engine) -> Option<bool> {
    let parts: Vec<&str> = cmd[1..].splitn(2, ' ').collect();
    let command = parts[0];
    let arg = parts.get(1).map(|s| s.trim());

    match command {
        "h" | "help" => print_help(),
        "q" | "quit" => return Some(true),
        "l" | "load" => {
            let Some(path) = arg else {
                eprintln!("Usage: :load <file.scm>");
                return Some(false);
            };
            match fs::read_to_string(path) {
                Ok(contents) => match engine.run(contents) {
                    Ok(_) => println!("Loaded {}", path),
                    Err(e) => eprintln!("Error in {}: {}", path, e),
                },
                Err(e) => eprintln!("Error reading {}: {}", path, e),
            }
        }
        _ => eprintln!(
            "Unknown command: {}. Type :help for available commands.",
            command
        ),
    }
    Some(false)
}

fn repl(mut engine: Engine) {
    let mut rl = DefaultEditor::new().expect("Failed to create editor");

    // Load history from file if it exists
    let history_path = dirs::data_local_dir().map(|p| p.join("feldspar").join("history.txt"));
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    loop {
        match rl.readline("\x1b[36mλ >\x1b[0m ") {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(&trimmed);

                // Handle : commands
                if trimmed.starts_with(':') {
                    if let Some(should_quit) = handle_command(&trimmed, &mut engine) {
                        if should_quit {
                            break;
                        }
                    }
                    continue;
                }

                match engine.run(trimmed) {
                    Ok(values) => {
                        for val in values {
                            println!("\x1b[35m=>\x1b[0m {}", val);
                        }
                    }
                    Err(e) => eprintln!("\x1b[31mError:\x1b[0m {}", e),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = rl.save_history(path);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut engine = init();

    // Load file if provided as argument
    if args.len() > 1 {
        let path = &args[1];
        match fs::read_to_string(path) {
            Ok(contents) => match engine.run(contents) {
                Ok(_) => println!("Loaded {}", path),
                Err(e) => eprintln!("Error in {}: {}", path, e),
            },
            Err(e) => eprintln!("Error reading {}: {}", path, e),
        }
    }

    repl(engine);
}

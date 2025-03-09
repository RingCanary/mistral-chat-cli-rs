use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use std::error::Error;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long)]
    debug: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Chat { prompt: String },
    Test,
    Code { code: String },
}

#[derive(Serialize)]
struct RequestMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<RequestMessage>,
    stream: bool,
    max_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

struct ChatClient {
    client: Client,
    mistral_api_key: String,
    codestral_api_key: String,
    debug: bool,
}

impl ChatClient {
    fn new(mistral_api_key: String, codestral_api_key: String, debug: bool) -> Self {
        ChatClient {
            client: Client::new(),
            mistral_api_key,
            codestral_api_key,
            debug,
        }
    }

    /// Streams chat completions from the API and prints them to stdout
    async fn chat_stream(&self, model: &str, messages: Vec<RequestMessage>) -> Result<(), Box<dyn Error>> {
        if self.debug {
            println!("DEBUG: Sending streaming request to {} API", model);
            println!("DEBUG: Using URL: {}", if model.contains("codestral") {
                "https://codestral.mistral.ai/v1/chat/completions"
            } else {
                "https://api.mistral.ai/v1/chat/completions"
            });
        }

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            max_tokens: None,
        };

        if self.debug {
            println!("DEBUG: Request body: {}", serde_json::to_string(&request)?);
        }

        let url = if model.contains("codestral") {
            "https://codestral.mistral.ai/v1/chat/completions"
        } else {
            "https://api.mistral.ai/v1/chat/completions"
        };

        let api_key = if model.contains("codestral") {
            &self.codestral_api_key
        } else {
            &self.mistral_api_key
        };

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await?;

        if self.debug {
            println!("DEBUG: Response status: {}", response.status());
        }

        let mut stream = response.bytes_stream();
        let mut stdout = tokio::io::stdout();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if self.debug {
                        println!("DEBUG: Received chunk: {}", text);
                    }
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                if self.debug {
                                    println!("DEBUG: Received [DONE]");
                                }
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                                break;
                            }
                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(json) => {
                                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                        stdout.write_all(content.as_bytes()).await?;
                                        stdout.flush().await?;
                                    } else if self.debug {
                                        println!("DEBUG: No content in JSON: {}", json);
                                    }
                                }
                                Err(e) => {
                                    if self.debug {
                                        println!("DEBUG: JSON parse error: {} - Data: {}", e, data);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if self.debug {
                        println!("DEBUG: Chunk error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Tests API connectivity with a minimal request
    async fn test_connection(&self) -> Result<(), Box<dyn Error>> {
        if self.debug {
            println!("DEBUG: Testing API connection...");
        }

        let messages = vec![RequestMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        let request = ChatRequest {
            model: "mistral-large-latest".to_string(),
            messages,
            stream: false,
            max_tokens: Some(1),
        };

        if self.debug {
            println!("DEBUG: Request body: {}", serde_json::to_string(&request)?);
        }

        let response = self.client
            .post("https://api.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.mistral_api_key))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if self.debug {
            println!("DEBUG: Status: {}", status);
        }

        if status.is_success() {
            println!("MISTRAL-API connection successful");
        } else {
            println!("MISTRAL-API connection failed: {}", status);
            if self.debug {
                let text = response.text().await?;
                println!("DEBUG: Response body: {}", text);
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                println!("Hint: Check your API key.");
            }
        }

        let code_messages = vec![RequestMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        let codestral_request = ChatRequest {
            model: "codestral-latest".to_string(),
            messages: code_messages,
            stream: false,
            max_tokens: None,
        };

        if self.debug {
            println!("DEBUG: Request body: {}", serde_json::to_string(&codestral_request)?);
        }

        let codestral_response = self.client
            .post("https://codestral.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.codestral_api_key))
            .json(&codestral_request)
            .send()
            .await?;

        let status = codestral_response.status();

        if self.debug {
            println!("DEBUG: Status: {}", status);
        }

        if status.is_success() {
            println!("CODESTRAL-API connection successful");
        } else {
            println!("CODESTRAL-API connection failed: {}", status);
            if self.debug {
                let text = codestral_response.text().await?;
                println!("DEBUG: Response body: {}", text);
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                println!("Hint: Check your API key.");
            }
        }

        Ok(())
    }

    /// Analyzes code using the Codestral API
    async fn analyze_code(&self, code: String) -> Result<String, Box<dyn Error>> {
        if self.debug {
            println!("DEBUG: Sending code to Codestral API");
        }

        let messages = vec![RequestMessage {
            role: "user".to_string(),
            content: code,
        }];

        let request = ChatRequest {
            model: "codestral-latest".to_string(),
            messages,
            stream: false,
            max_tokens: None,
        };

        if self.debug {
            println!("DEBUG: Request body: {}", serde_json::to_string(&request)?);
        }

        let response = self.client
            .post("https://codestral.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.codestral_api_key))
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mistral_api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
    let codestral_api_key = std::env::var("CODESTRAL_API_KEY").expect("CODESTRAL_API_KEY not set");
    let chat_client = ChatClient::new(mistral_api_key, codestral_api_key, cli.debug);

    match cli.command {
        Commands::Chat { prompt } => {
            let messages = vec![RequestMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            }];
            let model = if prompt.to_lowercase().contains("code") {
                "codestral-latest"
            } else {
                "mistral-large-latest"
            };
            chat_client.chat_stream(model, messages).await?;
        }
        Commands::Test => {
            chat_client.test_connection().await?;
        }
        Commands::Code { code } => {
            let analysis = chat_client.analyze_code(code).await?;
            println!("{}", analysis);
        }
    }

    Ok(())
}
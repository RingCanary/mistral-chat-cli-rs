use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Command-line argument parser for the CLI.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Enable debug mode for detailed logs.
    #[arg(long)]
    debug: bool,

    /// Subcommand to execute (e.g., chat, test, code).
    #[command(subcommand)]
    command: Commands,
}

/// Enum representing the available subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Send a chat prompt to the API.
    Chat { prompt: String },

    /// Test the API connection.
    Test,

    /// Analyze a code snippet using the API.
    Code { code: String },
}

/// Struct representing a request message sent to the API.
#[derive(Serialize)]
struct RequestMessage {
    /// The role of the message sender (e.g., "user").
    role: String,

    /// The content of the message.
    content: String,
}

/// Struct representing a response message received from the API.
#[derive(Deserialize)]
struct ResponseMessage {
    /// The content of the response message.
    content: String,
}

/// Struct representing a chat request sent to the API.
#[derive(Serialize)]
struct ChatRequest {
    /// The model to use for the chat completion.
    model: String,

    /// A vector of messages in the chat.
    messages: Vec<RequestMessage>,

    /// Whether to stream the response.
    stream: bool,

    /// The maximum number of tokens to generate.
    max_tokens: Option<u32>,
}

/// Struct representing a chat response received from the API.
#[derive(Deserialize)]
struct ChatResponse {
    /// A vector of choices in the chat response.
    choices: Vec<Choice>,
}

/// Struct representing a choice in the chat response.
#[derive(Deserialize)]
struct Choice {
    /// The message associated with the choice.
    message: ResponseMessage,
}

/// A client for interacting with the Mistral and Codestral APIs.
///
/// This struct manages the API keys and provides methods to send requests
/// to the Mistral and Codestral APIs for chat, testing connections, and analyzing code.
struct ChatClient {
    client: Client,
    mistral_api_key: String,
    codestral_api_key: String,
    debug: bool,
}

impl ChatClient {
    /// Creates a new `ChatClient` with the given API keys and debug mode.
    ///
    /// # Arguments
    ///
    /// * `mistral_api_key` - The API key for the Mistral API.
    /// * `codestral_api_key` - The API key for the Codestral API.
    /// * `debug` - A boolean indicating whether debug mode is enabled.
    ///
    /// # Returns
    ///
    /// A new `ChatClient` instance.
    fn new(mistral_api_key: String, codestral_api_key: String, debug: bool) -> Self {
        ChatClient {
            client: Client::new(),
            mistral_api_key,
            codestral_api_key,
            debug,
        }
    }

    /// Streams chat completions from the API and prints them to stdout.
    ///
    /// This method sends a request to the specified model's API and streams the response
    /// to stdout. It retries the request up to three times in case of transient errors.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use for the chat completion (e.g., "mistral-large-latest" or "codestral-latest").
    /// * `messages` - A vector of `RequestMessage` structs representing the chat messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails after multiple attempts or if there is an issue
    /// with the response stream.
    async fn chat_stream(&self, model: &str, messages: Vec<RequestMessage>) -> Result<()> {
        if self.debug {
            println!("DEBUG: Sending streaming request to {} API", model);
            println!(
                "DEBUG: Using URL: {}",
                if model.contains("codestral") {
                    "https://codestral.mistral.ai/v1/chat/completions"
                } else {
                    "https://api.mistral.ai/v1/chat/completions"
                }
            );
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

        let mut attempts = 0;
        let max_attempts = 3;

        let response = loop {
            match self
                .client
                .post(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request)
                .send()
                .await
            {
                Ok(resp) => break resp,
                Err(err) if attempts < max_attempts => {
                    attempts += 1;
                    println!("Retry attempt {}: {}", attempts, err);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(err) => {
                    return Err(err).context("Failed to send request after multiple attempts")
                }
            }
        };

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
                                    if let Some(content) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
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

    /// Tests API connectivity with a minimal request.
    ///
    /// This method sends a minimal request to both the Mistral and Codestral APIs to test
    /// the connectivity and prints the result to stdout.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the API key is invalid.
    async fn test_connection(&self) -> Result<()> {
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

        let response = self
            .client
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
            println!(
                "DEBUG: Request body: {}",
                serde_json::to_string(&codestral_request)?
            );
        }

        let codestral_response = self
            .client
            .post("https://codestral.mistral.ai/v1/chat/completions")
            .header(
                "Authorization",
                format!("Bearer {}", self.codestral_api_key),
            )
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

    /// Analyzes code using the Codestral API.
    ///
    /// This method sends the given code to the Codestral API for analysis and returns
    /// the response as a string.
    ///
    /// # Arguments
    ///
    /// * `code` - The code to analyze as a string.
    ///
    /// # Returns
    ///
    /// The analysis result as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if there is an issue with the response.
    async fn analyze_code(&self, code: String) -> Result<String> {
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

        let response = self
            .client
            .post("https://codestral.mistral.ai/v1/chat/completions")
            .header(
                "Authorization",
                format!("Bearer {}", self.codestral_api_key),
            )
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mistral_api_key = std::env::var("MISTRAL_API_KEY").context("MISTRAL_API_KEY not set")?;
    let codestral_api_key =
        std::env::var("CODESTRAL_API_KEY").context("CODESTRAL_API_KEY not set")?;
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

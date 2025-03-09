use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::{Config as ConfigFile, Environment, File};
use futures_util::StreamExt;
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

// Constants for API endpoints and model names.
const MISTRAL_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const CODESTRAL_URL: &str = "https://codestral.mistral.ai/v1/chat/completions";
const MISTRAL_MODEL: &str = "mistral-large-latest";
const CODESTRAL_MODEL: &str = "codestral-latest";

/// Command-line argument parser for the CLI.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Enable debug mode for detailed logs.
    #[arg(long)]
    debug: bool,

    /// Configuration file to use.
    #[arg(long, default_value = "config.toml")]
    config: String,

    /// Subcommand to execute (e.g., chat, test, code, config).
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

    /// Manage configuration files.
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
}

/// Struct representing a request message sent to the API.
#[derive(Serialize, Clone)]
struct RequestMessage {
    role: String,
    content: String,
}

/// Struct representing a response message received from the API.
#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Struct representing a chat request sent to the API.
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<RequestMessage>,
    stream: bool,
    max_tokens: Option<u32>,
}

/// Struct representing a chat response received from the API.
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

/// Struct representing a choice in the chat response.
#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// Enum representing the configuration subcommands.
#[derive(Subcommand)]
enum ConfigCommands {
    /// Generate a sample configuration file.
    Generate {
        /// Optional path to generate the config file.
        #[arg(short, long)]
        path: Option<String>,
    },

    /// View the current configuration.
    View,

    /// Load a configuration file from a specified path.
    Load {
        /// Path to the configuration file.
        #[arg(short, long)]
        file_path: String,
    },
}

/// Struct representing configuration for the CLI.
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    mistral_api_key: String,
    codestral_api_key: String,
    debug: bool,
}

impl Config {
    fn from_file(file_path: &str) -> Result<Self, config::ConfigError> {
        let settings = ConfigFile::builder()
            // Add configuration from a file.
            .add_source(File::with_name(file_path))
            // Add configuration from environment variables.
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        // Try to deserialize the configuration into the `Config` struct.
        settings.try_deserialize()
    }

    fn generate_sample_config(file_path: &str) -> Result<()> {
        let sample_config = Config {
            mistral_api_key: "your_mistral_api_key".to_string(),
            codestral_api_key: "your_codestral_api_key".to_string(),
            debug: false,
        };

        let config_content = toml::to_string(&sample_config)?;
        fs::write(file_path, config_content)?;
        Ok(())
    }

    // Mask API keys by showing only the first few characters.
    fn mask_key(key: &str) -> String {
        if key.len() > 5 {
            format!("{}{}", &key[..5], "*".repeat(key.len() - 5))
        } else {
            key.to_string()
        }
    }

    fn view_config(config: &Config) {
        println!("Current Configuration:");
        println!(
            "Mistral API Key: {}",
            Config::mask_key(&config.mistral_api_key)
        );
        println!(
            "Codestral API Key: {}",
            Config::mask_key(&config.codestral_api_key)
        );
        println!("Debug Mode: {}", config.debug);
    }
}

/// A client for interacting with the Mistral and Codestral APIs.
struct ChatClient {
    client: Client,
    mistral_api_key: String,
    codestral_api_key: String,
    debug: bool,
}

impl ChatClient {
    /// Creates a new `ChatClient` with the given API keys and debug mode.
    fn new(mistral_api_key: String, codestral_api_key: String, debug: bool) -> Self {
        ChatClient {
            client: Client::new(),
            mistral_api_key,
            codestral_api_key,
            debug,
        }
    }

    /// Helper for sending a request with retry logic.
    async fn send_with_retry<F, Fut>(&self, request_func: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = reqwest::Result<reqwest::Response>>,
    {
        let max_attempts = 3;
        for attempt in 1..=max_attempts {
            match request_func().await {
                Ok(resp) => return Ok(resp),
                Err(err) if attempt < max_attempts => {
                    error!("Retry attempt {}: {}", attempt, err);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(err) => {
                    return Err(err)
                        .context("Failed to send request after multiple attempts")
                }
            }
        }
        unreachable!();
    }

    /// Streams chat completions from the API and prints them to stdout.
    async fn chat_stream(&self, model: &str, messages: Vec<RequestMessage>) -> Result<()> {
        if self.debug {
            debug!("Sending streaming request to {} API", model);
            debug!(
                "Using URL: {}",
                if model.contains("codestral") {
                    CODESTRAL_URL
                } else {
                    MISTRAL_URL
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
            debug!("Request body: {}", serde_json::to_string(&request)?);
        }

        let url = if model.contains("codestral") {
            CODESTRAL_URL
        } else {
            MISTRAL_URL
        };

        let api_key = if model.contains("codestral") {
            &self.codestral_api_key
        } else {
            &self.mistral_api_key
        };

        let response = self
            .send_with_retry(|| {
                self.client
                    .post(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&request)
                    .send()
            })
            .await?;

        if self.debug {
            debug!("Response status: {}", response.status());
        }

        let mut stream = response.bytes_stream();
        let mut stdout = tokio::io::stdout();

        'outer: while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if self.debug {
                        debug!("Received chunk: {}", text);
                    }
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                if self.debug {
                                    debug!("Received [DONE]");
                                }
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                                break 'outer;
                            }
                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(json) => {
                                    if let Some(content) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
                                        stdout.write_all(content.as_bytes()).await?;
                                        stdout.flush().await?;
                                    } else if self.debug {
                                        debug!("No content in JSON: {}", json);
                                    }
                                }
                                Err(e) => {
                                    if self.debug {
                                        debug!("JSON parse error: {} - Data: {}", e, data);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Streaming failed: {}", e);
                    if self.debug {
                        debug!("Chunk error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Tests API connectivity with a minimal request.
    async fn test_connection(&self) -> Result<()> {
        if self.debug {
            debug!("Testing API connection...");
        }

        let messages = vec![RequestMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        // Test Mistral API.
        let request = ChatRequest {
            model: MISTRAL_MODEL.to_string(),
            messages: messages.clone(),
            stream: false,
            max_tokens: Some(1),
        };

        if self.debug {
            debug!("Mistral request body: {}", serde_json::to_string(&request)?);
        }

        let mistral_response = self
            .send_with_retry(|| {
                self.client
                    .post(MISTRAL_URL)
                    .header("Authorization", format!("Bearer {}", self.mistral_api_key))
                    .json(&request)
                    .send()
            })
            .await?;

        let status = mistral_response.status();
        if self.debug {
            debug!("MISTRAL status: {}", status);
        }
        if status.is_success() {
            info!("MISTRAL-API connection successful");
        } else {
            error!("MISTRAL-API connection failed: {}", status);
            if self.debug {
                let text = mistral_response.text().await?;
                debug!("MISTRAL response body: {}", text);
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                error!("Hint: Check your Mistral API key.");
            }
        }

        // Test Codestral API.
        let codestral_request = ChatRequest {
            model: CODESTRAL_MODEL.to_string(),
            messages,
            stream: false,
            max_tokens: None,
        };

        if self.debug {
            debug!(
                "Codestral request body: {}",
                serde_json::to_string(&codestral_request)?
            );
        }

        let codestral_response = self
            .send_with_retry(|| {
                self.client
                    .post(CODESTRAL_URL)
                    .header(
                        "Authorization",
                        format!("Bearer {}", self.codestral_api_key),
                    )
                    .json(&codestral_request)
                    .send()
            })
            .await?;

        let status = codestral_response.status();
        if self.debug {
            debug!("CODESTRAL status: {}", status);
        }
        if status.is_success() {
            info!("CODESTRAL-API connection successful");
        } else {
            error!("CODESTRAL-API connection failed: {}", status);
            if self.debug {
                let text = codestral_response.text().await?;
                debug!("CODESTRAL response body: {}", text);
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                error!("Hint: Check your Codestral API key.");
            }
        }

        Ok(())
    }

    /// Analyzes code using the Codestral API.
    async fn analyze_code(&self, code: String) -> Result<String> {
        if self.debug {
            debug!("Sending code to Codestral API");
        }

        let messages = vec![RequestMessage {
            role: "user".to_string(),
            content: code,
        }];

        let request = ChatRequest {
            model: CODESTRAL_MODEL.to_string(),
            messages,
            stream: false,
            max_tokens: None,
        };

        if self.debug {
            debug!("Analyze code request: {}", serde_json::to_string(&request)?);
        }

        let response = self
            .send_with_retry(|| {
                self.client
                    .post(CODESTRAL_URL)
                    .header(
                        "Authorization",
                        format!("Bearer {}", self.codestral_api_key),
                    )
                    .json(&request)
                    .send()
            })
            .await?
            .json::<ChatResponse>()
            .await?;

        if let Some(choice) = response.choices.get(0) {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!(
                "Empty response received from Codestral API"
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments.
    let cli = Cli::parse();

    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(if cli.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });
    builder.init();

    match &cli.command {
        Commands::Chat { prompt } => {
            let config = Config::from_file(&cli.config)
                .expect("Failed to read configuration file");
            let debug = cli.debug || config.debug;
            let chat_client = ChatClient::new(
                config.mistral_api_key,
                config.codestral_api_key,
                debug,
            );
            let messages = vec![RequestMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            }];
            let model = if prompt.to_lowercase().contains("code") {
                CODESTRAL_MODEL
            } else {
                MISTRAL_MODEL
            };
            chat_client.chat_stream(model, messages).await?;
        }
        Commands::Test => {
            let config = Config::from_file(&cli.config)
                .expect("Failed to read configuration file");
            let debug = cli.debug || config.debug;
            let chat_client = ChatClient::new(
                config.mistral_api_key,
                config.codestral_api_key,
                debug,
            );
            chat_client.test_connection().await?;
        }
        Commands::Code { code } => {
            let config = Config::from_file(&cli.config)
                .expect("Failed to read configuration file");
            let debug = cli.debug || config.debug;
            let chat_client = ChatClient::new(
                config.mistral_api_key,
                config.codestral_api_key,
                debug,
            );
            let analysis = chat_client.analyze_code(code.clone()).await?;
            info!("{}", analysis);
        }
        Commands::Config { config_command } => match config_command {
            ConfigCommands::Generate { path } => {
                let file_path = path.as_deref().unwrap_or("config.toml");
                Config::generate_sample_config(file_path)
                    .expect("Failed to generate config file");
                println!("Sample config file generated at {}", file_path);
            }
            ConfigCommands::View => {
                let config = Config::from_file(&cli.config)
                    .expect("Failed to read configuration file");
                Config::view_config(&config);
            }
            ConfigCommands::Load { file_path } => {
                let config = Config::from_file(file_path)
                    .expect("Failed to read configuration file");
                println!("Configuration loaded from {}", file_path);
                Config::view_config(&config);
                // Optionally, update the default configuration file if needed.
                // fs::copy(file_path, &cli.config).expect("Failed to set new default config file");
            }
        },
    }

    Ok(())
}
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio::io::{self, AsyncWriteExt};
use futures_util::StreamExt;


#[derive(Parser)]
#[command(name = "mistral-chat")]
#[command(about = "A CLI chat app using Mistral AI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Chat with Mistral AI
    Chat {
        #[arg(short, long)]
        prompt: String,
    },
    /// Test system connectivity and API availability
    Test,
    /// Analyze code with Codestral
    Code {
        #[arg(short, long)]
        code: String,
    },
}

#[derive(Serialize)]
struct ChatRequest {
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: String,
    // Add more fields as needed based on API documentation
}

struct ChatClient {
    client: Client,
    api_key: String,
    debug: bool,
}

impl ChatClient {
    /// Create a new ChatClient instance
    fn new(api_key: String, debug: bool) -> Self {
        ChatClient {
            client: Client::new(),
            api_key,
            debug,
        }
    }

    /// Stream chat responses from Mistral API
    async fn chat_stream(&self, prompt: String) -> Result<(), Box<dyn std::error::Error>> {
        if self.debug {
            println!("DEBUG: Sending prompt to Mistral API: {}", prompt);
        }

        let request = ChatRequest {
            prompt,
            stream: true,
        };

        let mut stream = self.client
            .post("https://api.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?
            .bytes_stream();

        let mut stdout = io::stdout();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if self.debug {
                println!("DEBUG: Received chunk: {:?}", chunk);
            }
            // Note: You may need to parse the chunk based on API format (e.g., JSON lines)
            stdout.write_all(&chunk).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Test connectivity to the Mistral API
    async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.debug {
            println!("DEBUG: Testing API connection...");
        }

        let response = self.client
            .get("https://api.mixtral.ai/v1/health") // Replace with actual health endpoint
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if self.debug {
            println!("DEBUG: Status: {}", response.status());
            println!("DEBUG: Headers: {:?}", response.headers());
        }

        if response.status().is_success() {
            println!("API connection successful");
        } else {
            println!("API connection failed: {}", response.status());
            if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                println!("Hint: Check your API key.");
            }
        }

        Ok(())
    }

    /// Analyze code using Codestral API
    async fn analyze_code(&self, code: String) -> Result<String, Box<dyn std::error::Error>> {
        if self.debug {
            println!("DEBUG: Sending code to Codestral API: {}", code);
        }

        let request = ChatRequest {
            prompt: code,
            stream: false,
        };

        let response = self.client
            .post("https://codestral.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        if self.debug {
            println!("DEBUG: Codestral response: {}", response.message);
        }

        Ok(response.message)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let api_key = std::env::var("MISTRAL_API_KEY").unwrap_or_else(|_| {
        panic!("MISTRAL_API_KEY environment variable not set")
    });

    let chat_client = ChatClient::new(api_key, cli.debug);

    match cli.command {
        Commands::Chat { prompt } => {
            // Simple heuristic to detect code-related queries
            let is_code_related = prompt.to_lowercase().contains("code") || 
                                 prompt.to_lowercase().contains("programming");
            if is_code_related {
                let analysis = chat_client.analyze_code(prompt.clone()).await?;
                println!("Code analysis: {}", analysis);
            }
            chat_client.chat_stream(prompt).await?;
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
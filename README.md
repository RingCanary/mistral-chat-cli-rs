# Mistral CLI

Welcome to **Mistral CLI**, a powerful command-line interface (CLI) tool built in Rust. This application allows you to interact with the Mistral and Codestral APIs to chat with AI, test API connections, and analyze code snippets—all from your terminal.

---

## ✨ Features

- **AI-Powered Chat**: Send prompts to Mistral or Codestral APIs and receive streamed responses in real-time.
- **API Health Check**: Quickly test your connection to the Mistral and Codestral APIs.
- **Code Analysis**: Get insights and suggestions for your code snippets using the Codestral API.
- **Configuration Management**: Generate, view, and load configuration files to manage your API keys and settings.

---

## 🚀 Installation

### Prerequisites

Before you begin, ensure you have the following installed:
- [Rust](https://www.rust-lang.org/tools/install)
- [Git](https://git-scm.com/downloads)

### Setup Steps

1. **Clone the Repository**
   Grab the code from GitHub and navigate into the project folder:
   ```bash
   git clone https://github.com/RingCanary/mistral-chat-cli-rs.git
   cd mistral-chat-cli-rs
   ```

2. **Build the Application**
   Compile the app with Cargo:
   ```bash
   cargo build --release
   ```

3. **Run It**
   Start using the CLI right away:
   ```bash
   cargo run --release -- [OPTIONS] <SUBCOMMAND>
   ```

---

## 🛠️ Usage

The CLI follows this structure:
```
mistral-chat-cli-rs [OPTIONS] <SUBCOMMAND>
```

### Available Subcommands

- **`chat <PROMPT>`**
  Send a prompt to the AI. If "code" is in your prompt, it uses Codestral; otherwise, it defaults to Mistral.
  _Example_: Streams the response directly to your console.

- **`test`**
  Checks if the Mistral and Codestral APIs are reachable and reports the result.
  _Example_: Perfect for verifying your setup.

- **`code <CODE_SNIPPET>`**
  Analyzes a code snippet using the Codestral API and displays the feedback.
  _Example_: Great for debugging or improving code.

- **`config <CONFIG_COMMAND>`**
  Manage configuration files. Available config commands:
  - `generate [--path <FILE_PATH>]`: Generate a sample configuration file.
  - `view`: View the current configuration.
  - `load --file-path <FILE_PATH>`: Load a configuration file from a specified path.

### Options

- **`--debug`**
  Enable debug mode to see detailed logs of API requests and responses.
  _Example_: Useful for troubleshooting.

---

## 🔑 Configuration

To interact with the APIs, you’ll need to set up your API keys in a configuration file or as environment variables:

- **`MISTRAL_API_KEY`**: Your key for the Mistral API.
- **`CODESTRAL_API_KEY`**: Your key for the Codestral API.

### How to Set Environment Variables

- **Linux/Mac**
  Add these lines to your shell (e.g., `~/.bashrc` or `~/.zshrc`):
  ```bash
  export MISTRAL_API_KEY=your_mistral_key
  export CODESTRAL_API_KEY=your_codestral_key
  ```

- **Windows**
  Use the Command Prompt to set them:
  ```cmd
  set MISTRAL_API_KEY=your_mistral_key
  set CODESTRAL_API_KEY=your_codestral_key
  ```

> **Tip**: Get your API keys from the Mistral and Codestral service providers and keep them secure!

---

## 🌟 Examples

Here’s how you can use the CLI in action:

### 1. Chat About Anything
```bash
cargo run --release -- chat "What’s the meaning of life?"
```
_Streams a thoughtful response from Mistral._

### 2. Code-Related Question
```bash
cargo run --release -- chat "How do I sort an array in Rust?"
```
_Triggers Codestral for a code-focused answer._

### 3. Test Your API Connection
```bash
cargo run --release -- test
```
_Confirms if Mistral and Codestral are online and ready._

### 4. Analyze a Code Snippet
```bash
cargo run --release -- code "fn add(a: i32, b: i32) -> i32 { a + b }"
```
_Gets feedback from Codestral on your code._

### 5. Generate a Configuration File
```bash
cargo run --release -- config generate
```
_Creates a sample configuration file._

### 6. View Current Configuration
```bash
cargo run --release -- config view
```
_Displays the current configuration settings._

### 7. Load a Configuration File
```bash
cargo run --release -- config load --file-path path/to/config.toml
```
_Loads a configuration file from the specified path._

---

## 🐞 Debugging

Need to troubleshoot? Use the `--debug` flag to peek under the hood:
```bash
cargo run --release -- --debug chat "Test this out"
```
This prints detailed info about API calls, helping you spot issues fast.

---

## 🛠️ Built With

This CLI is powered by some amazing tools:
- **Rust**: Fast, safe, and efficient programming language.
- **clap**: Command-line argument parsing made simple.
- **reqwest**: Robust HTTP client for API requests.
- **serde**: JSON serialization/deserialization magic.
- **tokio**: Asynchronous runtime for smooth streaming.
- **config**: Configuration file management.

---

## 🤝 Contributing

Love the project? Want to make it better? Contributions are welcome! Here’s how:
1. Fork the repository.
2. Create a branch for your changes (`git checkout -b feature/awesome-idea`).
3. Commit your updates (`git commit -m "Add awesome idea"`).
4. Push to your branch (`git push origin feature/awesome-idea`).
5. Open a pull request!

Found a bug or have a feature request? Open an issue on [GitHub](https://github.com/RingCanary/mistral-chat-cli-rs/issues).

---

## 📜 License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.

---

Happy coding, chatting, and exploring with **Mistral-Codestral CLI**!
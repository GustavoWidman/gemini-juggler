# Gemini Juggler

Gemini Juggler is a Rust web service built on Actix-Web that forwards content generation requests to the Gemini API. It handles API key rotation and rate-limiting with a simple key juggling mechanism.

## Features

- **API Key Juggling**: Rotates API keys to bypass rate limits.
- **Asynchronous HTTP Handling**: Uses Actix-Web and awc client for async request processing.
- **Custom Logging**: Integrated logging with colored, timestamped output.
- **Configurable**: Easily update API keys, host, and port via a TOML config file.

## Installation

1. Ensure you have Rust installed (preferably using [rustup](https://rustup.rs/)).
2. Clone the repository:

   ```bash
   git clone https://github.com/GustavoWidman/gemini-juggler.git
   ```

3. Change into the project directory:

   ```bash
   cd gemini-juggler
   ```

## Usage

1. Update the `config.toml` file with your preferred server settings and API keys.
2. Build and run:

   ```bash
   cargo run
   ```

3. The server will start at the host and port defined in `config.toml`.
   Example endpoint: `POST http://0.0.0.0:8080/v1beta/models/{model}:generateContent?key={api_key}`

## Configuration

The project uses a `config.toml` file located in the project root. Update it with:

- `api_key`: The primary API key (required).
- `keys`: A list of API keys for rotation.
- `host`, `port`: Server binding settings.

## Dependencies

- [Actix-Web](https://github.com/actix/actix-web)
- [Chrono](https://github.com/chronotope/chrono)
- [Env Logger](https://docs.rs/env_logger)
- [Colog](https://github.com/wojtekmach/rs-colog)
- [Serde](https://serde.rs/)

## License

Distributed under the MIT License. See `LICENSE` for more information.

# grafton-config

`grafton-config` is a Rust-based configuration library for Rust applications, featuring token expansion and layered configuration loading for TOML format configuration files.

## Features

- Layered configuration loading from multiple TOML files
- Dynamic token expansion within configuration files
- Support for environment-specific configurations
- Flexible and extensible design

## Why Configuration Matters

Robust configuration management is crucial for:

1. **Environment Flexibility:** Your app needs to adapt to various environments (dev, staging, production).
2. **Security:** Sensitive information like API keys must be handled with care.
3. **Scalability:** As your app grows, so does the complexity of its configuration.

`grafton-config` addresses these challenges, providing a comprehensive solution for Rust developers.  For more information on configuration management in Rust, check out my detailed blog post [here](https://blog.grafton.ai/configuration-management-for-rust-applications-15b2a0346b80).

## Installation

Add `grafton-config` to your `Cargo.toml`:

```toml
[dependencies]
grafton-config = "*"
```

## Usage

### Defining Your Configuration Structure

Let's create a basic configuration structure:

```rust
use serde::{Deserialize, Serialize};
use grafton_config::TokenExpandingConfig;

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    host: String,
    port: u16,
    database_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    server: Server,
}

impl TokenExpandingConfig for AppConfig {}
```

This structure provides a clear, type-safe representation of your app's configuration.

### Loading Your Configuration

```rust
use grafton_config::{load_config_from_dir, TokenExpandingConfig, Error};

fn main() -> Result<(), Error> {
    let config = grafton_config::load_config_from_dir::<AppConfig>("examples/config")?;
    println!("Database URL: {}", config.server.database_url);
    Ok(())
}
```

### Example

To run the example from the repository, use the following command:

```sh
cargo run --example config_example
```

### Layered Configuration: Flexibility at Its Core

`grafton-config` supports layered configurations, allowing different settings for various environments.

**Configuration Files**:

The configuration is loaded from the following files in a specified directory:

1. `default.toml`: Your base configuration (required)
2. `local.toml`: Local overrides (optional)
3. `{run_mode}.toml`: Environment-specific config (optional)

The `run_mode` is determined by the `RUN_MODE` environment variable, defaulting to `dev` if not set. Files are loaded in the order listed above, with later files overriding any values from earlier ones.

**Example Setup**:

`default.toml`:

```toml
[server]
host = "localhost"
port = 5432
database_url = "postgresql://user:password@${server.host}:${server.port}/mydb"
```

`local.toml` (optional, for local development):

```toml
[server]
host = "127.0.0.1"
```

`prod.toml` (optional, for production.  See run_mode above):

```toml
[server]
host = "db.production.com"
database_url = "postgresql://user:password@${server.host}:${server.port}/mydb"
```

## Token Expansion: From Basics to Advanced Usage

Token expansion is a key feature of `grafton-config`. It allows you to reference other values within your configuration, making it more dynamic and reducing redundancy.

### Basic Token Usage

Tokens use the `${key_path}` syntax, where `key_path` is a dot-separated path to the desired value.

```toml
[database]
host = "db.example.com"
port = 5432
url = "postgresql://user:password@${database.host}:${database.port}/mydb"
```

### Advanced Token Usage

1. **Nested Paths**:

   ```toml
   [person]
   first_name = "John"
   last_name = "Doe"
   full_name = "${person.first_name} ${person.last_name}"
   ```

2. **Array Access**:

   ```toml
   fruits = ["apple", "banana", "cherry"]
   favorite = "My favorite fruit is ${fruits.1}"
   ```

3. **Escaping Tokens**:

   ```toml
   literal = "This is a \${literal} dollar sign"
   ```

### Handling Edge Cases

`grafton-config` handles various scenarios gracefully:

- **Circular References**: There's a recursion limit (currently 99) to prevent infinite loops.
- **Partial Expansions**: If a token can't be fully expanded, the unexpandable parts remain as-is.
- **Type Handling**: Tokens can expand to various TOML data types, including strings, integers, floats, booleans, and datetimes.

## API Reference

- `load_config_from_dir(path: &str) -> Result<T, Error>`: Load and parse configuration from a directory
- `GraftonConfig`: Trait for grafton-configuration structs
- `TokenExpandingConfig`: Trait for configuration structs that support token expansion

## Resources

- [Blog tutorial: Configuration Management for Rust Applications](https://blog.grafton.ai/configuration-management-for-rust-applications-15b2a0346b80)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

`grafton-config` is dual-licensed under the Apache License 2.0 (for open-source use) and a commercial license.

### Open Source License

Unless explicitly stated otherwise, all files in this repository are licensed under the Apache License 2.0. The full text of the license can be found in the LICENSE file.

#### Key Features of the Apache License 2.0

- **Permissive License**: Enables free commercial and non-commercial use, distribution, and modification of the software without requiring source code access.
- **Explicit Patent License**: Grants an express patent license from all contributors to users.
- **Patent Retaliation Provision**: Protects users and contributors from patent infringement claims.

### Commercial License

For those wishing to integrate `grafton-config` into closed-source applications or who need more flexibility than the Apache License 2.0 allows, a commercial license is available. For commercial licensing inquiries, please contact <grant@grafton.ai>

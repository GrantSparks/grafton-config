
# Grafton Config

Grafton Config is a Rust-based configuration library for Rust applications, featuring token expansion and layered configuration loading for TOML format configuration files.

## Features

- Layered configuration loading from multiple TOML files
- Dynamic token expansion within configuration files
- Support for environment-specific configurations
- Flexible and extensible design

## Installation

Add Grafton Config to your `Cargo.toml`:

```toml
[dependencies]
grafton-config = "*"
```

## Usage

### Basic Usage

Create your configuration struct:

```rust
use serde::{Deserialize, Serialize};
use grafton_config::TokenExpandingConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct MyConfig {
    // Your configuration fields here
}

impl TokenExpandingConfig for MyConfig {}
```

Load the configuration:

```rust
use grafton_config::load_config_from_dir;

fn main() -> Result<(), grafton_config::Error> {
    let config: MyConfig = load_config_from_dir("path/to/config/directory")?;
    // Use your configuration
    Ok(())
}
```

### Configuration Loading

The configuration is loaded from the following files in a specified directory:

- `default.toml`
- `local.toml`
- `{run_mode}.toml`

The `run_mode` is determined by the `RUN_MODE` environment variable, defaulting to `dev` if not set. Files are loaded in the order listed above, with later files overriding values from earlier ones.

### Token Expansion

Grafton Config supports token expansion within your TOML files. Use the `${key_path}` syntax to reference other values in your configuration.

## Example

To run the example from the repository, use the following command:
```sh
cargo run --example config_example
```

### Application Code

Here's an example of using Grafton Config in a Rust application:

```rust
use serde::{Deserialize, Serialize};
use grafton_config::{TokenExpandingConfig, Error};

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    host: String,
    port: u16,
    database_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    pub server: Server,
}

impl TokenExpandingConfig for AppConfig {}

fn main() -> Result<(), Error> {
    let config: AppConfig = grafton_config::load_config_from_dir("examples/config")?;
    println!("Database URL: {}", config.server.database_url);
    Ok(())
}
```

### Configuration Files

Example `default.toml`:

```toml
[server]
host = "localhost"
port = 5432
database_url = "postgresql://user:password@${server.host}:${server.port}/mydb"
```

Example `local.toml`:

```toml
[server]
host = "127.0.0.1"
```

Example `prod.toml`:

```toml
[server]
host = "db.production.com"
database_url = "postgresql://user:password@${server.host}:${server.port}/mydb"
```

## API Reference

- `load_config_from_dir(path: &str) -> Result<T, Error>`: Load and parse configuration from a directory
- `GraftonConfig`: Trait for Grafton configuration structs
- `TokenExpandingConfig`: Trait for configuration structs that support token expansion

## Token Expansion Details

### Basic Syntax

Tokens use the following format in your TOML files:

```
${key_path}
```

Where `key_path` is a dot-separated path to the desired value within the TOML structure.

### Advanced Usage

- **Nested Paths**: Use dot notation to access nested values.

```toml
[person]
first_name = "John"
last_name = "Doe"

full_name = "${person.first_name} ${person.last_name}"
```

- **Array Access**: Use numeric indices to access array elements.

```toml
fruits = ["apple", "banana", "cherry"]
favorite = "My favorite fruit is ${fruits.1}"
```

- **Tokens in Tables and Arrays**: Tokens can be used in table keys, values, and array elements.

```toml
[users]
names = ["${user1}", "${user2}"]

[metadata]
created_by = "${admin.name}"
```

- **Escaping Tokens**: To use a literal `${}`, escape it with a backslash: `\${`.

```toml
literal = "This is a \${literal} dollar sign"
```

### Behavior and Limitations

- **Data Types**: Can expand to various TOML data types (string, integer, float, boolean, datetime).
- **Circular References**: Will result in an error due to the recursion limit. Current limit is depth 99.
- **Partial Expansions**: Tokens are expanded as much as possible, with unexpandable parts remaining.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Grafton Config is dual-licensed under the Apache License 2.0 (for open-source use) and a commercial license.

### Open Source License

Unless explicitly stated otherwise, all files in this repository are licensed under the Apache License 2.0. The full text of the license can be found in the LICENSE file.

#### Key Features of the Apache License 2.0

- **Permissive License**: Enables free commercial and non-commercial use, distribution, and modification of the software without requiring source code access.
- **Explicit Patent License**: Grants an express patent license from all contributors to users.
- **Patent Retaliation Provision**: Protects users and contributors from patent infringement claims.

### Commercial License

For those wishing to integrate Grafton Config into closed-source applications or who need more flexibility than the Apache License 2.0 allows, a commercial license is available.

#### Benefits of the Commercial License

- **Use in Proprietary Applications**: Integrate Grafton Config seamlessly into closed-source applications.
- **Flexibility and Freedom**: Greater flexibility in the use, modification, and distribution of the project.
- **Support and Warranty**: Access to enhanced support, maintenance services, and warranty options.

For commercial licensing inquiries, please contact grant@grafton.ai.

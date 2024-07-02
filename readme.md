# Grafton Config

Grafton Config is a flexible and powerful configuration library for Rust applications, featuring token expansion and layered configuration loading.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Basic Usage](#basic-usage)
  - [Configuration Loading](#configuration-loading)
  - [Token Expansion](#token-expansion)
- [Examples](#examples)
  - [Simple Configuration](#simple-configuration)
  - [Token Expansion Examples](#token-expansion-examples)
  - [Generic Application Example](#generic-application-example)
- [API Reference](#api-reference)
- [Token Expansion Details](#token-expansion-details)
  - [Basic Syntax](#basic-syntax)
  - [Advanced Usage](#advanced-usage)
  - [Behavior and Limitations](#behavior-and-limitations)
- [Contributing](#contributing)
- [License](#license)

## Features

- Layered configuration loading from multiple TOML files
- Dynamic token expansion within configuration files
- Support for environment-specific configurations
- Flexible and extensible design
- Integration with popular Rust web frameworks

## Installation

Add Grafton Config to your `Cargo.toml`:

```toml
[dependencies]
grafton-config = "0.1.0"
```

## Usage

### Basic Usage

1. Create your configuration struct:

```rust
use serde::{Deserialize, Serialize};
use grafton_config::TokenExpandingConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct MyConfig {
    // Your configuration fields here
}

impl TokenExpandingConfig for MyConfig {}
```

1. Load the configuration:

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

1. `default.toml`
2. `local.toml`
3. `{run_mode}.toml`

The `run_mode` is determined by the `RUN_MODE` environment variable, defaulting to "dev" if not set. Files are loaded in the order listed above, with later files overriding values from earlier ones.

### Token Expansion

Grafton Config supports token expansion within your TOML files. Use the `${key_path}` syntax to reference other values in your configuration.

## Examples

### Simple Configuration

```toml
# config/default.toml
[database]
host = "localhost"
port = 5432
url = "postgresql://user:password@${database.host}:${database.port}/mydb"
```

### Token Expansion Examples

```toml
# Basic expansion
name = "John"
greeting = "Hello, ${name}!"

# Nested expansion
[person]
first_name = "John"
last_name = "Doe"
full_name = "${person.first_name} ${person.last_name}"

# Mixed types
age = 30
is_student = false
profile = "Age: ${age}, Student: ${is_student}"
```

### Generic Application Example

Here's an example of using Grafton Config in a typical Rust application:

```rust
use serde::{Deserialize, Serialize};
use grafton_config::{load_config_from_dir, TokenExpandingConfig, Error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub debug_mode: bool,
}

impl TokenExpandingConfig for AppConfig {}

fn main() -> Result<(), Error> {
    // Load the configuration from the 'config' directory
    let config: AppConfig = load_config_from_dir("config")?;

    // Use the configuration
    println!("Database URL: postgres://{}:{}@{}:{}/{}",
        config.database.username,
        config.database.password,
        config.database.host,
        config.database.port,
        config.database.database_name
    );

    println!("Server will start on {}:{}", config.server.host, config.server.port);
    println!("Debug mode: {}", config.debug_mode);

    // Your application logic here...

    Ok(())
}
```

Example TOML files:

```toml
# config/default.toml
[database]
host = "localhost"
port = 5432
username = "default_user"
password = "default_password"
database_name = "myapp"

[server]
host = "127.0.0.1"
port = 8080

debug_mode = false

# config/local.toml
[database]
username = "dev_user"
password = "dev_password"

debug_mode = true

# config/prod.toml
[database]
host = "db.production.com"
username = "${PROD_DB_USER}"
password = "${PROD_DB_PASSWORD}"

[server]
host = "0.0.0.0"
port = 80

debug_mode = false
```

This setup demonstrates:

- A layered configuration approach with default, local, and production settings
- Use of token expansion for sensitive information in the production config
- A typical structure for a Rust application using Grafton Config

## API Reference

- `load_config_from_dir(path: &str) -> Result<T, Error>`: Load and parse configuration from a directory
- `expand_tokens(config: &mut T) -> Result<(), Error>`: Expand tokens in a configuration struct
- `GraftonConfig`: Trait for Grafton configuration structs
- `TokenExpandingConfig`: Trait for configuration structs that support token expansion

## Token Expansion Details

### Basic Syntax

Tokens use the following format in your TOML files:

```toml
${key_path}
```

Where `key_path` is a dot-separated path to the desired value within the TOML structure.

### Advanced Usage

1. **Nested Paths**: Use dot notation to access nested values.

   ```toml
   [person]
   first_name = "John"
   last_name = "Doe"

   full_name = "${person.first_name} ${person.last_name}"
   ```

2. **Array Access**: Use numeric indices to access array elements.

   ```toml
   fruits = ["apple", "banana", "cherry"]
   favorite = "My favorite fruit is ${fruits.1}"
   ```

3. **Tokens in Tables and Arrays**: Tokens can be used in table keys, values, and array elements.

   ```toml
   [users]
   names = ["${user1}", "${user2}"]

   [metadata]
   created_by = "${admin.name}"
   ```

4. **Escaping Tokens**: To use a literal `${`, escape it with a backslash: `\${`.

   ```toml
   literal = "This is a \${literal} dollar sign"
   ```

### Behavior and Limitations

- **Recursion Depth**: Limited to 99 to prevent infinite loops.
- **Non-existent Paths**: Left unexpanded if the path doesn't exist.
- **Data Types**: Can expand to various TOML data types (string, integer, float, boolean, datetime).
- **Circular References**: Will result in an error due to the recursion limit.
- **Partial Expansions**: Tokens are expanded as much as possible, with unexpandable parts remaining.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Grafton Config is dual-licensed under the Apache License 2.0 (for open-source use) and a commercial license.

### Open Source License

Unless explicitly stated otherwise, all files in this repository are licensed under the Apache License 2.0. The full text of the license can be found in the [LICENSE](LICENSE) file.

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

For commercial licensing inquiries, please contact [grant@grafton.ai](mailto:grant@grafton.ai).

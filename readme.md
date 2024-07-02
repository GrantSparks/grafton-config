# Token Expansion in TOML Configuration

This library provides a flexible token expansion system for TOML configurations, allowing dynamic value substitution within your configuration files.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Configuration Loading](#configuration-loading)
- [Usage](#usage)
  - [Basic Syntax](#basic-syntax)
  - [Advanced Usage](#advanced-usage)
- [Behavior and Limitations](#behavior-and-limitations)
- [Examples](#examples)
  - [Simple Expansion](#simple-expansion)
  - [Nested Expansion](#nested-expansion)
  - [Mixed Types](#mixed-types)
  - [Configuration Override Example](#configuration-override-example)
- [Contributing](#contributing)
- [License](#license)

## Overview

The token expansion system allows you to reference and reuse values across your TOML configuration. Tokens are expanded during the parsing process, resulting in a fully resolved configuration.

## Installation

```bash
cargo add your_crate_name
```

## Configuration Loading

The configuration is loaded from the following files in a specified directory:

1. `default.toml`
2. `local.toml`
3. `{run_mode}.toml`

The `run_mode` is determined by the `RUN_MODE` environment variable, defaulting to "dev" if not set.

Files are loaded in the order listed above, with later files overriding values from earlier ones. This allows for a layered configuration approach:

- `default.toml`: Contains default settings
- `local.toml`: Override defaults with local settings (not version controlled)
- `{run_mode}.toml`: Environment-specific overrides (e.g., `dev.toml`, `prod.toml`)

## Usage

### Basic Syntax

Tokens use the following format in your TOML files:

```
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

## Behavior and Limitations

- **Recursion Depth**: Limited to 99 to prevent infinite loops.
- **Non-existent Paths**: Left unexpanded if the path doesn't exist.
- **Data Types**: Can expand to various TOML data types (string, integer, float, boolean, datetime).
- **Circular References**: Will result in an error due to the recursion limit.
- **Partial Expansions**: Tokens are expanded as much as possible, with unexpandable parts remaining.

## Examples

### Simple Expansion

```toml
name = "John"
greeting = "Hello, ${name}!"
```

Expands to:

```toml
name = "John"
greeting = "Hello, John!"
```

### Nested Expansion

```toml
[person]
first_name = "John"
last_name = "Doe"

full_name = "${person.first_name} ${person.last_name}"
```

Expands to:

```toml
[person]
first_name = "John"
last_name = "Doe"

full_name = "John Doe"
```

### Mixed Types

```toml
age = 30
is_student = false
profile = "Age: ${age}, Student: ${is_student}"
```

Expands to:

```toml
age = 30
is_student = false
profile = "Age: 30, Student: false"
```

### Configuration Override Example

Let's consider a realistic scenario for a web application with different environments. We'll use three configuration files: `default.toml`, `local.toml`, and `prod.toml`.

`default.toml`:

```toml
run_mode = "dev"

[website]
bind_address = "127.0.0.1"
port = 8080
public_url = "http://${website.bind_address}:${website.port}"

[database]
host = "localhost"
port = 5432
name = "myapp_db"
user = "db_user"
password = "default_password"
url = "postgresql://${database.user}:${database.password}@${database.host}:${database.port}/${database.name}"

[logging]
level = "info"
file = "logs/myapp.log"

[feature_flags]
enable_new_ui = false
max_users = 100

[oauth]
provider = "google"
client_id = "default_client_id"
client_secret = "default_client_secret"
```

`local.toml`:

```toml
[database]
password = "local_dev_password"

[logging]
level = "debug"

[feature_flags]
enable_new_ui = true
max_users = 10

[oauth]
client_id = "local_client_id"
client_secret = "local_client_secret"
```

`prod.toml`:

```toml
run_mode = "prod"

[website]
bind_address = "0.0.0.0"
port = 80
public_url = "https://example.com"

[database]
host = "db.internal.example.com"
password = "${PROD_DB_PASSWORD}"  # This will be set via an environment variable

[logging]
level = "warn"
file = "/var/log/myapp/production.log"

[feature_flags]
enable_new_ui = true
max_users = 10000

[oauth]
provider = "okta"
client_id = "${PROD_OAUTH_CLIENT_ID}"  # This will be set via an environment variable
client_secret = "${PROD_OAUTH_CLIENT_SECRET}"  # This will be set via an environment variable
```

Now, let's examine how the configuration would resolve in different scenarios:

1. **Development Environment (default)**:
   - The final configuration will be a merge of `default.toml` and `local.toml`.
   - The database will use the local development password.
   - Logging will be set to debug level.
   - The new UI will be enabled, but with a limit of 10 users.
   - OAuth will use the local client ID and secret.

2. **Production Environment**:
   - Set the environment variable `RUN_MODE=prod`.
   - The final configuration will be a merge of `default.toml`, `local.toml`, and `prod.toml`, with `prod.toml` taking precedence.
   - The website will bind to all interfaces (0.0.0.0) on port 80.
   - The database will use the production host and the password from the `PROD_DB_PASSWORD` environment variable.
   - Logging will be set to warn level and output to the production log file.
   - The new UI will be enabled with a limit of 10000 users.
   - OAuth will use Okta as the provider, with client ID and secret from environment variables.

This example demonstrates:

- Layered configuration with sensible defaults and environment-specific overrides.
- Use of token expansion to build complex values (like database URLs) from individual components.
- Integration with environment variables for sensitive information in production.
- How different run modes can significantly alter the application's behavior and settings.

Remember to handle the expansion of environment variables securely and to never commit sensitive information (like production passwords or API keys) to version control. Instead, use placeholders that can be filled by secure environment variables or a secrets management system in your production environment.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Licensing

The Grafton Machine Shed API server repository is dual-licensed, offering both open source and commercial licensing options.

### Open Source License

Unless explicitly stated otherwise, all files in this repository are licensed under the Apache License 2.0. The full text of the license can be found in the [LICENSE](LICENSE) file.

#### Key Features of the Apache License 2.0

- **Permissive License**: Enables free commercial and non-commercial use, distribution, and modification of the software without requiring source code access. Allows combining with other open source licenses.

- **Explicit Patent License**: Grants an express patent license from all contributors to users, protecting both parties from patent infringement claims related to the software.

- **Patent Retaliation Provision**: Terminates the license for any party that files a patent infringement lawsuit alleging that the software infringes a patent, thus protecting users and contributors.

### Commercial License

For those wishing to integrate the Grafton Machine Shed server into closed-source applications, or who need more flexibility than the Apache License 2.0 allows, a commercial license is available. This license permits private modifications and proprietary integration.

#### Benefits of the Commercial License

- **Use in Proprietary Applications**: Integrate the Grafton Machine Shed server seamlessly into closed-source applications.
- **Flexibility and Freedom**: Greater flexibility in the use, modification, and distribution of the project.
- **Support and Warranty**: Access to enhanced support, maintenance services, and warranty options.

## Questions and Commercial Licensing

For any questions about licensing, using the software from the Grafton Machine Shed or to inquire about our commercial license please contact us at [grant@grafton.ai](mailto:grant@grafton.ai).

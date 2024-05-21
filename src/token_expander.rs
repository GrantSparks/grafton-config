use {
    once_cell::sync::Lazy,
    regex::{Captures, Regex},
    serde_json::Value,
};

const TOKEN_RESOLVE_DEPTH_LIMIT: usize = 99;

static TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{(.*?)\}").unwrap());

pub fn expand_tokens(val: &Value) -> Value {
    expand_tokens_helper(val, val, 0, "").unwrap()
}

fn expand_tokens_helper(
    val: &Value,
    root: &Value,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, String> {
    if current_depth > TOKEN_RESOLVE_DEPTH_LIMIT {
        return Err(format!(
            "Token resolve recursion detected at depth {current_depth}. Current path: {current_path}, Current value: {val:?}"
        ));
    }

    match val {
        Value::String(s) => {
            let result = TOKEN_REGEX.replace_all(s, |caps: &Captures| {
                let key_path: Vec<&str> = caps[1].split('.').collect();
                get_value_from_path(&key_path, root).map_or_else(
                    || format!("${{{}}}", key_path.join(".")),
                    |replacement_val| {
                        expand_tokens_helper(
                            &replacement_val,
                            root,
                            current_depth + 1,
                            &key_path.join("."),
                        )
                        .map_or_else(
                            |_| format!("${{{}}}", key_path.join(".")),
                            |expanded_val| match expanded_val {
                                Value::String(s) => s,
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".to_string(),
                                _ => format!("${{{}}}", key_path.join(".")),
                            },
                        )
                    },
                )
            });
            Ok(Value::String(result.to_string()))
        }
        Value::Object(o) => Ok(Value::Object(
            o.iter()
                .map(|(k, v)| {
                    let expanded_path = if current_path.is_empty() {
                        k.to_string()
                    } else {
                        format!("{current_path}.{k}")
                    };
                    (
                        k.clone(),
                        expand_tokens_helper(v, root, current_depth + 1, &expanded_path).unwrap(),
                    )
                })
                .collect(),
        )),
        Value::Array(arr) => Ok(Value::Array(
            arr.iter()
                .map(|v| expand_tokens_helper(v, root, current_depth + 1, current_path).unwrap())
                .collect(),
        )),
        _ => Ok(val.clone()),
    }
}

fn get_value_from_path(key_path: &[&str], root: &Value) -> Option<Value> {
    key_path
        .iter()
        .try_fold(root, |acc, &key| acc.as_object()?.get(key))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestCase {
        input: Value,
        expected: Value,
    }

    impl TestCase {
        fn run(self) {
            let result = expand_tokens(&self.input);
            assert_eq!(result, self.expected, "Failed on input: {:?}", self.input);
        }
    }

    #[test]
    fn test_invalid_token_formatting() {
        let test_cases = vec![
            TestCase {
                input: json!({"name": "John Doe", "alias": "${ name}"}),
                expected: json!({"name": "John Doe", "alias": "${ name}"}),
            },
            TestCase {
                input: json!({"name": "John Doe", "alias": "${name"}),
                expected: json!({"name": "John Doe", "alias": "${name"}),
            },
        ];

        for test_case in test_cases {
            test_case.run();
        }
    }

    #[test]
    fn test_nested_tokens() {
        TestCase {
            input: json!({
                "firstName": "John",
                "lastName": "Doe",
                "fullName": "${firstName} ${lastName}",
                "greeting": "Hello, ${fullName}!"
            }),
            expected: json!({
                "firstName": "John",
                "lastName": "Doe",
                "fullName": "John Doe",
                "greeting": "Hello, John Doe!"
            }),
        }
        .run();
    }

    #[test]
    fn test_non_existent_path() {
        TestCase {
            input: json!({
                "name": "John",
                "message": "Hello, ${nonExistentPath}!"
            }),
            expected: json!({
                "name": "John",
                "message": "Hello, ${nonExistentPath}!"
            }),
        }
        .run();
    }

    #[test]
    fn test_special_characters_in_path() {
        TestCase {
            input: json!({
                "data": {
                    "special key": "value"
                },
                "message": "This is a ${data.special key}."
            }),
            expected: json!({
                "data": {
                    "special key": "value"
                },
                "message": "This is a value."
            }),
        }
        .run();
    }

    #[test]
    fn test_replacement_with_various_types() {
        let scenarios = vec![
            TestCase {
                input: json!({"age": 30, "message": "I am ${age} years old."}),
                expected: json!({"age": 30, "message": "I am 30 years old."}),
            },
            TestCase {
                input: json!({"valid": true, "message": "The statement is ${valid}."}),
                expected: json!({"valid": true, "message": "The statement is true."}),
            },
            TestCase {
                input: json!({"nothing": null, "message": "There is ${nothing} here."}),
                expected: json!({"nothing": null, "message": "There is null here."}),
            },
        ];

        for test_case in scenarios {
            test_case.run();
        }
    }

    #[test]
    fn test_token_value_has_unused_tokens() {
        TestCase {
            input: json!({
                "firstName": "John",
                "unused": "${lastName}",
                "name": "${firstName}"
            }),
            expected: json!({
                "firstName": "John",
                "unused": "${lastName}",
                "name": "John"
            }),
        }
        .run();
    }

    #[test]
    fn test_token_recursion_limit() {
        let json_obj = json!({"recursion": "${recursion}"});

        let panicked = std::panic::catch_unwind(|| {
            expand_tokens(&json_obj);
        });

        assert!(panicked.is_err());
    }

    #[test]
    fn test_mixed_type_array() {
        TestCase {
            input: json!({
                "data": ["${name}", 1, true, null, ["${name}", "${age}"], {"alias": "${name}"}],
                "name": "John",
                "age": 30
            }),
            expected: json!({
                "data": ["John", 1, true, null, ["John", "30"], {"alias": "John"}],
                "name": "John",
                "age": 30
            }),
        }
        .run();
    }

    #[test]
    fn test_multiple_nested_paths() {
        TestCase {
            input: json!({
                "person": {
                    "firstName": "John",
                    "lastName": "Doe",
                    "meta": {
                        "alias": "${person.firstName}-${person.lastName}"
                    }
                }
            }),
            expected: json!({
                "person": {
                    "firstName": "John",
                    "lastName": "Doe",
                    "meta": {
                        "alias": "John-Doe"
                    }
                }
            }),
        }
        .run();
    }

    #[test]
    fn test_empty_json() {
        TestCase {
            input: json!({}),
            expected: json!({}),
        }
        .run();
    }

    #[test]
    fn test_array_in_json() {
        TestCase {
            input: json!({
                "names": ["${name1}", "${name2}"],
                "name1": "John",
                "name2": "Doe"
            }),
            expected: json!({
                "names": ["John", "Doe"],
                "name1": "John",
                "name2": "Doe"
            }),
        }
        .run();
    }

    #[test]
    fn test_mega_case() {
        TestCase {
            input: json!({
                "website": {
                    "bind_address": "127.0.0.1",
                    "plugin_info": {
                        "api": {
                            "url": "https://${website.public_hostname}/chatgpt-plugin/openapi.yaml"
                        },
                        "legal_info_url": "https://${website.public_hostname}/legal",
                        "logo_url": "https://${website.public_hostname}/images/website_logo_500x500.png"
                    },
                    "public_hostname": "localhost"
                }
            }),
            expected: json!({
                "website": {
                    "bind_address": "127.0.0.1",
                    "plugin_info": {
                        "api": {
                            "url": "https://localhost/chatgpt-plugin/openapi.yaml"
                        },
                        "legal_info_url": "https://localhost/legal",
                        "logo_url": "https://localhost/images/website_logo_500x500.png"
                    },
                    "public_hostname": "localhost"
                }
            }),
        }
        .run();
    }
}

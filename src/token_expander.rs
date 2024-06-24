use {
    once_cell::sync::Lazy,
    regex::{Captures, Regex},
    serde_json::Value,
};

const TOKEN_RESOLVE_DEPTH_LIMIT: usize = 99; // The tests will fail below a depth limit of at least 7

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
    assert!(
        current_depth <= TOKEN_RESOLVE_DEPTH_LIMIT,
        "Token resolve recursion detected at depth {current_depth}. Current path: {current_path}, Current value: {val:?}"
    );

    match val {
        Value::String(s) => {
            let result = TOKEN_REGEX.replace_all(s, |caps: &Captures| {
                let key_path: Vec<&str> = caps[1].split('.').collect();
                get_value_from_path(&key_path, root).map_or_else(
                    || format!("${{{}}}", key_path.join(".")),
                    |replacement_val| {
                        expand_tokens_helper(
                            replacement_val,
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
            Ok(Value::String(result.into_owned()))
        }
        Value::Object(o) => {
            let mut map = serde_json::Map::new();
            for (k, v) in o {
                let expanded_path = if current_path.is_empty() {
                    k.to_string()
                } else {
                    format!("{current_path}.{k}")
                };
                map.insert(
                    k.clone(),
                    expand_tokens_helper(v, root, current_depth + 1, &expanded_path)?,
                );
            }
            Ok(Value::Object(map))
        }
        Value::Array(arr) => {
            let mut vec = Vec::with_capacity(arr.len());
            for v in arr {
                vec.push(expand_tokens_helper(
                    v,
                    root,
                    current_depth + 1,
                    current_path,
                )?);
            }
            Ok(Value::Array(vec))
        }
        _ => Ok(val.clone()),
    }
}

fn get_value_from_path<'a>(key_path: &[&str], root: &'a Value) -> Option<&'a Value> {
    key_path
        .iter()
        .try_fold(root, |acc, &key| acc.as_object()?.get(key))
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
    fn test_deeply_nested_recursion_should_panic() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{}", i);
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{}}}", key)));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        let result = std::panic::catch_unwind(|| {
            expand_tokens_helper(
                &Value::Object(deep_json.clone()),
                &Value::Object(deep_json),
                0,
                "",
            )
            .unwrap();
        });

        assert!(result.is_err(), "Test failed: expected panic, got Ok");
    }

    #[test]
    fn test_deeply_nested_objects_with_mixed_types() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{}", i);
            let mut next = serde_json::Map::new();
            next.insert(
                "next".to_string(),
                Value::Array(vec![Value::String(format!("${{{}}}", key))]),
            );
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        // Add a final key that should not exceed the limit
        current.insert("final".to_string(), Value::Bool(true));

        let result = std::panic::catch_unwind(|| {
            expand_tokens_helper(
                &Value::Object(deep_json.clone()),
                &Value::Object(deep_json),
                0,
                "",
            )
            .unwrap();
        });

        assert!(result.is_err(), "Test failed: expected panic, got Ok");
    }

    #[test]
    fn test_multiple_nested_tokens_at_limit() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{}", i);
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{}}}", key)));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        let result = std::panic::catch_unwind(|| {
            expand_tokens_helper(
                &Value::Object(deep_json.clone()),
                &Value::Object(deep_json),
                0,
                "",
            )
            .unwrap();
        });

        assert!(result.is_err(), "Test failed: expected panic, got Ok");
    }

    #[test]
    fn test_recursive_tokens_across_different_paths() {
        TestCase {
            input: json!({
                "a": "${b}",
                "b": "${c}",
                "c": "${d}",
                "d": "${e}",
                "e": "final_value"
            }),
            expected: json!({
                "a": "final_value",
                "b": "final_value",
                "c": "final_value",
                "d": "final_value",
                "e": "final_value"
            }),
        }
        .run();
    }

    #[test]
    fn test_complex_structure_with_array_and_objects() {
        TestCase {
            input: json!({
                "level1": {
                    "array": [
                        {
                            "nested": "${level1.value1}"
                        },
                        "${level1.value2}"
                    ],
                    "value1": "nested_value1",
                    "value2": "nested_value2"
                }
            }),
            expected: json!({
                "level1": {
                    "array": [
                        {
                            "nested": "nested_value1"
                        },
                        "nested_value2"
                    ],
                    "value1": "nested_value1",
                    "value2": "nested_value2"
                }
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
    fn test_deeply_nested_recursion() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT + 1 {
            let key = format!("level{}", i);
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{}}}", key)));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        let result = std::panic::catch_unwind(|| {
            expand_tokens_helper(
                &Value::Object(deep_json.clone()),
                &Value::Object(deep_json),
                0,
                "",
            )
            .unwrap();
        });

        assert!(result.is_err());
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

    #[test]
    fn test_deeply_nested_objects() {
        TestCase {
            input: json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "level4": {
                                "level5": {
                                    "value": "${level1.level2.level3.level4.level5.deep}",
                                    "deep": "nested_value"
                                }
                            }
                        }
                    }
                }
            }),
            expected: json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "level4": {
                                "level5": {
                                    "value": "nested_value",
                                    "deep": "nested_value"
                                }
                            }
                        }
                    }
                }
            }),
        }
        .run();
    }

    #[test]
    fn test_large_json_object() {
        let mut large_json = serde_json::Map::new();
        for i in 0..1000 {
            large_json.insert(format!("key{}", i), json!("value"));
        }
        large_json.insert("replace_me".to_string(), json!("${replace_with}"));
        large_json.insert("replace_with".to_string(), json!("replaced_value"));

        TestCase {
            input: Value::Object(large_json.clone()),
            expected: {
                large_json.insert("replace_me".to_string(), json!("replaced_value"));
                Value::Object(large_json)
            },
        }
        .run();
    }
}

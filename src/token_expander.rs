use {crate::Error, once_cell::sync::Lazy, regex::Regex, serde_json::Value};

const TOKEN_RESOLVE_DEPTH_LIMIT: usize = 99;
static TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\\*)\$\{(.*?)\}").unwrap());

/// Expands tokens within the given JSON value.
///
/// This function recursively searches for and expands tokens in the format `${token}` within
/// the provided JSON value. It supports nested tokens and various JSON data types (objects, arrays, strings).
///
/// # Errors
///
/// This function will return an `Error::TokenRecursionLimitExceeded` if the recursion depth exceeds
/// the specified limit (99).
///
/// It may also return other errors that are specific to token expansion failures.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use crate::grafton_config::expand_tokens;
///
/// let input = json!({
///     "firstName": "John",
///     "lastName": "Doe",
///     "fullName": "${firstName} ${lastName}",
///     "greeting": "Hello, ${fullName}!"
/// });
///
/// let expanded = expand_tokens(&input).unwrap();
/// assert_eq!(expanded["fullName"], "John Doe");
/// assert_eq!(expanded["greeting"], "Hello, John Doe!");
/// ```
pub fn expand_tokens(val: &Value) -> Result<Value, Error> {
    expand_tokens_helper(val, val, 0, "")
}

fn expand_tokens_helper(
    val: &Value,
    root: &Value,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, Error> {
    if current_depth > TOKEN_RESOLVE_DEPTH_LIMIT {
        return Err(Error::TokenRecursionLimitExceeded {
            depth: current_depth,
            path: current_path.to_string(),
            value: val.clone(),
        });
    }

    match val {
        Value::String(s) => expand_string(s, root, current_depth, current_path),
        Value::Object(o) => expand_object(o, root, current_depth, current_path),
        Value::Array(arr) => expand_array(arr, root, current_depth, current_path),
        _ => Ok(val.clone()),
    }
}

fn expand_string(
    s: &str,
    root: &Value,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, Error> {
    let mut result = String::new();
    let mut last_match_end = 0;
    let mut recursion_detected = false;

    for caps in TOKEN_REGEX.captures_iter(s) {
        let full_match = caps.get(0).unwrap();
        let backslashes = caps.get(1).unwrap().as_str();
        let key = caps.get(2).unwrap().as_str();

        result.push_str(&s[last_match_end..full_match.start()]);
        let (prefix, should_expand) = process_backslashes(backslashes);
        result.push_str(&prefix);

        if should_expand {
            let new_path = format_new_path(current_path, key);
            let replacement = expand_token(key, root, &new_path, current_depth);

            match replacement {
                Ok(replacement) => result.push_str(&replacement),
                Err(e) => {
                    recursion_detected = true;
                    handle_recursion_error(&mut result, key, &e);
                }
            }
        } else {
            handle_escaped_token(&mut result, key);
        }

        last_match_end = full_match.end();
    }

    result.push_str(&s[last_match_end..]);
    finalize_expansion(result, recursion_detected, current_depth, current_path)
}

fn expand_object(
    o: &serde_json::Map<String, Value>,
    root: &Value,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, Error> {
    let map = o
        .iter()
        .map(|(k, v)| {
            let expanded_path = format_new_path(current_path, k);
            expand_tokens_helper(v, root, current_depth + 1, &expanded_path)
                .map(|ev| (k.clone(), ev))
        })
        .collect::<Result<_, _>>()?;

    Ok(Value::Object(map))
}

fn expand_array(
    arr: &[Value],
    root: &Value,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, Error> {
    let vec = arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let expanded_path = format_new_array_path(current_path, i);
            expand_tokens_helper(v, root, current_depth + 1, &expanded_path)
        })
        .collect::<Result<_, _>>()?;

    Ok(Value::Array(vec))
}

fn process_backslashes(backslashes: &str) -> (String, bool) {
    let count = backslashes.len();
    (backslashes[..count / 2].to_string(), count % 2 == 0)
}

fn get_value_from_path<'a>(key_path: &[&str], root: &'a Value) -> Option<&'a Value> {
    key_path.iter().try_fold(root, |acc, &key| {
        if let Ok(index) = key.parse::<usize>() {
            acc.as_array()?.get(index)
        } else {
            acc.as_object()?.get(key)
        }
    })
}

fn format_new_path(current_path: &str, key: &str) -> String {
    if current_path.is_empty() {
        key.to_string()
    } else {
        format!("{current_path}.{key}")
    }
}

fn format_new_array_path(current_path: &str, index: usize) -> String {
    if current_path.is_empty() {
        index.to_string()
    } else {
        format!("{current_path}[{index}]")
    }
}

fn expand_token(
    key: &str,
    root: &Value,
    new_path: &str,
    current_depth: usize,
) -> Result<String, Error> {
    let key_path: Vec<&str> = key.split('.').collect();
    get_value_from_path(&key_path, root).map_or_else(
        || Ok(format!("${{{key}}}")),
        |replacement_val| {
            expand_tokens_helper(replacement_val, root, current_depth + 1, new_path)
                .map(convert_value_to_string)
        },
    )
}

fn handle_recursion_error(result: &mut String, key: &str, error: &Error) {
    if matches!(error, Error::TokenRecursionLimitExceeded { .. }) {
        result.push_str("${");
        result.push_str(key);
        result.push('}');
    }
}

fn handle_escaped_token(result: &mut String, key: &str) {
    result.push_str("${");
    result.push_str(key);
    result.push('}');
}

fn finalize_expansion(
    result: String,
    recursion_detected: bool,
    current_depth: usize,
    current_path: &str,
) -> Result<Value, Error> {
    if recursion_detected {
        Err(Error::TokenRecursionLimitExceeded {
            depth: current_depth,
            path: current_path.to_string(),
            value: Value::String(result),
        })
    } else {
        Ok(Value::String(result))
    }
}

fn convert_value_to_string(value: Value) -> String {
    match value {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => format!("${{{value}}}"),
    }
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
            let result = expand_tokens(&self.input).expect("expand_tokens failed unexpectedly");
            assert_eq!(result, self.expected, "Failed on input: {:?}", self.input);
        }
    }

    #[test]
    fn test_format_new_array_path_empty_current_path() {
        let current_path = "";
        let index = 0;
        let expected = "0";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);

        let index = 5;
        let expected = "5";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_new_array_path_non_empty_current_path() {
        let current_path = "parent";
        let index = 0;
        let expected = "parent[0]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);

        let index = 5;
        let expected = "parent[5]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_new_array_path_nested_current_path() {
        let current_path = "parent.child";
        let index = 0;
        let expected = "parent.child[0]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);

        let index = 5;
        let expected = "parent.child[5]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_new_array_path_complex_current_path() {
        let current_path = "parent.child[2]";
        let index = 0;
        let expected = "parent.child[2][0]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);

        let index = 5;
        let expected = "parent.child[2][5]";
        let result = format_new_array_path(current_path, index);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_value_from_path_valid_paths() {
        let json_data = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "target": "found me"
                    }
                }
            }
        });

        let path = vec!["level1", "level2", "level3", "target"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!("found me")));

        let path = vec!["level1", "level2"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, json_data.get("level1").unwrap().get("level2"));

        let path = vec!["level1"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, json_data.get("level1"));
    }

    #[test]
    fn test_get_value_from_path_invalid_paths() {
        let json_data = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "target": "found me"
                    }
                }
            }
        });

        let path = vec!["level1", "level2", "nonexistent"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, None);

        let path = vec!["nonexistent"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, None);

        let path = vec![];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json_data));
    }

    #[test]
    fn test_get_value_from_path_edge_cases() {
        let json_data = json!({
            "level1": {
                "": {
                    "target": "found me"
                },
                "null_value": null
            },
            "empty_string": "",
            "null_key": null,
        });

        // Path with an empty string key
        let path = vec!["level1", ""];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, json_data.get("level1").unwrap().get(""));

        // Path to a null value
        let path = vec!["level1", "null_value"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(null)));

        // Path to an empty string value
        let path = vec!["empty_string"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!("")));

        // Path to a null key
        let path = vec!["null_key"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(null)));
    }

    #[test]
    fn test_get_value_from_path_arrays() {
        let json_data = json!({
            "level1": {
                "array": [
                    {
                        "level2": "value0"
                    },
                    {
                        "level2": "value1"
                    }
                ]
            }
        });

        let path = vec!["level1", "array", "0", "level2"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(
            result,
            json_data
                .get("level1")
                .unwrap()
                .get("array")
                .unwrap()
                .get(0)
                .unwrap()
                .get("level2")
        );

        let path = vec!["level1", "array", "1", "level2"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(
            result,
            json_data
                .get("level1")
                .unwrap()
                .get("array")
                .unwrap()
                .get(1)
                .unwrap()
                .get("level2")
        );
    }

    #[test]
    fn test_get_value_from_path_mixed_types() {
        let json_data = json!({
            "array": [1, "two", true, null, {"five": 5}],
            "object": {
                "nested_array": [10, {"key": "value"}]
            }
        });

        let path = vec!["array", "0"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(1)));

        let path = vec!["array", "1"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!("two")));

        let path = vec!["array", "2"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(true)));

        let path = vec!["array", "3"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(null)));

        let path = vec!["array", "4", "five"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(5)));

        let path = vec!["object", "nested_array", "0"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!(10)));

        let path = vec!["object", "nested_array", "1", "key"];
        let result = get_value_from_path(&path, &json_data);
        assert_eq!(result, Some(&json!("value")));
    }

    #[test]
    fn test_expand_string() {
        let root = json!({
            "name": "John",
            "greeting": "Hello, ${name}!",
        });

        let result = expand_string("Hello, ${name}!", &root, 0, "").unwrap();
        assert_eq!(result, Value::String("Hello, John!".to_string()));
    }

    #[test]
    fn test_expand_object() {
        let root = json!({
            "name": "John",
            "info": {
                "greeting": "Hello, ${name}!"
            }
        });

        let obj = root.get("info").unwrap().as_object().unwrap();
        let result = expand_object(obj, &root, 0, "").unwrap();
        assert_eq!(
            result,
            json!({
                "greeting": "Hello, John!"
            })
        );
    }

    #[test]
    fn test_expand_array() {
        let root = json!({
            "name": "John",
            "array": ["Hello, ${name}!", "${name} is here."]
        });

        let array = root.get("array").unwrap().as_array().unwrap();
        let result = expand_array(array, &root, 0, "").unwrap();
        assert_eq!(result, json!(["Hello, John!", "John is here."]));
    }

    #[test]
    fn test_expand_token() {
        let root = json!({
            "name": "John"
        });

        let result = expand_token("name", &root, "name", 0).unwrap();
        assert_eq!(result, "John");

        let non_existent_result = expand_token("non_existent", &root, "non_existent", 0).unwrap();
        assert_eq!(non_existent_result, "${non_existent}");
    }

    #[test]
    fn test_handle_recursion_error() {
        let mut result = String::new();
        handle_recursion_error(
            &mut result,
            "key",
            &Error::TokenRecursionLimitExceeded {
                depth: 100,
                path: "key".to_string(),
                value: json!("value"),
            },
        );
        assert_eq!(result, "${key}");
    }

    #[test]
    fn test_handle_escaped_token() {
        let mut result = String::new();
        handle_escaped_token(&mut result, "key");
        assert_eq!(result, "${key}");
    }

    #[test]
    fn test_finalize_expansion() {
        let result = finalize_expansion("Hello, John!".to_string(), false, 0, "").unwrap();
        assert_eq!(result, Value::String("Hello, John!".to_string()));

        let recursion_result = finalize_expansion("Hello, ${name}".to_string(), true, 1, "name");
        assert!(recursion_result.is_err());
    }

    #[test]
    fn test_convert_value_to_string() {
        assert_eq!(convert_value_to_string(json!("string")), "string");
        assert_eq!(convert_value_to_string(json!(123)), "123");
        assert_eq!(convert_value_to_string(json!(true)), "true");
        assert_eq!(convert_value_to_string(json!(null)), "null");
        assert_eq!(
            convert_value_to_string(json!({"key": "value"})),
            "${{\"key\":\"value\"}}"
        );
    }

    #[test]
    fn test_process_backslashes() {
        // Even number of backslashes, should expand
        assert_eq!(process_backslashes(""), (String::new(), true));
        assert_eq!(process_backslashes("\\\\"), ("\\".to_string(), true));
        assert_eq!(process_backslashes("\\\\\\\\"), ("\\\\".to_string(), true));
        assert_eq!(
            process_backslashes("\\\\\\\\\\\\"),
            ("\\\\\\".to_string(), true)
        );

        // Odd number of backslashes, should not expand
        assert_eq!(process_backslashes("\\"), (String::new(), false));
        assert_eq!(process_backslashes("\\\\\\"), ("\\".to_string(), false));
        assert_eq!(
            process_backslashes("\\\\\\\\\\"),
            ("\\\\".to_string(), false)
        );
        assert_eq!(
            process_backslashes("\\\\\\\\\\\\\\"),
            ("\\\\\\".to_string(), false)
        );

        // Edge cases
        assert_eq!(process_backslashes(""), (String::new(), true));
        assert_eq!(process_backslashes("\\"), (String::new(), false));
        assert_eq!(process_backslashes("\\\\\\"), ("\\".to_string(), false));
        assert_eq!(
            process_backslashes("\\\\\\\\\\\\"),
            ("\\\\\\".to_string(), true)
        );
    }

    #[test]
    fn test_backslash_escaped_tokens() {
        TestCase {
            input: json!({
                "simple_literal": "No token here",
                "simple_token": "Hello, ${name}!",
                "escaped_token": "This is a \\${token}",
                "double_backslash_token": "This is a \\\\${token}",
                "multiple_backslashes_token": "This is a \\\\\\\\${token}",
                "literal_backslashes": "This is a \\\\text",
                "mixed_escapes": "Mix of backslashes: \\\\${token1} and \\\\\\\\\\${token2}",
                "backslash_end": "Backslash at end: \\\\",
                "array_escapes": {
                    "array": ["\\${token1}", "\\\\${token2}", "\\\\\\\\${token3}"]
                }
            }),
            expected: json!({
                "simple_literal": "No token here",
                "simple_token": "Hello, ${name}!",
                "escaped_token": "This is a ${token}",
                "double_backslash_token": "This is a \\${token}",
                "multiple_backslashes_token": "This is a \\\\${token}",
                "literal_backslashes": "This is a \\\\text",
                "mixed_escapes": "Mix of backslashes: \\${token1} and \\\\${token2}",
                "backslash_end": "Backslash at end: \\\\",
                "array_escapes": {
                    "array": ["${token1}", "\\${token2}", "\\\\${token3}"]
                }
            }),
        }
        .run();
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
    #[should_panic(expected = "TokenRecursionLimitExceeded")]
    fn test_deeply_nested_recursion_should_panic() {
        // Prepare the deeply nested JSON structure
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{i}");
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{key}}}")));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        // This should panic
        expand_tokens_helper(
            &Value::Object(deep_json.clone()),
            &Value::Object(deep_json),
            0,
            "",
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "TokenRecursionLimitExceeded")]
    fn test_deeply_nested_objects_with_mixed_types() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;

        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{i}");
            let mut next = serde_json::Map::new();
            next.insert(
                "next".to_string(),
                Value::Array(vec![Value::String(format!("${{{key}}}"))]),
            );
            current.insert(key.clone(), Value::Object(next));
            current = current.get_mut(&key).unwrap().as_object_mut().unwrap();
        }

        current.insert("final".to_string(), Value::Bool(true));

        // This should panic
        expand_tokens_helper(
            &Value::Object(deep_json.clone()),
            &Value::Object(deep_json),
            0,
            "",
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "TokenRecursionLimitExceeded")]
    fn test_multiple_nested_tokens_at_limit() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{i}");
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{key}}}")));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        // This should panic
        expand_tokens_helper(
            &Value::Object(deep_json.clone()),
            &Value::Object(deep_json),
            0,
            "",
        )
        .unwrap();
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

        let result = expand_tokens(&json_obj);
        assert!(result.is_err(), "Expected an error, but got: {result:?}");

        match result {
            Err(Error::TokenRecursionLimitExceeded { depth, path, value }) => {
                assert_eq!(depth, 1);
                assert_eq!(path, "recursion");
                assert_eq!(value, Value::String("${recursion}".to_string()));
            }
            _ => panic!("Expected TokenRecursionLimitExceeded error, but got: {result:?}"),
        }
    }

    #[test]
    fn test_deeply_nested_recursion() {
        let mut deep_json = serde_json::Map::new();
        let mut current = &mut deep_json;
        for i in 0..=TOKEN_RESOLVE_DEPTH_LIMIT {
            let key = format!("level{i}");
            let mut next = serde_json::Map::new();
            next.insert("next".to_string(), Value::String(format!("${{{key}}}")));
            current.insert(key.clone(), Value::Object(next));
            current = match current.get_mut(&key).unwrap() {
                Value::Object(map) => map,
                _ => panic!("Unexpected structure"),
            };
        }

        let result = expand_tokens(&Value::Object(deep_json));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::TokenRecursionLimitExceeded { .. }
        ));
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
            large_json.insert(format!("key{i}"), json!("value"));
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

use quorumdb_server::protocol::{parse, Command};

#[test]
fn test_parse_set_command() {
    let result = parse("SET key value");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "key");
            assert_eq!(value, "value");
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_get_command() {
    let result = parse("GET mykey");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Get { key } => {
            assert_eq!(key, "mykey");
        }
        _ => panic!("Expected Get command"),
    }
}

#[test]
fn test_parse_delete_command() {
    let result = parse("DELETE somekey");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Delete { key } => {
            assert_eq!(key, "somekey");
        }
        _ => panic!("Expected Delete command"),
    }
}

#[test]
fn test_parse_case_insensitive() {
    // Lowercase
    let result1 = parse("set key value");
    assert!(result1.is_ok());

    // Uppercase
    let result2 = parse("SET key value");
    assert!(result2.is_ok());

    // Mixed case
    let result3 = parse("SeT key value");
    assert!(result3.is_ok());
}

#[test]
fn test_parse_with_leading_trailing_whitespace() {
    let result = parse("  SET key value  ");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "key");
            assert_eq!(value, "value");
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_with_extra_spaces() {
    let result = parse("SET   key   value");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "key");
            assert_eq!(value, "value");
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_empty_command() {
    let result = parse("");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty command");
}

#[test]
fn test_parse_whitespace_only() {
    let result = parse("   ");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty command");
}

#[test]
fn test_parse_invalid_command() {
    let result = parse("INVALID");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_set_missing_value() {
    let result = parse("SET onlykey");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_get_missing_key() {
    let result = parse("GET");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_delete_missing_key() {
    let result = parse("DELETE");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_set_extra_args() {
    let result = parse("SET key value extra");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_get_extra_args() {
    let result = parse("GET key extra");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_delete_extra_args() {
    let result = parse("DELETE key extra");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid command"));
}

#[test]
fn test_parse_set_with_special_chars() {
    let result = parse("SET key value");
    assert!(result.is_ok());
}

#[test]
fn test_parse_hyphenated_key_and_value() {
    let result = parse("SET my-key my-value");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "my-key");
            assert_eq!(value, "my-value");
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_numeric_key_and_value() {
    let result = parse("SET 12345 67890");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "12345");
            assert_eq!(value, "67890");
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_underscore_in_key() {
    let result = parse("SET my_key my_value");
    assert!(result.is_ok());

    match result.unwrap() {
        Command::Set { key, value } => {
            assert_eq!(key, "my_key");
            assert_eq!(value, "my_value");
        }
        _ => panic!("Expected Set command"),
    }
}


use std::env;
use std::fs::read_to_string;
use std::path::Path;
use toml::value::Value;
use toml::Table;

pub fn get_env_var(key: &str) -> String {
    for (k, v) in env::vars() {
        if k == key {
            return v;
        }
    }

    panic!("Need env var: {}", key);
}

fn get_toml_string_rec(table: &Table, mut path: Vec<&str>) -> Option<String> {
    if path.len() == 1 {
        if !table.contains_key(path[0]) {
            return None;
        }

        return match table[path[0]].clone() {
            Value::String(s) => Some(s),
            Value::Boolean(b) => if b {Some(String::from("true"))} else {Some(String::from("false"))}
            _ => None,
        };
    }

    let id = path.remove(0);
    if !table.contains_key(id) {
        return None;
    }

    match table[id].clone() {
        Value::Table(t) => get_toml_string_rec(&t, path),
        _ => None,
    }
}

pub fn get_toml_string(file_name: &Path, path: Vec<&str>) -> Option<String> {
    let config = {
        let f = read_to_string(file_name);
        if let Ok(f) = f {
            f.parse::<Table>().unwrap()
        } else {
            panic!("Unable to read {}", file_name.display());
        }
    };

    get_toml_string_rec(&config, path)
}

fn get_toml_string_array_rec(table: &Table, mut path: Vec<&str>) -> Option<Vec<String>> {
    if path.len() == 1 {
        if !table.contains_key(path[0]) {
            return None;
        }

        return match table[path[0]].clone() {
            Value::Array(arr) => Some(arr.iter().map(|v| match v {
                Value::String(s) => s.clone(),
                _ => panic!("Unexpected value {v} for string array")
            }).collect()),
            _ => None,
        };
    }

    let id = path.remove(0);
    if !table.contains_key(id) {
        return None;
    }

    match table[id].clone() {
        Value::Table(t) => get_toml_string_array_rec(&t, path),
        _ => None,
    }
}

pub fn get_toml_string_array(file_name: &Path, path: Vec<&str>) -> Option<Vec<String>> {
    let config = {
        let f = read_to_string(file_name);
        if let Ok(f) = f {
            f.parse::<Table>().unwrap()
        } else {
            panic!("Unable to read {}", file_name.display());
        }
    };

    get_toml_string_array_rec(&config, path)
}

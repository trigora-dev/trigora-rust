use std::env;
use std::fs;

fn durable_functions(source: &str) -> Vec<(String, usize)> {
    let mut functions = Vec::new();
    for (index, _) in source.match_indices("pub async fn ") {
        let rest = &source[index + "pub async fn ".len()..];
        let name = rest
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .next()
            .unwrap_or("");
        let params_at = rest.find('(').expect("function parameters");
        let params = &rest[params_at + 1..];
        let mut depth = 1;
        let mut end = 0;
        for (offset, ch) in params.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = offset;
                        break;
                    }
                }
                _ => {}
            }
        }
        let arity = parameter_count(&params[..end]);
        functions.push((name.to_string(), arity));
    }
    functions.sort();
    functions
}

fn parameter_count(params: &str) -> usize {
    let mut depth = 0;
    let mut count = 0;
    let mut started = false;
    for ch in params.chars() {
        match ch {
            '<' | '(' | '[' => {
                depth += 1;
                started = true;
            }
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                count += 1;
                started = false;
            }
            ch if !ch.is_whitespace() => started = true,
            _ => {}
        }
    }
    if started {
        count += 1;
    }
    count
}

#[test]
fn durable_signatures_match_the_compiler_prelude() {
    let own = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    let parsed = durable_functions(&own);
    let expected = vec![
        ("effect".to_string(), 2),
        ("invoke".to_string(), 2),
        ("join".to_string(), 2),
        ("race".to_string(), 2),
        ("sleep".to_string(), 1),
        ("wait_for_event".to_string(), 1),
    ];
    assert_eq!(parsed, expected);

    let Some(prelude) = env::var_os("TCC_RUST_PRELUDE") else {
        return;
    };
    let prelude_source = fs::read_to_string(prelude).unwrap();
    assert_eq!(durable_functions(&prelude_source), parsed);
}

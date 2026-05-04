use crate::lambda::DeBruijn;
use crate::parser::parse_with_env;
use crate::script::build_env_from_content;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Default)]
struct LspDocuments {
    texts: HashMap<String, String>,
}

/// LSP サーバーの基本情報
pub struct LspServer;

impl LspServer {
    /// LSP サーバーを起動
    pub fn start() {
        eprintln!("Lambda LSP server initialized");
    }
}

/// LSP サーバーを起動（JSON-RPC over stdio スタブ）
pub async fn run_lsp_server() -> anyhow::Result<()> {
    eprintln!("Lambda LSP server initialized on stdin/stdout");
    let debug = lsp_debug_enabled();
    let trace = env::var("LAMBDA_LSP_TRACE").unwrap_or_else(|_| "off".to_string());
    if debug {
        eprintln!("[LSP DEBUG] enabled=true trace={trace}");
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut documents = LspDocuments::default();
    let mut shutdown_requested = false;

    loop {
        let Some(raw) = read_lsp_message(&mut reader)? else {
            // EOF
            break;
        };

        if debug && trace != "off" {
            eprintln!("[LSP DEBUG] <= {raw}");
        }

        let msg: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Invalid JSON-RPC message: {err}");
                continue;
            }
        };

        let method = msg.get("method").and_then(Value::as_str);
        let id = msg.get("id").cloned();

        match method {
            Some("initialize") => {
                if let Some(id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": 1,
                                "hoverProvider": true,
                                "completionProvider": {
                                    "resolveProvider": false,
                                    "triggerCharacters": [" ", "\\", ".", "(", "\""]
                                },
                                "inlayHintProvider": true
                            },
                            "serverInfo": {
                                "name": "lambda-lsp",
                                "version": "0.1.0"
                            }
                        }
                    });
                    write_lsp_message(&mut writer, &response)?;
                    if debug && trace == "verbose" {
                        eprintln!("[LSP DEBUG] => initialize response");
                    }
                }
            }
            Some("initialized") => {
                // 通知なので応答不要
            }
            Some("textDocument/didOpen") => {
                update_document_from_params(&mut documents, msg.get("params"));
            }
            Some("textDocument/didChange") => {
                update_document_from_change(&mut documents, msg.get("params"));
            }
            Some("textDocument/didClose") => {
                remove_document_from_params(&mut documents, msg.get("params"));
            }
            Some("shutdown") => {
                shutdown_requested = true;
                if let Some(id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    write_lsp_message(&mut writer, &response)?;
                    if debug && trace != "off" {
                        eprintln!("[LSP DEBUG] => shutdown response");
                    }
                }
            }
            Some("exit") => {
                if debug {
                    eprintln!("[LSP DEBUG] exit received");
                }
                break;
            }
            Some("textDocument/hover") => {
                if let Some(id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    write_lsp_message(&mut writer, &response)?;
                }
            }
            Some("textDocument/completion") => {
                if let Some(id) = id {
                    let result = build_completion_result(&documents, msg.get("params"));
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result
                    });
                    write_lsp_message(&mut writer, &response)?;
                }
            }
            Some("textDocument/inlayHint") => {
                if let Some(id) = id {
                    let result = build_inlay_hints(&documents, msg.get("params"));
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result
                    });
                    write_lsp_message(&mut writer, &response)?;
                }
            }
            Some(_) => {
                // 未実装メソッド。request なら null を返して落ちないようにする。
                if let Some(id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    write_lsp_message(&mut writer, &response)?;
                    if debug && trace == "verbose" {
                        eprintln!("[LSP DEBUG] => fallback null response");
                    }
                }
            }
            None => {
                // method なし（通常は response）。このサーバーでは特に処理しない。
            }
        }

        // shutdown 後に新規リクエストが来てもプロセスは維持し、exit を待つ。
        if shutdown_requested {
            // no-op
        }
    }

    eprintln!("LSP server terminated");
    Ok(())
}

fn lsp_debug_enabled() -> bool {
    match env::var("LAMBDA_LSP_DEBUG") {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

fn update_document_from_params(documents: &mut LspDocuments, params: Option<&Value>) {
    let Some(params) = params else {
        return;
    };
    let Some(uri) = params
        .get("textDocument")
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(text) = params
        .get("textDocument")
        .and_then(|v| v.get("text"))
        .and_then(Value::as_str)
    else {
        return;
    };
    documents.texts.insert(uri.to_string(), text.to_string());
}

fn update_document_from_change(documents: &mut LspDocuments, params: Option<&Value>) {
    let Some(params) = params else {
        return;
    };
    let Some(uri) = params
        .get("textDocument")
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(changes) = params.get("contentChanges").and_then(Value::as_array) else {
        return;
    };
    if let Some(last_change) = changes.last() {
        if let Some(text) = last_change.get("text").and_then(Value::as_str) {
            documents.texts.insert(uri.to_string(), text.to_string());
        }
    }
}

fn remove_document_from_params(documents: &mut LspDocuments, params: Option<&Value>) {
    let Some(params) = params else {
        return;
    };
    let Some(uri) = params
        .get("textDocument")
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
    else {
        return;
    };
    documents.texts.remove(uri);
}

fn build_completion_result(documents: &LspDocuments, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return json!({"isIncomplete": false, "items": []});
    };
    let Some(uri) = params
        .get("textDocument")
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
    else {
        return json!({"isIncomplete": false, "items": []});
    };
    let Some(position) = params.get("position") else {
        return json!({"isIncomplete": false, "items": []});
    };
    let line = position.get("line").and_then(Value::as_u64).unwrap_or(0) as usize;
    let character = position
        .get("character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;

    let Some(text) = documents.texts.get(uri) else {
        return json!({"isIncomplete": false, "items": []});
    };

    let prefix_text = text_before_line(text, line);
    let env = build_env_for_document(prefix_text, uri, line).unwrap_or_default();
    let current_prefix = prefix_from_position(text, line, character);
    let items = completion_items(text, &env, &current_prefix);

    json!({"isIncomplete": false, "items": items})
}

fn build_inlay_hints(documents: &LspDocuments, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return json!([]);
    };
    let Some(uri) = params
        .get("textDocument")
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
    else {
        return json!([]);
    };
    let Some(range) = params.get("range") else {
        return json!([]);
    };
    let Some(text) = documents.texts.get(uri) else {
        return json!([]);
    };

    let start_line = range
        .get("start")
        .and_then(|v| v.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let end_line = range
        .get("end")
        .and_then(|v| v.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(start_line as u64) as usize;

    let all_lines: Vec<&str> = text.lines().collect();
    let mut hints = Vec::new();

    for line_idx in start_line..=end_line.min(all_lines.len().saturating_sub(1)) {
        let line = all_lines[line_idx];
        let trimmed = line.trim_start();
        let prefix_text = text_before_line(text, line_idx);
        let env = build_env_for_document(prefix_text, uri, line_idx).unwrap_or_default();

        if let Some(rest) = trimmed.strip_prefix("reduce_steps") {
            let (steps, expr) = parse_reduce_steps_like(rest);
            if let Some(expr_text) = expr {
                if let Some(hint) = compute_reduce_hint(&env, &expr_text, steps) {
                    hints.push(json!({
                        "position": {"line": line_idx, "character": line.len()},
                        "label": hint,
                        "paddingLeft": true,
                        "paddingRight": true
                    }));
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("reduce") {
            if !rest.starts_with('_') {
                if let Some(hint) = compute_reduce_hint(&env, rest.trim(), None) {
                    hints.push(json!({
                        "position": {"line": line_idx, "character": line.len()},
                        "label": hint,
                        "paddingLeft": true,
                        "paddingRight": true
                    }));
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("assert") {
            if let Some(hint) = compute_assert_hint(&env, rest.trim()) {
                hints.push(json!({
                    "position": {"line": line_idx, "character": line.len()},
                    "label": hint,
                    "paddingLeft": true,
                    "paddingRight": true
                }));
            }
        }
    }

    json!(hints)
}

fn build_env_for_document(
    content_prefix: &str,
    uri: &str,
    line: usize,
) -> Result<HashMap<String, DeBruijn>, Box<dyn std::error::Error>> {
    let base_path = uri_to_path(uri).and_then(|path| path.parent().map(Path::to_path_buf));
    let max_steps = 1000;
    let _ = line;
    build_env_from_content(content_prefix, base_path.as_deref(), max_steps)
}

fn completion_items(text: &str, env: &HashMap<String, DeBruijn>, prefix: &str) -> Vec<Value> {
    let mut candidates: HashSet<String> = HashSet::new();
    let keywords = [
        "reduce",
        "reduce_steps",
        "assert",
        "search",
        "include",
        "from",
        "import",
        "as",
    ];

    for keyword in keywords {
        candidates.insert(keyword.to_string());
    }

    for name in env.keys() {
        candidates.insert(name.clone());
    }

    for token in collect_identifier_tokens(text) {
        candidates.insert(token);
    }

    let mut items: Vec<Value> = candidates
        .into_iter()
        .filter(|candidate| prefix.is_empty() || candidate.starts_with(prefix))
        .take(60)
        .map(|candidate| {
            json!({
                "label": candidate,
                "kind": 14,
                "insertText": candidate,
            })
        })
        .collect();

    items.sort_by(|a, b| {
        a.get("label")
            .and_then(Value::as_str)
            .cmp(&b.get("label").and_then(Value::as_str))
    });

    items
}

fn collect_identifier_tokens(text: &str) -> Vec<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.insert(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        tokens.insert(current);
    }

    tokens.into_iter().collect()
}

fn prefix_from_position(text: &str, line_idx: usize, character: usize) -> String {
    let line = text.lines().nth(line_idx).unwrap_or("");
    let prefix: String = line.chars().take(character).collect();
    let mut collected = String::new();

    for ch in prefix.chars().rev() {
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            collected.push(ch);
        } else {
            break;
        }
    }

    collected.chars().rev().collect()
}

fn text_before_line(text: &str, line_idx: usize) -> &str {
    let mut current_line = 0;
    for (byte_idx, ch) in text.char_indices() {
        if current_line == line_idx {
            return &text[..byte_idx];
        }
        if ch == '\n' {
            current_line += 1;
        }
    }

    if current_line >= line_idx {
        text
    } else {
        text
    }
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let mut decoded = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                decoded.push((h * 16 + l) as char);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i] as char);
        i += 1;
    }
    Some(PathBuf::from(decoded))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_reduce_steps_like(input: &str) -> (Option<usize>, Option<String>) {
    let mut rest = input.trim_start();
    let mut steps = None;

    if let Some(after_paren) = rest.strip_prefix('(') {
        if let Some(end) = after_paren.find(')') {
            steps = after_paren[..end].trim().parse().ok();
            rest = after_paren[end + 1..].trim_start();
        }
    }

    let expr = rest.trim();
    if expr.is_empty() {
        (steps, None)
    } else {
        (steps, Some(expr.to_string()))
    }
}

fn compute_reduce_hint(
    env: &HashMap<String, DeBruijn>,
    expr_text: &str,
    steps: Option<usize>,
) -> Option<String> {
    let parsed = parse_with_env(expr_text, env).ok()?;
    let max_steps = steps.unwrap_or(1000);
    let normalized = parsed.normalize(max_steps);
    Some(format!("=> {}", normalized))
}

fn compute_assert_hint(env: &HashMap<String, DeBruijn>, rest: &str) -> Option<String> {
    let mut input = rest.trim_start();
    let mut steps = None;

    if let Some(after_paren) = input.strip_prefix('(') {
        if let Some(end) = after_paren.find(')') {
            steps = after_paren[..end].trim().parse().ok();
            input = after_paren[end + 1..].trim_start();
        }
    }

    if let Some(after_colon) = input.strip_prefix(':') {
        input = after_colon.trim_start();
    }

    let (left, right) = split_balanced_separator(input, "==")?;
    let left_expr = parse_with_env(left.trim(), env).ok()?;
    let right_expr = parse_with_env(right.trim(), env).ok()?;
    let limit = steps.unwrap_or(1000);
    let left_normal = left_expr.normalize(limit);
    let right_normal = right_expr.normalize(limit);

    if left_normal == right_normal {
        Some("✓".to_string())
    } else {
        Some(format!("✗ {} ≠ {}", left_normal, right_normal))
    }
}

fn split_balanced_separator<'a>(input: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let mut paren_count: i32 = 0;
    let mut idx = 0;

    while idx < input.len() {
        let ch = input[idx..].chars().next()?;
        match ch {
            '(' => paren_count += 1,
            ')' => paren_count = paren_count.saturating_sub(1),
            _ => {}
        }

        if paren_count == 0 && input[idx..].starts_with(separator) {
            return Some((&input[..idx], &input[idx + separator.len()..]));
        }

        idx += ch.len_utf8();
    }

    None
}

fn read_lsp_message<R: BufRead + Read>(reader: &mut R) -> anyhow::Result<Option<String>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }

        let line_trimmed = line.trim_end_matches(['\r', '\n']);
        if line_trimmed.is_empty() {
            break;
        }

        if let Some(rest) = line_trimmed.strip_prefix("Content-Length:") {
            let value = rest.trim().parse::<usize>()?;
            content_length = Some(value);
        }
    }

    let len = content_length.ok_or_else(|| anyhow::anyhow!("Missing Content-Length header"))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(String::from_utf8(body)?))
}

fn write_lsp_message<W: Write>(writer: &mut W, value: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_string(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()?;
    Ok(())
}

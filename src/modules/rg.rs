/// ripgrep integration for content search across files.
use std::path::PathBuf;
use std::process::Stdio;

#[derive(Clone, Debug)]
pub struct ContentSearchResult {
    pub path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub content: String,
}

/// Run ripgrep asynchronously and return results.
/// Searches `query` in `path` (recursively if directory).
pub async fn search(query: &str, path: &PathBuf) -> Vec<ContentSearchResult> {
    let query = query.to_string();
    let path = path.clone();

    tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new("rg");
        cmd.arg("--json")
            .arg("--line-number")
            .arg("--column")
            .arg("--smart-case")
            .arg("--max-count=1")
            .arg("--")
            .arg(&query)
            .arg(&path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_json_lines(&stdout)
    })
    .await
    .unwrap_or_default()
}

fn parse_json_lines(output: &str) -> Vec<ContentSearchResult> {
    let mut results = Vec::new();
    for line in output.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("type").and_then(|t| t.as_str()) == Some("match") {
                if let Some(data) = json.get("data") {
                    let path = data
                        .get("path")
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_str())
                        .map(PathBuf::from)
                        .unwrap_or_default();
                    let line_number = data
                        .get("line_number")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0) as usize;
                    let absolute_offset = data
                        .get("absolute_offset")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0) as usize;
                    let content = data
                        .get("lines")
                        .and_then(|l| l.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .trim_end()
                        .to_string();
                    results.push(ContentSearchResult {
                        path,
                        line_number,
                        column: absolute_offset,
                        content,
                    });
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_rg_json() {
        let json = r#"{"type":"match","data":{"path":{"text":"src/main.rs"},"lines":{"text":"fn main() {\\n"},"line_number":10,"absolute_offset":120}}"#;
        let results = parse_json_lines(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, PathBuf::from("src/main.rs"));
        assert_eq!(results[0].line_number, 10);
        assert_eq!(results[0].content, "fn main() {");
    }

    #[test]
    fn parse_ignores_non_match() {
        let json = r#"{"type":"begin","data":{"path":{"text":"src/main.rs"}}}"#;
        let results = parse_json_lines(json);
        assert!(results.is_empty());
    }
}

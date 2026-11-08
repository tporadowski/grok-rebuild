//! Attaches file content and renders it in the training format
use agent_client_protocol::{BlobResourceContents, EmbeddedResource, EmbeddedResourceResource};
use base64::{Engine as _, engine::general_purpose};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::warn;
use xai_grok_tools::util::truncate::estimate_tokens;
#[cfg(test)]
mod persistence {
    use std::path::PathBuf;
    pub fn session_dir(_suffix: &str) -> PathBuf {
        super::session_scratch_root()
    }
}
/// Files over this many estimated tokens become a metadata-only stub so the model knows they exist without blowing up the context window.
const MAX_FILE_TOKENS: usize = 5_000;
/// 8-char content hash for dedup and collision avoidance.
fn content_hash(content: &[u8]) -> String {
    let hex = format!("{:x}", Sha256::digest(content));
    hex.get(..8).unwrap_or(&hex).to_string()
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReference {
    pub path: PathBuf,
    /// 1-indexed start line (inclusive)
    pub start_line: Option<usize>,
    /// 1-indexed end line (inclusive)
    pub end_line: Option<usize>,
}
impl FileReference {
    /// Parse `@{file}`, `@{file}:N`, `@{file}:N-M`, and the same forms with a leading `!`.
    ///
    /// `:N` is that one line. `:N-M` is the inclusive range. A leading `!` (`@!.env`) is the hidden-file
    /// marker and is not part of the path. `L` before a number is optional (`:L10`, `:L10-L12`).
    pub fn parse(input: &str) -> Option<Self> {
        let re = Regex::new(r"^@?!?([^@!].*?)(?::L?(\d+)(?:-L?(\d+))?)?$").ok()?;
        let caps = re.captures(input)?;
        let path = PathBuf::from(caps.get(1)?.as_str());
        if path.as_os_str().is_empty() {
            return None;
        }
        let start_line: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let end_line: Option<usize> = caps
            .get(3)
            .and_then(|m| m.as_str().parse().ok())
            .or(start_line);
        Some(Self {
            path,
            start_line,
            end_line,
        })
    }
}

/// Turn a typed `@~/...` or `@~` mention into a path under `home`.
///
/// `~other` stays as written. A missing home, or a path that is not a home mention, is unchanged.
/// The caller joins a relative result onto the working directory.
pub fn expand_typed_home_mention(path: &Path, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else {
        return path.to_path_buf();
    };
    let Some(raw) = path.to_str() else {
        return path.to_path_buf();
    };
    if raw == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return if rest.is_empty() {
            home.to_path_buf()
        } else {
            home.join(rest)
        };
    }
    path.to_path_buf()
}

/// Resolve a typed `@` path: `~/` names `home`, anything else is relative to `working_directory`.
pub fn resolve_typed_at_path(path: &Path, working_directory: &Path, home: Option<&Path>) -> PathBuf {
    working_directory.join(expand_typed_home_mention(path, home))
}
/// Cursor clients get `<code_selection>`; others get `<file_contents>`.
/// Content over [`MAX_FILE_TOKENS`] becomes a metadata-only stub so the model still knows the file exists.
pub async fn render_file_reference(file_ref: FileReference, is_cursor: bool) -> Option<String> {
    let read_file = tokio::fs::read(&file_ref.path).await;
    let file_content = if let Ok(read_file_output) = read_file {
        String::from_utf8(read_file_output).ok()
    } else {
        None
    };
    let path = file_ref.path.to_string_lossy();
    let start_line = file_ref.start_line;
    let end_line = file_ref.end_line;
    file_content
        .map(|file_content| {
            let lines: Vec<&str> = file_content.lines().collect();
            let line_offset = start_line.unwrap_or(1);
            let start_idx = (line_offset.saturating_sub(1)).min(lines.len());
            let end_idx = end_line.unwrap_or(lines.len()).min(lines.len());
            let sliced_lines = lines.get(start_idx..end_idx).unwrap_or(&[]);
            let file_content = sliced_lines
                .iter()
                .enumerate()
                .map(|(line_number, content)| {
                    format!("{}→{content}", line_offset + line_number)
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = is_cursor;
            let attrs = match (start_line, end_line) {
                (Some(s), Some(e)) => format!(r#"startLine="{}" endLine="{}""#, s, e),
                _ => r#"isFullFile="true""#.to_string(),
            };
            if estimate_tokens(&file_content) > MAX_FILE_TOKENS {
                return format!(
                r#"<file_contents path="{path}" {attrs} skipped="true" reason="file too large (~{} estimated tokens, limit {MAX_FILE_TOKENS}). Use read_file tool to read specific sections."/>"#,
                estimate_tokens(&file_content),
            );
            }
            format!(
            r#"<file_contents path="{path}" {attrs}>
{file_content}
</file_contents>"#
        )
        })
}
const FILE_REGEX: &str = r"^(?:file://)?([^#]+)(?:#L(\d+)-L?(\d+))?$";
/// Cursor clients get `<code_selection>`; others get `<file_contents>`. URIs are `file://[path]#L[start]-[end]`.
/// Over [`MAX_FILE_TOKENS`], text is written under `pasted/` so the model can `read_file` sections; binary blobs go to `attachments/` with a path hint.
pub async fn render_embedded_resource(
    resource: &EmbeddedResource,
    is_cursor: bool,
) -> Option<String> {
    match &resource.resource {
        EmbeddedResourceResource::TextResourceContents(text_resource)
            if text_resource.mime_type.as_deref() == Some("text/x-diff") =>
        {
            render_diff_resource(text_resource).await
        }
        EmbeddedResourceResource::TextResourceContents(text_resource) => {
            render_text_resource(text_resource, is_cursor).await
        }
        EmbeddedResourceResource::BlobResourceContents(blob) => render_blob_attachment(blob).await,
        _ => None,
    }
}
async fn render_text_resource(
    text_resource: &agent_client_protocol::TextResourceContents,
    is_cursor: bool,
) -> Option<String> {
    let re = Regex::new(FILE_REGEX).ok()?;
    let caps = re.captures(&text_resource.uri)?;
    let path = caps.get(1)?.as_str();
    let start_line: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
    let end_line: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());
    let line_offset = start_line.unwrap_or(1);
    let file_content = text_resource
        .text
        .lines()
        .enumerate()
        .map(|(line_number, content)| format!("{}→{content}", line_offset + line_number))
        .collect::<Vec<_>>()
        .join("\n");
    let attrs = match (start_line, end_line) {
        (Some(s), Some(e)) => format!(r#"startLine="{}" endLine="{}""#, s, e),
        _ => r#"isFullFile="true""#.to_string(),
    };
    let (tag, attrs_str) = ("file_contents", attrs);
    let _ = is_cursor;
    if estimate_tokens(&file_content) > MAX_FILE_TOKENS {
        let dest = write_to_session_subdir("pasted", path, text_resource.text.as_bytes()).await;
        let display_path = dest
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        return Some(format!(
            r#"<{tag} path="{display_path}" {attrs_str} skipped="true" reason="file too large (~{} estimated tokens, limit {MAX_FILE_TOKENS}). Use read_file tool to read specific sections."/>"#,
            estimate_tokens(&file_content),
        ));
    }
    Some(format!(
        r#"<{tag} path="{path}" {attrs_str}>
{file_content}
</{tag}>"#
    ))
}
/// Render a diff citation as `<diff_contents>`.
///
/// The text is passed through verbatim (no line-number rewriting) since it represents a change, not a file snapshot.
async fn render_diff_resource(
    text_resource: &agent_client_protocol::TextResourceContents,
) -> Option<String> {
    let re = Regex::new(FILE_REGEX).ok()?;
    let caps = re.captures(&text_resource.uri)?;
    let path = caps.get(1)?.as_str();
    let start_line: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
    let end_line: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());
    let mut attrs = Vec::new();
    if let Some(s) = start_line {
        attrs.push(format!(r#"startLine="{s}""#));
    }
    if let Some(e) = end_line {
        attrs.push(format!(r#"endLine="{e}""#));
    }
    let attrs_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };
    if estimate_tokens(&text_resource.text) > MAX_FILE_TOKENS {
        let dest = write_to_session_subdir("pasted", path, text_resource.text.as_bytes()).await;
        let display_path = dest
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        return Some(format!(
            r#"<diff_contents path="{display_path}"{attrs_str} skipped="true" reason="diff too large (~{} estimated tokens, limit {MAX_FILE_TOKENS}). Use read_file tool to read specific sections."/>"#,
            estimate_tokens(&text_resource.text),
        ));
    }
    Some(format!(
        r#"<diff_contents path="{path}"{attrs_str}>
{text}
</diff_contents>"#,
        text = text_resource.text,
    ))
}
/// Decode base64 blob, write to `attachments/`, return `<file_contents type="binary">` hint.
async fn render_blob_attachment(blob: &BlobResourceContents) -> Option<String> {
    let raw_name = blob.uri.strip_prefix("file://").unwrap_or(&blob.uri);
    let filename = PathBuf::from(raw_name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".to_string());
    let bytes = match general_purpose::STANDARD.decode(&blob.blob) {
        Ok(b) => b,
        Err(e) => {
            warn!("binary attachment {filename}: base64 decode failed: {e}");
            return None;
        }
    };
    let size = bytes.len();
    let mime = blob
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let dest = write_to_session_subdir("attachments", &filename, &bytes).await?;
    let path = dest.to_string_lossy();
    Some(format!(
        r#"<file_contents type="binary" path="{path}" mime_type="{mime}" size="{size}"/>"#
    ))
}
/// Base directory for Phase-1 session-scoped scratch files; the real session_dir lives in shell persistence.
/// The PID suffix keeps concurrent test processes sharing `/tmp` from colliding on identical content-hash paths or racing each other's cleanup.
fn session_scratch_root() -> PathBuf {
    std::env::temp_dir().join(format!("grok-test-sessions-{}", std::process::id()))
}
/// The content-hash prefix in the filename dedups: identical content maps to the same path, different content to a unique path.
async fn write_to_session_subdir(subdir: &str, filename: &str, content: &[u8]) -> Option<PathBuf> {
    let dir = session_scratch_root().join(subdir);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!("failed to create {subdir} directory: {e}");
        return None;
    }
    let basename = PathBuf::from(filename)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| filename.to_string());
    let hash = content_hash(content);
    let dest = dir.join(format!("{hash}-{basename}"));
    if dest.exists() {
        return Some(dest);
    }
    if let Err(e) = tokio::fs::write(&dest, content).await {
        warn!("failed to write to {}: {e}", dest.display());
        return None;
    }
    Some(dest)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_at_tilde_slash_resolves_to_the_home_folder() {
        let home = PathBuf::from("/home/ada");
        let cwd = PathBuf::from("/work/repo");
        let parsed = FileReference::parse("~/notes.txt").expect("parse ~/notes.txt");
        assert_eq!(
            resolve_typed_at_path(&parsed.path, &cwd, Some(&home)),
            home.join("notes.txt")
        );
        let ranged = FileReference::parse("~/notes.txt:10-12").expect("parse range");
        assert_eq!(ranged.start_line, Some(10));
        assert_eq!(ranged.end_line, Some(12));
        assert_eq!(
            resolve_typed_at_path(&ranged.path, &cwd, Some(&home)),
            home.join("notes.txt")
        );
        let bare = FileReference::parse("~").expect("parse ~");
        assert_eq!(resolve_typed_at_path(&bare.path, &cwd, Some(&home)), home);
        let slash = FileReference::parse("~/").expect("parse ~/");
        assert_eq!(resolve_typed_at_path(&slash.path, &cwd, Some(&home)), home);
        let relative = FileReference::parse("src/main.rs").expect("parse relative");
        assert_eq!(
            resolve_typed_at_path(&relative.path, &cwd, Some(&home)),
            cwd.join("src/main.rs")
        );
        let other = FileReference::parse("~ada/notes.txt").expect("parse ~user");
        assert_eq!(
            resolve_typed_at_path(&other.path, &cwd, Some(&home)),
            cwd.join("~ada/notes.txt")
        );
        let missing_home = FileReference::parse("~/notes.txt").expect("parse again");
        assert_eq!(
            resolve_typed_at_path(&missing_home.path, &cwd, None),
            cwd.join("~/notes.txt")
        );
    }
    #[tokio::test]
    async fn at_mention_colon_n_and_bang_hidden_file() {
        let single = FileReference::parse("@src/lib.rs:10").expect("single line");
        assert_eq!(single.path, PathBuf::from("src/lib.rs"));
        assert_eq!(single.start_line, Some(10));
        assert_eq!(single.end_line, Some(10));
        let l_form = FileReference::parse("@src/lib.rs:L7").expect("L form");
        assert_eq!((l_form.start_line, l_form.end_line), (Some(7), Some(7)));

        let hidden = FileReference::parse("@!.env").expect("hidden marker");
        assert_eq!(hidden.path, PathBuf::from(".env"));
        assert_eq!(hidden.start_line, None);
        let hidden_range = FileReference::parse("@!.env:2-4").expect("hidden range");
        assert_eq!(hidden_range.path, PathBuf::from(".env"));
        assert_eq!(
            (hidden_range.start_line, hidden_range.end_line),
            (Some(2), Some(4))
        );

        let home = PathBuf::from("/home/ada");
        let cwd = PathBuf::from("/work/repo");
        let tilde = FileReference::parse("@!~/notes.txt:3").expect("hidden home line");
        assert_eq!((tilde.start_line, tilde.end_line), (Some(3), Some(3)));
        assert_eq!(
            resolve_typed_at_path(&tilde.path, &cwd, Some(&home)),
            home.join("notes.txt")
        );

        let dir = std::env::temp_dir().join(format!("grok-at-mention-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let notes = dir.join("notes.txt");
        std::fs::write(&notes, "one\ntwo\nthree\n").expect("notes");
        let secret = dir.join(".secret");
        std::fs::write(&secret, "hidden-line\n").expect("secret");

        let parsed = FileReference::parse(&format!("{}:2", notes.display())).expect("abs :N");
        assert_eq!(parsed.path, notes);
        assert_eq!((parsed.start_line, parsed.end_line), (Some(2), Some(2)));
        let rendered = render_file_reference(parsed, false)
            .await
            .expect("render the one line");
        assert!(rendered.contains("2→two"), "{rendered}");
        assert!(!rendered.contains("1→one"), "{rendered}");
        assert!(!rendered.contains("three"), "{rendered}");

        let mut bang = FileReference::parse("@!.secret").expect("bang name");
        bang.path = resolve_typed_at_path(&bang.path, &dir, None);
        assert_eq!(bang.path, secret);
        let rendered = render_file_reference(bang, false)
            .await
            .expect("render the hidden file");
        assert!(rendered.contains("hidden-line"), "{rendered}");
        assert!(rendered.contains(".secret"), "{rendered}");

        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn test_file_url_regex() {
        let expected = vec![
            (
                "file://Users/alice/first.txt#L10-20",
                "Users/alice/first.txt",
                "10",
                "20",
            ),
            (
                "file://Users/alice/second.txt#L12-L44",
                "Users/alice/second.txt",
                "12",
                "44",
            ),
        ];
        let re = Regex::new(FILE_REGEX).unwrap();
        for (path, expected_path, expected_start, expected_end) in expected {
            let captures = re.captures(path).unwrap();
            assert_eq!(captures.get(1).unwrap().as_str(), expected_path);
            assert_eq!(captures.get(2).unwrap().as_str(), expected_start);
            assert_eq!(captures.get(3).unwrap().as_str(), expected_end);
        }
    }
    #[test]
    fn test_file_path_regex() {
        let path = "/Users/test/019c6024-aef0-7472-89ec-65b62c577c09/prompt_3.txt#L929-L933";
        let re = Regex::new(FILE_REGEX).unwrap();
        let captures = re.captures(path).unwrap();
        assert_eq!(
            captures.get(1).unwrap().as_str(),
            "/Users/test/019c6024-aef0-7472-89ec-65b62c577c09/prompt_3.txt"
        );
        assert_eq!(captures.get(2).unwrap().as_str(), "929");
        assert_eq!(captures.get(3).unwrap().as_str(), "933");
    }
    fn file_reference(
        path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Option<FileReference> {
        Some(FileReference {
            path: PathBuf::from(path),
            start_line,
            end_line,
        })
    }
    #[test]
    fn test_parse_file_references() {
        let data: Vec<(&str, Option<FileReference>)> = vec![
            ("", None),
            ("@", None),
            ("@@foo", None),
            ("foo", file_reference("foo", None, None)),
            ("@foo", file_reference("foo", None, None)),
            (
                "@Users/test/bar",
                file_reference("Users/test/bar", None, None),
            ),
            (
                "@Users/test/bar:1-12",
                file_reference("Users/test/bar", Some(1), Some(12)),
            ),
            // Absolute path, L prefix on start only
            (
                "@/asdf/asdf/asdf/asdf/asdf:L1-12",
                file_reference("/asdf/asdf/asdf/asdf/asdf", Some(1), Some(12)),
            ),
            // Trailing slash in the path, L prefix on both
            (
                "@ssasdf/asdf/dsa/fsda/f/sdf/:L1-L12",
                file_reference("ssasdf/asdf/dsa/fsda/f/sdf/", Some(1), Some(12)),
            ),
            // Absolute path without @ prefix
            (
                "/home/user/project/src/main.rs",
                file_reference("/home/user/project/src/main.rs", None, None),
            ),
            // No @ prefix with line range
            (
                "src/lib.rs:10-20",
                file_reference("src/lib.rs", Some(10), Some(20)),
            ),
            // Dots in path and extension, L-prefixed range
            (
                "@my.project/src/file.test.rs:L100-L200",
                file_reference("my.project/src/file.test.rs", Some(100), Some(200)),
            ),
            // Single-line range (start == end)
            ("@foo.rs:L5-L5", file_reference("foo.rs", Some(5), Some(5))),
        ];
        for (input, expected) in data {
            let reference = FileReference::parse(input);
            assert_eq!(reference, expected, "Failed for input: {input:?}");
        }
    }
    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(estimate_tokens(&"x".repeat(20_000)), 5_000);
    }
    fn test_info(suffix: &str) -> String {
        format!("test-session-{suffix}")
    }
    #[tokio::test]
    async fn test_render_embedded_resource_large_file_skipped() {
        let large_content = "x".repeat(80).repeat(300);
        let _info = test_info("large-file");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                large_content.clone(),
                "file:///project/huge.rs",
            ),
        ));
        let rendered = render_embedded_resource(&resource, true).await.unwrap();
        assert!(rendered.contains("skipped=\"true\""));
        assert!(!rendered.contains("xxxxxxxx"));
        assert!(rendered.contains("pasted"));
        assert!(rendered.contains("huge.rs"));
        let hash = content_hash(large_content.as_bytes());
        let expected_path = persistence::session_dir(&_info).join(format!("pasted/{hash}-huge.rs"));
        assert!(
            expected_path.exists(),
            "expected {}",
            expected_path.display()
        );
        let _ = std::fs::remove_file(&expected_path);
    }
    #[tokio::test]
    async fn test_render_embedded_resource_small_file_included() {
        let _info = test_info("small-file");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                "fn main() {}\n",
                "file:///project/small.rs",
            ),
        ));
        let rendered = render_embedded_resource(&resource, true).await.unwrap();
        assert!(rendered.contains("fn main()"));
        assert!(!rendered.contains("skipped"));
    }
    #[tokio::test]
    async fn test_render_blob_attachment_written_to_disk() {
        use base64::{Engine as _, engine::general_purpose};
        let _info = test_info("blob-write");
        let content = b"fake pdf bytes";
        let encoded = general_purpose::STANDARD.encode(content);
        let resource = EmbeddedResource::new(EmbeddedResourceResource::BlobResourceContents(
            agent_client_protocol::BlobResourceContents::new(encoded.clone(), "file://report.pdf")
                .mime_type(Some("application/pdf".to_string())),
        ));
        let rendered = render_embedded_resource(&resource, false).await.unwrap();
        assert!(rendered.contains("file_contents"), "got: {rendered}");
        assert!(rendered.contains(r#"type="binary""#), "got: {rendered}");
        assert!(rendered.contains("report.pdf"), "got: {rendered}");
        assert!(rendered.contains("application/pdf"), "got: {rendered}");
        assert!(
            rendered.contains(&content.len().to_string()),
            "got: {rendered}"
        );
        assert!(
            !rendered.contains(&encoded),
            "blob leaked into hint: {rendered}"
        );
        let hash = content_hash(content);
        let expected =
            persistence::session_dir(&_info).join(format!("attachments/{hash}-report.pdf"));
        assert!(
            expected.exists(),
            "attachment not written to disk at {}",
            expected.display()
        );
        assert_eq!(std::fs::read(&expected).unwrap(), content);
        let _ = std::fs::remove_file(&expected);
    }
    #[tokio::test]
    async fn test_render_blob_attachment_bad_base64_returns_none() {
        let _info = test_info("blob-bad-b64");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::BlobResourceContents(
            agent_client_protocol::BlobResourceContents::new(
                "!!! not valid base64 !!!",
                "file://bad.pdf",
            ),
        ));
        let result = render_embedded_resource(&resource, false).await;
        assert!(
            result.is_none(),
            "expected None for bad base64, got: {result:?}"
        );
    }
    fn diff_resource(uri: &str, text: &str) -> EmbeddedResource {
        EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(text, uri)
                .mime_type(Some("text/x-diff".to_string())),
        ))
    }
    #[tokio::test]
    async fn test_render_diff_resource_uses_diff_contents_tag() {
        let _info = test_info("diff-small");
        let resource = diff_resource(
            "file:///project/main.rs#L10-L12",
            "+ new line\n  context\n- old line",
        );
        let rendered = render_embedded_resource(&resource, true).await.unwrap();
        assert!(rendered.contains("<diff_contents"), "got: {rendered}");
        assert!(rendered.contains("</diff_contents>"), "got: {rendered}");
        assert!(
            rendered.contains(r#"path="/project/main.rs""#),
            "got: {rendered}"
        );
        assert!(rendered.contains(r#"startLine="10""#), "got: {rendered}");
        assert!(rendered.contains(r#"endLine="12""#), "got: {rendered}");
        assert!(rendered.contains("+ new line"), "got: {rendered}");
        assert!(rendered.contains("- old line"), "got: {rendered}");
        assert!(!rendered.contains("code_selection"), "got: {rendered}");
    }
    #[tokio::test]
    async fn test_render_diff_resource_without_line_range() {
        let _info = test_info("diff-no-range");
        let resource = diff_resource("file:///project/lib.rs", "  unchanged line");
        let rendered = render_embedded_resource(&resource, true).await.unwrap();
        assert!(rendered.contains("<diff_contents"), "got: {rendered}");
        assert!(
            rendered.contains(r#"path="/project/lib.rs""#),
            "got: {rendered}"
        );
        assert!(!rendered.contains("startLine"), "got: {rendered}");
        assert!(!rendered.contains("endLine"), "got: {rendered}");
    }
    #[tokio::test]
    async fn test_render_diff_resource_large_skipped() {
        let large_diff = "x".repeat(80).repeat(300);
        let _info = test_info("diff-large");
        let resource = diff_resource("file:///project/big.rs#L1-L999", &large_diff);
        let rendered = render_embedded_resource(&resource, true).await.unwrap();
        assert!(rendered.contains("<diff_contents"), "got: {rendered}");
        assert!(rendered.contains("skipped=\"true\""), "got: {rendered}");
        assert!(
            !rendered.contains("xxxxxxxx"),
            "full content leaked: {rendered}"
        );
        assert!(rendered.contains("pasted"), "got: {rendered}");
        let hash = content_hash(large_diff.as_bytes());
        let expected = persistence::session_dir(&_info).join(format!("pasted/{hash}-big.rs"));
        let _ = std::fs::remove_file(&expected);
    }
    #[tokio::test]
    async fn test_grok_render_embedded_resource_uses_file_contents_tag() {
        let _info = test_info("grok-text");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                "const x = 1;\nconst y = 2;\n",
                "file:///project/app.ts#L5-L6",
            ),
        ));
        let rendered = render_embedded_resource(&resource, false).await.unwrap();
        assert!(rendered.contains("<file_contents"), "got: {rendered}");
        assert!(rendered.contains("</file_contents>"), "got: {rendered}");
        assert!(rendered.contains(r#"startLine="5""#), "got: {rendered}");
        assert!(rendered.contains(r#"endLine="6""#), "got: {rendered}");
        assert!(!rendered.contains("code_selection"), "got: {rendered}");
    }
    #[tokio::test]
    async fn test_grok_render_embedded_resource_full_file_uses_is_full_file() {
        let _info = test_info("grok-full-file");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                "fn main() {}\n",
                "file:///project/main.rs",
            ),
        ));
        let rendered = render_embedded_resource(&resource, false).await.unwrap();
        assert!(rendered.contains("<file_contents"), "got: {rendered}");
        assert!(rendered.contains(r#"isFullFile="true""#), "got: {rendered}");
        assert!(!rendered.contains("code_selection"), "got: {rendered}");
    }
}

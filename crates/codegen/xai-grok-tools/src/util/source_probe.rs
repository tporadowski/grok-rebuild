//! Preflight for text tools. A pipe, socket, or device is refused before it is
//! opened. A regular file over [`TEXT_TOOL_MAX_BYTES`] is refused before it is read.

use std::path::Path;

use crate::util::binary::is_binary;

/// Same ceiling as `rg --max-filesize 5M`.
pub const TEXT_TOOL_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Why a path cannot be treated as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSourceBlock {
    Directory,
    /// Pipe, socket, device, or any other non-regular file.
    Special,
    TooLarge { len: u64 },
}

/// `None` when the path is missing, unreadable, or a regular file within the byte cap.
/// Callers keep their existing not-found and permission errors for those cases.
pub async fn text_source_block(path: &Path) -> Option<TextSourceBlock> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    if metadata.is_dir() {
        return Some(TextSourceBlock::Directory);
    }
    if !metadata.is_file() {
        return Some(TextSourceBlock::Special);
    }
    let len = metadata.len();
    if len > TEXT_TOOL_MAX_BYTES {
        return Some(TextSourceBlock::TooLarge { len });
    }
    None
}

pub fn text_source_message(display: &Path, block: &TextSourceBlock) -> String {
    match block {
        TextSourceBlock::Directory => {
            format!("Error: {} is a directory, not a file.", display.display())
        }
        TextSourceBlock::Special => format!(
            "Error: {} is not a regular file. Pipes, sockets, and devices cannot be read.",
            display.display()
        ),
        TextSourceBlock::TooLarge { len } => format!(
            "Error: {} is too large to read ({len} bytes, limit {TEXT_TOOL_MAX_BYTES}).",
            display.display()
        ),
    }
}

/// First bytes of a regular file, for the binary check. `None` when the file cannot be opened.
pub async fn text_source_prefix(path: &Path) -> Option<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut buf = vec![0u8; 8192];
    let n = file.read(&mut buf).await.ok()?;
    buf.truncate(n);
    Some(buf)
}

pub fn binary_source_message(display: &Path, extension: &str, bytes: &[u8]) -> Option<String> {
    is_binary(extension, bytes)
        .then(|| format!("Cannot read binary file: {}", display.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_path_is_not_blocked() {
        let dir = tempfile::tempdir().unwrap();
        assert!(text_source_block(&dir.path().join("absent.txt")).await.is_none());
    }

    #[tokio::test]
    async fn directory_special_and_oversized_files_are_blocked() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            text_source_block(dir.path()).await,
            Some(TextSourceBlock::Directory)
        );

        let small = dir.path().join("note.txt");
        std::fs::write(&small, b"hello\n").unwrap();
        assert!(text_source_block(&small).await.is_none());

        let large = dir.path().join("huge.txt");
        let file = std::fs::File::create(&large).unwrap();
        file.set_len(TEXT_TOOL_MAX_BYTES + 1).unwrap();
        assert_eq!(
            text_source_block(&large).await,
            Some(TextSourceBlock::TooLarge {
                len: TEXT_TOOL_MAX_BYTES + 1
            })
        );
        let message = text_source_message(
            Path::new("huge.txt"),
            &TextSourceBlock::TooLarge {
                len: TEXT_TOOL_MAX_BYTES + 1,
            },
        );
        assert!(message.contains("too large"));
        assert!(message.contains(&TEXT_TOOL_MAX_BYTES.to_string()));

        let fifo = dir.path().join("pipe");
        let status = std::process::Command::new("mkfifo").arg(&fifo).status().unwrap();
        assert!(status.success(), "mkfifo failed");
        assert_eq!(
            text_source_block(&fifo).await,
            Some(TextSourceBlock::Special)
        );
        assert!(
            text_source_message(Path::new("pipe"), &TextSourceBlock::Special)
                .contains("not a regular file")
        );
    }

    #[test]
    fn binary_message_uses_the_shared_detector() {
        assert!(binary_source_message(Path::new("a.dat"), "", &[0, 1, 2, 3]).is_some());
        assert!(binary_source_message(Path::new("a.txt"), "txt", b"hello\n").is_none());
    }
}

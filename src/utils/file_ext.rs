//! File extension to language mapping utilities

use std::path::Path;

/// File type enumeration for categorizing files
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileType {
    // Programming languages
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
    C,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Dart,
    Lua,
    R,
    Julia,
    Elixir,
    Elm,

    // Data formats
    Markdown,
    Json,
    Yaml,
    Toml,
    Xml,
    Html,
    Css,

    // Other
    Text,
    Other,
}

impl FileType {
    /// Determine file type from path
    pub fn from_path(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "rs" => FileType::Rust,
            "py" => FileType::Python,
            "js" | "mjs" | "cjs" => FileType::JavaScript,
            "ts" | "tsx" => FileType::TypeScript,
            "go" => FileType::Go,
            "java" => FileType::Java,
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hxx" | "h++" => FileType::Cpp,
            "c" | "h" => FileType::C,
            "cs" => FileType::CSharp,
            "rb" => FileType::Ruby,
            "php" => FileType::Php,
            "swift" => FileType::Swift,
            "kt" | "kts" => FileType::Kotlin,
            "scala" | "sc" => FileType::Scala,
            "hs" => FileType::Haskell,
            "dart" => FileType::Dart,
            "lua" => FileType::Lua,
            "r" => FileType::R,
            "jl" => FileType::Julia,
            "ex" | "exs" => FileType::Elixir,
            "elm" => FileType::Elm,
            "md" | "markdown" => FileType::Markdown,
            "json" => FileType::Json,
            "yaml" | "yml" => FileType::Yaml,
            "toml" => FileType::Toml,
            "xml" => FileType::Xml,
            "html" | "htm" => FileType::Html,
            "css" | "scss" | "sass" | "less" => FileType::Css,
            "txt" | "text" => FileType::Text,
            _ => {
                // Check if it's a text file by name
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");

                match filename {
                    "README" | "LICENSE" | "CHANGELOG" | "AUTHORS" | "CONTRIBUTORS" => {
                        FileType::Text
                    }
                    "Makefile" | "Dockerfile" | "Vagrantfile" | "Jenkinsfile" => FileType::Text,
                    _ if !is_binary_extension(path) => FileType::Text,
                    _ => FileType::Other,
                }
            }
        }
    }
}

/// Get the markdown code fence language for a file extension
pub fn get_language_from_extension(path: &Path) -> &'static str {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        // Programming languages
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "jsx" => "jsx",
        "go" => "go",
        "c" => "c",
        "cpp" | "cc" | "cxx" | "c++" => "cpp",
        "h" | "hpp" | "hxx" => "cpp",
        "cs" => "csharp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "r" => "r",
        "scala" => "scala",
        "clj" | "cljs" => "clojure",
        "ex" | "exs" => "elixir",
        "elm" => "elm",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "fs" | "fsx" => "fsharp",
        "pl" => "perl",
        "sh" => "bash",
        "fish" => "fish",
        "zsh" => "zsh",
        "ps1" => "powershell",
        "dart" => "dart",
        "julia" | "jl" => "julia",
        "nim" => "nim",
        "zig" => "zig",
        "v" => "v",
        "d" => "d",

        // Web technologies
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "vue" => "vue",
        "svelte" => "svelte",

        // Data formats
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "csv" => "csv",
        "sql" => "sql",

        // Markup languages
        "md" | "markdown" => "markdown",
        "tex" => "latex",
        "rst" => "rst",
        "adoc" | "asciidoc" => "asciidoc",

        // Configuration files
        "ini" | "cfg" => "ini",
        "conf" | "config" => "text",
        "env" => "dotenv",
        "dockerfile" => "dockerfile",
        "makefile" | "mk" => "makefile",

        // Shell scripts
        "bash" => "bash",
        "bat" | "cmd" => "batch",

        // Other
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "tf" => "hcl",
        "vim" => "vim",
        "diff" | "patch" => "diff",

        // Default to text for unknown extensions
        _ => "text",
    }
}

/// Check if a file is likely to be binary based on its extension
pub fn is_binary_extension(path: &Path) -> bool {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    matches!(
        extension.to_lowercase().as_str(),
        // Executables and libraries
        "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "bin" |
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" |
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "ico" | "svg" | "webp" |
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" |
        // Video
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" |
        // Documents
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" |
        // Database
        "db" | "sqlite" | "sqlite3" |
        // Other binary formats
        "pyc" | "pyo" | "class" | "o" | "obj" | "pdb"
    )
}

/// Detect if content contains binary data (null bytes)
pub fn is_binary_content(content: &[u8]) -> bool {
    // Check first 8KB for null bytes
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_language_detection() {
        assert_eq!(get_language_from_extension(Path::new("test.rs")), "rust");
        assert_eq!(get_language_from_extension(Path::new("test.py")), "python");
        assert_eq!(
            get_language_from_extension(Path::new("test.js")),
            "javascript"
        );
        assert_eq!(
            get_language_from_extension(Path::new("test.unknown")),
            "text"
        );
        assert_eq!(get_language_from_extension(Path::new("Makefile")), "text");
    }

    #[test]
    fn test_binary_extension_detection() {
        assert!(is_binary_extension(Path::new("test.exe")));
        assert!(is_binary_extension(Path::new("image.png")));
        assert!(is_binary_extension(Path::new("archive.zip")));
        assert!(!is_binary_extension(Path::new("code.rs")));
        assert!(!is_binary_extension(Path::new("text.md")));
    }

    #[test]
    fn test_binary_content_detection() {
        assert!(!is_binary_content(b"Hello, world!"));
        assert!(is_binary_content(b"Hello\0world"));
        assert!(is_binary_content(&[0xFF, 0xFE, 0x00, 0x00]));
    }
}

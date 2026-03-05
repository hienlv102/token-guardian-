use std::collections::HashMap;

/// Build the static dictionary for Rust code patterns.
pub fn rust_dict() -> HashMap<String, String> {
    let pairs = [
        ("println!", "p!"),
        ("eprintln!", "ep!"),
        ("format!", "fmt!"),
        ("Vec<String>", "VS"),
        ("Vec<u8>", "VU8"),
        ("HashMap<String, String>", "HSS"),
        ("HashMap<String, Value>", "HSV"),
        ("Option<String>", "OS"),
        ("Result<(), Error>", "RE"),
        ("Result<String, Error>", "RSE"),
        ("async fn", "afn"),
        ("pub async fn", "pafn"),
        ("pub fn", "pfn"),
        ("pub struct", "pst"),
        ("pub enum", "pen"),
        ("impl ", "im "),
        (".to_string()", ".ts()"),
        (".unwrap()", ".uw()"),
        (".clone()", ".cl()"),
        (".as_str()", ".as()"),
        ("&self", "&s"),
        ("&mut self", "&ms"),
        ("#[derive(", "#[d("),
        ("#[cfg(test)]", "#[t]"),
        ("use std::collections::", "use std::c::"),
        ("use serde::{Deserialize, Serialize}", "use serde::DS"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Build the static dictionary for JavaScript/TypeScript code patterns.
pub fn js_dict() -> HashMap<String, String> {
    let pairs = [
        ("console.log", "cl"),
        ("console.error", "ce"),
        ("function ", "fn "),
        ("const ", "c "),
        ("export default", "ed"),
        ("export const", "ec"),
        ("import ", "im "),
        (" from ", " f "),
        ("async function", "afn"),
        ("Promise<", "P<"),
        ("Array<", "A<"),
        ("interface ", "if "),
        ("undefined", "undef"),
        (".toString()", ".ts()"),
        (".forEach(", ".fe("),
        (".filter(", ".fi("),
        (".map(", ".m("),
        ("document.getElementById", "doc.gid"),
        ("addEventListener", "ael"),
        ("removeEventListener", "rel"),
        ("JSON.stringify", "J.s"),
        ("JSON.parse", "J.p"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Build the static dictionary for Python code patterns.
pub fn python_dict() -> HashMap<String, String> {
    let pairs = [
        ("print(", "pr("),
        ("def ", "d "),
        ("class ", "cl "),
        ("import ", "im "),
        ("from ", "fm "),
        ("self.", "s."),
        ("__init__", "__i__"),
        ("__str__", "__s__"),
        ("isinstance(", "isi("),
        ("enumerate(", "enum("),
        ("Exception", "Exc"),
        ("ValueError", "VErr"),
        ("TypeError", "TErr"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Build the static dictionary for common prompt/natural language phrases.
pub fn prompt_dict() -> HashMap<String, String> {
    let pairs = [
        ("Please implement", "Impl"),
        ("The function should", "Fn:"),
        ("Please fix the", "Fix:"),
        ("Can you help me", "Help:"),
        ("I need to", "Need:"),
        ("error message", "errmsg"),
        ("return value", "retval"),
        ("null pointer", "nullptr"),
        ("stack trace", "strace"),
        ("undefined behavior", "UB"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Build the static dictionary for Markdown documentation patterns.
pub fn markdown_dict() -> HashMap<String, String> {
    let pairs = [
        ("```bash\n", "«sh\n"),
        ("```json\n", "«js\n"),
        ("```typescript\n", "«ts\n"),
        ("```javascript\n", "«js\n"),
        ("```python\n", "«py\n"),
        ("```rust\n", "«rs\n"),
        ("```env\n", "«env\n"),
        ("```\n", "»\n"),
        ("## Getting Started", "## Start"),
        ("### Prerequisites", "### Prereq"),
        ("### Installation", "### Install"),
        ("#### Configuration", "#### Config"),
        ("#### Development", "#### Dev"),
        ("#### Database Setup", "#### DB Setup"),
        ("## Environment Variables", "## Env Vars"),
        ("## Project Structure", "## Structure"),
        ("**Framework**", "**FW**"),
        ("**Database**", "**DB**"),
        ("**Language**", "**Lang**"),
        ("**Build Tool**", "**Build**"),
        ("**State Management**", "**State**"),
        ("**Testing**", "**Test**"),
        ("**Styling**", "**Style**"),
        ("TypeScript", "TS"),
        ("JavaScript", "JS"),
        ("PostgreSQL", "PG"),
        ("# Start development server", "# Dev server"),
        ("# Build for production", "# Prod build"),
        ("# Run migrations", "# Migrate"),
        ("# Generate Prisma Client", "# Prisma gen"),
        ("# Push schema to database (alternative)", "# Push schema"),
        ("# Start production server", "# Prod start"),
        ("# Run all tests", "# Tests"),
        ("# Run tests in watch mode", "# Watch tests"),
        ("npm run start:dev", "npm dev"),
        ("npm install", "npm i"),
        ("(datetime)", "(dt)"),
        ("(string, unique)", "(str,uniq)"),
        ("(string, optional)", "(str,opt)"),
        ("(string)", "(str)"),
        ("(UUID, FK to User)", "(→User)"),
        ("(UUID, FK to Project)", "(→Project)"),
        ("(UUID)", "(uid)"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Get a combined dictionary for a given language.
pub fn get_dict(lang: Option<&str>) -> HashMap<String, String> {
    let mut dict = prompt_dict();
    match lang {
        Some("rust" | "rs") => dict.extend(rust_dict()),
        Some("javascript" | "typescript" | "js" | "ts") => dict.extend(js_dict()),
        Some("python" | "py") => dict.extend(python_dict()),
        Some("markdown" | "md") => dict.extend(markdown_dict()),
        _ => {
            // Include all if no language specified
            dict.extend(rust_dict());
            dict.extend(js_dict());
            dict.extend(python_dict());
            dict.extend(markdown_dict());
        }
    }
    dict
}

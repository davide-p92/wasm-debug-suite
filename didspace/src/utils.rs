
use colored::*;

pub fn highlight_wat(wat: &str) -> String {
    wat.lines()
        .map(|line| {
            let mut highlighted = line.to_string();
            // Highlight keywords
            for kw in ["module", "func", "param", "result", "local.get", "i32.add", "export"] {
                highlighted = highlighted.replace(kw, &kw.blue().bold().to_string());
            }
            // Highlight comments
            if highlighted.trim_start().starts_with(";;") {
                highlighted = highlighted.green().to_string();
            }
            highlighted
        })
        .collect::<Vec<_>>()
        .join("\n")
}

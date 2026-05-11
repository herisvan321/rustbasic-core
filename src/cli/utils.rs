pub fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i != 0 {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}
pub fn to_pascal_case(s: &str) -> String {
    let mut pascal = String::new();
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            pascal.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            pascal.push(ch);
        }
    }
    pascal
}

pub fn open_browser(url: &str) {
    let _ = match std::env::consts::OS {
        "macos" => std::process::Command::new("open").arg(url).spawn(),
        "windows" => std::process::Command::new("cmd").args(["/C", "start", url]).spawn(),
        _ => std::process::Command::new("xdg-open").arg(url).spawn(),
    };
}

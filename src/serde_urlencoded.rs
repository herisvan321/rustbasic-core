
/// Decodes a URL-encoded string into any type that implements serde::de::DeserializeOwned.
pub fn from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    let mut map = serde_json::Map::new();
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");

        let decoded_key = url_decode(key)?;
        let decoded_val = url_decode(val)?;

        map.insert(decoded_key, serde_json::Value::String(decoded_val));
    }
    serde_json::from_value(serde_json::Value::Object(map))
        .map_err(|e| e.to_string())
}

/// Decodes a URL-encoded byte array into any type that implements serde::de::DeserializeOwned.
pub fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    from_str(s)
}

fn url_decode(s: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '+' {
            decoded.push(' ');
        } else if c == '%' {
            let h0 = chars.next().ok_or("Format URL-encoded tidak valid")?;
            let h1 = chars.next().ok_or("Format URL-encoded tidak valid")?;
            let hex_str = format!("{}{}", h0, h1);
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|e| e.to_string())?;
            decoded.push(byte as char);
        } else {
            decoded.push(c);
        }
    }
    Ok(decoded)
}

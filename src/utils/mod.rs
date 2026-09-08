//! 工具层喵~ 日志、路径等小工具都住这里喵。

pub mod logger;

/// 按 RFC 3986 对 URL 组件做百分号编码喵
///
/// 保留字符 `A-Za-z0-9 - _ . ~`,其余(含中文)按 UTF-8 逐字节转 `%XX` 喵。
pub fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

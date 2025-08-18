pub fn get_translation<'a>(
    key: &'a str,
    translations: &'a std::collections::HashMap<String, String>,
) -> &'a str {
    let clean_key = key.strip_prefix('@').unwrap_or(key);
    translations
        .get(clean_key)
        .map(String::as_str)
        .unwrap_or(key)
}

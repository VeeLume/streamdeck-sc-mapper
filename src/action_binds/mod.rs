pub mod activation_mode;
pub mod bind;
pub mod binds;
pub mod action_binding;
pub mod action_map;
pub mod action_bindings;
pub mod constants;
pub mod bind_generator;
pub mod generate_mappings_xml;

fn get_translation<'a>(
    key: &'a str,
    translations: &'a std::collections::HashMap<String, String>
) -> &'a str {
    let clean_key = key.strip_prefix('@').unwrap_or(key);
    translations.get(clean_key).map(String::as_str).unwrap_or(key)
}

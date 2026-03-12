rust_i18n::i18n!("locales", fallback = "en");

/// Get a translated rule description by rule code (e.g. "LT01").
/// Falls back to the provided default if no translation exists.
pub fn rule_description(code: &str, default: &str) -> String {
    let key = format!("rules.{code}.description");
    let translated = rust_i18n::t!(&key);
    // rust-i18n returns the key itself when no translation is found
    if translated == key {
        default.to_string()
    } else {
        translated.to_string()
    }
}

/// Get a translated rule explanation by rule code (e.g. "LT01").
/// Falls back to the provided default if no translation exists.
pub fn rule_explanation(code: &str, default: &str) -> String {
    let key = format!("rules.{code}.explanation");
    let translated = rust_i18n::t!(&key);
    if translated == key {
        default.to_string()
    } else {
        translated.to_string()
    }
}

/// Set the global locale. Use "en", "ja", etc.
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// Get the current global locale.
pub fn get_locale() -> String {
    rust_i18n::locale().to_string()
}

/// Translate a CLI message key with no parameters.
pub fn t(key: &str) -> String {
    let translated = rust_i18n::t!(key);
    translated.to_string()
}

/// Translate a violation message using its key and parameters.
/// Returns the translated message with parameters substituted.
/// Falls back to `fallback` if no translation is found or key is empty.
pub fn rule_message(key: &str, params: &[(String, String)], fallback: &str) -> String {
    if key.is_empty() {
        return fallback.to_string();
    }
    let translated = rust_i18n::t!(key);
    let translated_str = translated.to_string();
    // rust-i18n returns the key itself when no translation is found
    if translated_str == key {
        return fallback.to_string();
    }
    // Substitute parameters: %{name} → value
    // Parameter values themselves may have translations (e.g. "upper" → "大文字")
    let mut result = translated_str;
    for (k, v) in params {
        let translated_v = translate_param_value(v);
        result = result.replace(&format!("%{{{k}}}"), &translated_v);
    }
    result
}

/// Try to translate a parameter value using the "params.*" namespace.
/// Falls back to the original value if no translation exists.
fn translate_param_value(value: &str) -> String {
    let key = format!("params.{value}");
    let translated = rust_i18n::t!(&key);
    let translated_str = translated.to_string();
    if translated_str == key {
        value.to_string()
    } else {
        translated_str
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // rust-i18n uses a global locale, so tests that change locale must run
    // sequentially in a single test to avoid races.
    #[test]
    fn test_locale_switching() {
        // English
        set_locale("en");
        assert_eq!(
            rule_description("LT01", "fallback"),
            "Inappropriate spacing found."
        );
        assert_eq!(t("cli.no_sql_files"), "No SQL files found.");

        // Japanese
        set_locale("ja");
        assert_eq!(
            rule_description("LT01", "fallback"),
            "不適切なスペースが見つかりました。"
        );
        assert_eq!(t("cli.no_sql_files"), "SQLファイルが見つかりません。");

        // Fallback for unknown rule
        set_locale("en");
        assert_eq!(rule_description("XX99", "my fallback"), "my fallback");

        // Reset
        set_locale("en");
    }
}

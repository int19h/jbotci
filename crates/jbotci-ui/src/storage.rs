use super::*;

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn current_query() -> String {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_empty())]
pub(super) fn current_query() -> String {
    String::new()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn query_param(query: &str, name: &str) -> Option<String> {
    let trimmed = query.strip_prefix('?').unwrap_or(query);
    trimmed
        .split('&')
        .filter(|part| !part.is_empty())
        .find_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode_query_component(key) == name)
                .then(|| percent_decode_query_component(value))
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn percent_decode_query_component(input: &str) -> String {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            output.push(b' ');
            index += 1;
        } else if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&input[index + 1..index + 3], 16) {
                output.push(value);
                index += 3;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn load_settings() -> UserSettings {
    let mut settings = UserSettings::default();
    if let Some(theme) = storage_get("jbotci.theme").and_then(|value| parse_theme(&value)) {
        settings.theme = theme;
    }
    if let Some(script) = storage_get("jbotci.script").and_then(|value| parse_script(&value)) {
        settings.script = script;
    }
    if let Some(stress) =
        storage_get("jbotci.output.stress").and_then(|value| parse_stress_mark(&value))
    {
        settings.stress = stress;
    }
    if let Some(glides) =
        storage_get("jbotci.output.glides").and_then(|value| parse_glide_mark(&value))
    {
        settings.glides = glides;
    }
    if let Some(depth) = storage_get("jbotci.parsing.error-context-depth")
        .and_then(|value| parse_error_context_depth(&value))
    {
        settings.error_context_depth = depth;
    }
    settings
}

#[requires(true)]
#[ensures(true)]
pub(super) fn load_dialect_settings() -> DialectSettings {
    storage_get(DIALECT_SETTINGS_STORAGE_KEY)
        .and_then(|raw| serde_json::from_str::<DialectSettings>(&raw).ok())
        .map(normalize_loaded_dialect_settings)
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn normalize_loaded_dialect_settings(mut settings: DialectSettings) -> DialectSettings {
    settings
        .custom_dialects
        .retain(|custom| !custom.name.trim().is_empty());
    settings
}

#[requires(true)]
#[ensures(true)]
pub(super) fn save_dialect_settings(settings: &DialectSettings) {
    if let Ok(raw) = serde_json::to_string(settings) {
        storage_set(DIALECT_SETTINGS_STORAGE_KEY, &raw);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_theme(value: &str) -> Option<ThemeMode> {
    match value {
        "auto" | "system" => Some(ThemeMode::Auto),
        "day" | "light" => Some(ThemeMode::Day),
        "night" | "dark" => Some(ThemeMode::Night),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_stress_mark(value: &str) -> Option<StressMark> {
    match value {
        "none" => Some(StressMark::None),
        "acute" => Some(StressMark::Acute),
        "caps" => Some(StressMark::Caps),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_glide_mark(value: &str) -> Option<GlideMark> {
    match value {
        "none" => Some(GlideMark::None),
        "breve" => Some(GlideMark::Breve),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_error_context_depth(value: &str) -> Option<usize> {
    value.trim().parse().ok()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn stress_mark_storage_value(mark: StressMark) -> &'static str {
    match mark {
        StressMark::None => "none",
        StressMark::Acute => "acute",
        StressMark::Caps => "caps",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn glide_mark_storage_value(mark: GlideMark) -> &'static str {
    match mark {
        GlideMark::None => "none",
        GlideMark::Breve => "breve",
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn save_settings(settings: &UserSettings) {
    storage_set("jbotci.theme", theme_class(settings.theme));
    storage_set("jbotci.script", script_class(settings.script));
    storage_set(
        "jbotci.output.stress",
        stress_mark_storage_value(settings.stress),
    );
    storage_set(
        "jbotci.output.glides",
        glide_mark_storage_value(settings.glides),
    );
    storage_set(
        "jbotci.parsing.error-context-depth",
        &settings.error_context_depth.to_string(),
    );
}

#[requires(true)]
#[ensures(true)]
pub(super) fn load_vlacku_jvozba_pane_state() -> VlackuJvozbaPaneState {
    let open = matches!(
        storage_get("jbotci.vlacku.jvozba.open.v1").as_deref(),
        Some("1" | "true")
    );
    let mode = storage_get("jbotci.vlacku.jvozba.mode.v1")
        .as_deref()
        .and_then(parse_vlacku_jvozba_mode)
        .unwrap_or(VlackuJvozbaMode::Lujvo);
    let items = storage_get("jbotci.vlacku.jvozba.items.v1")
        .map(|raw| parse_vlacku_jvozba_items(&raw))
        .unwrap_or_default();
    VlackuJvozbaPaneState { open, mode, items }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn save_vlacku_jvozba_pane_state(state: &VlackuJvozbaPaneState) {
    storage_set(
        "jbotci.vlacku.jvozba.open.v1",
        if state.open { "1" } else { "0" },
    );
    storage_set(
        "jbotci.vlacku.jvozba.mode.v1",
        match state.mode {
            VlackuJvozbaMode::Lujvo => "lujvo",
            VlackuJvozbaMode::Cmevla => "cmevla",
        },
    );
    storage_set(
        "jbotci.vlacku.jvozba.items.v1",
        &format_vlacku_jvozba_items(&state.items),
    );
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_vlacku_jvozba_mode(value: &str) -> Option<VlackuJvozbaMode> {
    match value {
        "lujvo" => Some(VlackuJvozbaMode::Lujvo),
        "cmevla" => Some(VlackuJvozbaMode::Cmevla),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_vlacku_jvozba_items(raw: &str) -> Vec<VlackuJvozbaItem> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(items) = value.as_array() {
            return items
                .iter()
                .filter_map(parse_vlacku_jvozba_json_item)
                .collect();
        }
    }
    parse_vlacku_jvozba_legacy_items(raw)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_vlacku_jvozba_json_item(value: &serde_json::Value) -> Option<VlackuJvozbaItem> {
    let object = value.as_object()?;
    let kind_text = object.get("kind")?.as_str()?;
    let item_kind = match kind_text {
        "word" => VlackuJvozbaItemKind::Word,
        "rafsi" | "fixed-rafsi" => VlackuJvozbaItemKind::FixedRafsi,
        _ => return None,
    };
    let item_value = object.get("value")?.as_str()?.trim();
    if item_value.is_empty() {
        return None;
    }
    let source = object
        .get("source")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let indent_level = object
        .get("indentLevel")
        .or_else(|| object.get("indent_level"))
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0);
    Some(VlackuJvozbaItem {
        kind: item_kind,
        value: item_value.to_owned(),
        source,
        indent_level,
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_vlacku_jvozba_legacy_items(raw: &str) -> Vec<VlackuJvozbaItem> {
    raw.lines()
        .filter_map(|line| {
            let (kind, value) = line.split_once('\t')?;
            let item_kind = match kind {
                "word" => VlackuJvozbaItemKind::Word,
                "rafsi" => VlackuJvozbaItemKind::FixedRafsi,
                _ => return None,
            };
            (!value.is_empty()).then(|| VlackuJvozbaItem {
                kind: item_kind,
                value: value.to_owned(),
                source: None,
                indent_level: 0,
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn format_vlacku_jvozba_items(items: &[VlackuJvozbaItem]) -> String {
    let values = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "kind": match item.kind {
                    VlackuJvozbaItemKind::Word => "word",
                    VlackuJvozbaItemKind::FixedRafsi => "rafsi",
                },
                "value": item.value.as_str(),
                "source": item.source.as_deref(),
                "indentLevel": item.indent_level,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_owned())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_script(value: &str) -> Option<GentufaScript> {
    match value {
        "latin" => Some(GentufaScript::Latin),
        "cyrillic" => Some(GentufaScript::Cyrillic),
        "zbalermorna" => Some(GentufaScript::Zbalermorna),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn storage_get(key: &str) -> Option<String> {
    native_storage_get(key)
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn storage_set(key: &str, value: &str) {
    let _ = native_storage_set(key, value);
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn session_storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.session_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn session_storage_get(key: &str) -> Option<String> {
    native_session_storage()
        .lock()
        .ok()
        .and_then(|values| values.get(key).cloned())
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn session_storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.session_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn session_storage_set(key: &str, value: &str) {
    if let Ok(mut values) = native_session_storage().lock() {
        values.insert(key.to_owned(), value.to_owned());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) static NATIVE_SESSION_STORAGE: OnceLock<
    Mutex<std::collections::HashMap<String, String>>,
> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn native_session_storage() -> &'static Mutex<std::collections::HashMap<String, String>>
{
    NATIVE_SESSION_STORAGE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn native_storage_get(key: &str) -> Option<String> {
    native_storage_values().ok()?.get(key).cloned()
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn native_storage_set(key: &str, value: &str) -> Result<(), String> {
    let mut values = native_storage_values()?;
    values.insert(key.to_owned(), value.to_owned());
    write_native_storage_values(&values)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|values| values.keys().all(|key| !key.is_empty())) || ret.is_err())]
pub(super) fn native_storage_values() -> Result<std::collections::BTreeMap<String, String>, String>
{
    let path = native_storage_path()?;
    if !path.is_file() {
        return Ok(std::collections::BTreeMap::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "failed to read native settings `{}`: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse native settings `{}`: {error}",
            path.display()
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn write_native_storage_values(
    values: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let path = native_storage_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create native settings directory `{}`: {error}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(values)
        .map_err(|error| format!("failed to serialize native settings: {error}"))?;
    std::fs::write(&path, raw).map_err(|error| {
        format!(
            "failed to write native settings `{}`: {error}",
            path.display()
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.ends_with("ui-settings.json")) || ret.is_err())]
pub(super) fn native_storage_path() -> Result<std::path::PathBuf, String> {
    let dirs = directories::ProjectDirs::from("org", "int19h", "jbotci")
        .ok_or_else(|| "could not resolve native settings directory".to_owned())?;
    Ok(dirs.config_dir().join("ui-settings.json"))
}

//! KWin window rules management for Wayland/KDE Plasma
//!
//! On Wayland, applications cannot control their own window position, always-on-top state,
//! or focus behavior. KWin window rules are the proper solution for KDE Plasma.
//!
//! This module provides detection and management of KWin rules for the VoKey HUD window.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Unique identifier for our KWin rule
const RULE_ID: &str = "vokey-hud-rule";

/// The wmclass to match (must match the app's window class)
const WMCLASS: &str = "vokey-transcribe";

/// Status of the KWin rules environment
#[derive(Clone, serde::Serialize)]
pub struct KwinStatus {
    /// Whether running on Wayland (vs X11)
    pub is_wayland: bool,
    /// Whether running on KDE desktop
    pub is_kde: bool,
    /// Whether KWin rules are applicable (Wayland + KDE)
    pub rules_applicable: bool,
    /// Whether the VoKey rule is currently installed
    pub rule_installed: bool,
    /// Path to kwinrulesrc (for debugging)
    pub config_path: Option<String>,
    /// Any error message
    pub error: Option<String>,
}

/// Check if running on Wayland
fn is_wayland() -> bool {
    // Check XDG_SESSION_TYPE first (most reliable)
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        if session_type.to_lowercase() == "wayland" {
            return true;
        }
    }
    // Fallback: check if WAYLAND_DISPLAY is set
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Check if running on KDE
fn is_kde() -> bool {
    // Check XDG_CURRENT_DESKTOP
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        if desktop.to_uppercase().contains("KDE") {
            return true;
        }
    }
    // Fallback: check KDE_SESSION_VERSION
    std::env::var("KDE_SESSION_VERSION").is_ok()
}

/// Get the path to kwinrulesrc
fn kwinrulesrc_path() -> Option<PathBuf> {
    // Standard location: ~/.config/kwinrulesrc
    dirs::config_dir().map(|d| d.join("kwinrulesrc"))
}

/// Simple INI parser for kwinrulesrc
/// Returns a map of section name -> map of key -> value
fn parse_kwinrulesrc(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            sections.entry(current_section.clone()).or_default();
        } else if let Some((key, value)) = line.split_once('=') {
            if !current_section.is_empty() {
                sections
                    .entry(current_section.clone())
                    .or_default()
                    .insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    sections
}

/// Check if our rule is installed in kwinrulesrc
fn check_rule_installed(path: &PathBuf) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist yet - expected on first run
            return false;
        }
        Err(e) => {
            // Log unexpected errors (permission denied, I/O errors, etc.)
            log::warn!(
                "Failed to read kwinrulesrc at {:?}: {} (kind: {:?})",
                path,
                e,
                e.kind()
            );
            return false;
        }
    };

    let sections = parse_kwinrulesrc(&content);

    // Check if General section has our rule ID in the rules list
    if let Some(general) = sections.get("General") {
        if let Some(rules) = general.get("rules") {
            if rules.contains(RULE_ID) {
                // Also verify the rule section exists and has correct wmclass
                if let Some(rule_section) = sections.get(RULE_ID) {
                    if let Some(wmclass) = rule_section.get("wmclass") {
                        return wmclass == WMCLASS;
                    }
                }
            }
        }
    }

    false
}

/// Install our KWin rule
fn install_rule(path: &PathBuf) -> Result<(), String> {
    // Read existing content or start fresh
    // Distinguish between "file not found" (safe to create new) and other errors
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("kwinrulesrc does not exist, will create new file");
            String::new()
        }
        Err(e) => {
            return Err(format!(
                "Cannot read existing kwinrulesrc at {:?}: {}. Will not overwrite to avoid data loss.",
                path, e
            ));
        }
    };
    let mut sections = parse_kwinrulesrc(&content);

    // Update General section
    let general = sections.entry("General".to_string()).or_default();

    // Get current rules list
    let mut rules_list: Vec<String> = general
        .get("rules")
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Add our rule if not present
    if !rules_list.contains(&RULE_ID.to_string()) {
        rules_list.push(RULE_ID.to_string());
    }

    // Update count and rules
    general.insert("count".to_string(), rules_list.len().to_string());
    general.insert("rules".to_string(), rules_list.join(","));

    // Add our rule section
    let mut rule_section: HashMap<String, String> = HashMap::new();
    rule_section.insert(
        "Description".to_string(),
        "VoKey HUD - Always on top, top-left, no focus".to_string(),
    );
    rule_section.insert("above".to_string(), "true".to_string());
    rule_section.insert("aboverule".to_string(), "2".to_string());
    rule_section.insert("acceptfocus".to_string(), "false".to_string());
    rule_section.insert("acceptfocusrule".to_string(), "2".to_string());
    rule_section.insert("position".to_string(), "20,20".to_string());
    rule_section.insert("positionrule".to_string(), "2".to_string());
    rule_section.insert("wmclass".to_string(), WMCLASS.to_string());
    rule_section.insert("wmclassmatch".to_string(), "1".to_string());

    sections.insert(RULE_ID.to_string(), rule_section);

    // Write back to file
    let new_content = serialize_kwinrulesrc(&sections);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    fs::write(path, new_content).map_err(|e| format!("Failed to write kwinrulesrc: {}", e))?;

    Ok(())
}

/// Remove our KWin rule
fn remove_rule(path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read kwinrulesrc: {}", e))?;

    let mut sections = parse_kwinrulesrc(&content);

    // Remove our rule section
    sections.remove(RULE_ID);

    // Update General section
    if let Some(general) = sections.get_mut("General") {
        // Remove our rule from the rules list
        if let Some(rules) = general.get("rules") {
            let rules_list: Vec<String> = rules
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|r| r != RULE_ID)
                .collect();

            general.insert("count".to_string(), rules_list.len().to_string());
            general.insert("rules".to_string(), rules_list.join(","));
        }
    }

    // Write back to file
    let new_content = serialize_kwinrulesrc(&sections);
    fs::write(path, new_content).map_err(|e| format!("Failed to write kwinrulesrc: {}", e))?;

    Ok(())
}

/// Serialize sections back to INI format
fn serialize_kwinrulesrc(sections: &HashMap<String, HashMap<String, String>>) -> String {
    let mut output = String::new();

    // Write General section first
    if let Some(general) = sections.get("General") {
        output.push_str("[General]\n");
        // Write count and rules first for consistency
        if let Some(count) = general.get("count") {
            output.push_str(&format!("count={}\n", count));
        }
        if let Some(rules) = general.get("rules") {
            output.push_str(&format!("rules={}\n", rules));
        }
        // Write any other keys
        for (key, value) in general {
            if key != "count" && key != "rules" {
                output.push_str(&format!("{}={}\n", key, value));
            }
        }
        output.push('\n');
    }

    // Write other sections
    for (section_name, section_data) in sections {
        if section_name == "General" {
            continue;
        }
        output.push_str(&format!("[{}]\n", section_name));
        // Write Description first if present
        if let Some(desc) = section_data.get("Description") {
            output.push_str(&format!("Description={}\n", desc));
        }
        // Write remaining keys alphabetically for consistency
        let mut keys: Vec<_> = section_data.keys().filter(|k| *k != "Description").collect();
        keys.sort();
        for key in keys {
            output.push_str(&format!("{}={}\n", key, section_data[key]));
        }
        output.push('\n');
    }

    output
}

/// Reload KWin configuration via D-Bus
fn reload_kwin() -> Result<(), String> {
    // Try qdbus6 first (Plasma 6), then fall back to qdbus (Plasma 5)
    let result = Command::new("qdbus6")
        .args(["org.kde.KWin", "/KWin", "reconfigure"])
        .output();

    let qdbus6_error: Option<String> = match result {
        Ok(output) if output.status.success() => {
            log::info!("KWin reconfigured via qdbus6");
            return Ok(());
        }
        Ok(output) => {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            log::debug!(
                "qdbus6 failed (exit {}): {}, trying qdbus...",
                output.status.code().unwrap_or(-1),
                err
            );
            Some(err)
        }
        Err(e) => {
            log::debug!("qdbus6 not available: {}, trying qdbus...", e);
            Some(e.to_string())
        }
    };

    // Try qdbus as fallback
    let result = Command::new("qdbus")
        .args(["org.kde.KWin", "/KWin", "reconfigure"])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            log::info!("KWin reconfigured via qdbus");
            Ok(())
        }
        Ok(output) => {
            let qdbus_err = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Failed to reconfigure KWin. qdbus6: {:?}, qdbus: {}",
                qdbus6_error, qdbus_err
            ))
        }
        Err(e) => Err(format!(
            "Failed to run D-Bus tools. qdbus6: {:?}, qdbus: {}",
            qdbus6_error, e
        )),
    }
}

/// Get the current KWin status
pub fn get_status() -> KwinStatus {
    let is_wayland = is_wayland();
    let is_kde = is_kde();
    let rules_applicable = is_wayland && is_kde;

    let config_path = kwinrulesrc_path();
    let rule_installed = config_path
        .as_ref()
        .map(|p| check_rule_installed(p))
        .unwrap_or(false);

    KwinStatus {
        is_wayland,
        is_kde,
        rules_applicable,
        rule_installed,
        config_path: config_path.map(|p| p.to_string_lossy().to_string()),
        error: None,
    }
}

/// Install the KWin rule and reload KWin
pub fn install_kwin_rule() -> Result<(), String> {
    let path = kwinrulesrc_path().ok_or("Could not determine config directory")?;

    install_rule(&path)?;
    log::info!("KWin rule installed to {:?}", path);

    reload_kwin()?;

    Ok(())
}

/// Remove the KWin rule and reload KWin
pub fn remove_kwin_rule() -> Result<(), String> {
    let path = kwinrulesrc_path().ok_or("Could not determine config directory")?;

    if !path.exists() {
        return Err("kwinrulesrc does not exist".to_string());
    }

    remove_rule(&path)?;
    log::info!("KWin rule removed from {:?}", path);

    reload_kwin()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kwinrulesrc() {
        let content = r#"
[General]
count=2
rules=rule1,rule2

[rule1]
Description=Test Rule 1
wmclass=test-app
wmclassmatch=1

[rule2]
Description=Test Rule 2
above=true
aboverule=2
"#;

        let sections = parse_kwinrulesrc(content);

        assert!(sections.contains_key("General"));
        assert!(sections.contains_key("rule1"));
        assert!(sections.contains_key("rule2"));

        let general = sections.get("General").unwrap();
        assert_eq!(general.get("count"), Some(&"2".to_string()));
        assert_eq!(general.get("rules"), Some(&"rule1,rule2".to_string()));

        let rule1 = sections.get("rule1").unwrap();
        assert_eq!(rule1.get("Description"), Some(&"Test Rule 1".to_string()));
        assert_eq!(rule1.get("wmclass"), Some(&"test-app".to_string()));
    }

    #[test]
    fn test_serialize_kwinrulesrc() {
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();

        let mut general: HashMap<String, String> = HashMap::new();
        general.insert("count".to_string(), "1".to_string());
        general.insert("rules".to_string(), "test-rule".to_string());
        sections.insert("General".to_string(), general);

        let mut rule: HashMap<String, String> = HashMap::new();
        rule.insert("Description".to_string(), "Test".to_string());
        rule.insert("wmclass".to_string(), "test".to_string());
        sections.insert("test-rule".to_string(), rule);

        let output = serialize_kwinrulesrc(&sections);

        assert!(output.contains("[General]"));
        assert!(output.contains("count=1"));
        assert!(output.contains("rules=test-rule"));
        assert!(output.contains("[test-rule]"));
        assert!(output.contains("Description=Test"));
    }
}

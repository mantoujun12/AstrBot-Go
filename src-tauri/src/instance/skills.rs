//! Offline Skills management for AstrBot instances.
//!
//! Mirrors upstream AstrBot: local skills live in
//! `data/skills/<name>/SKILL.md` (a legacy lowercase `skill.md` is read the
//! same way), the enabled state is `data/skills.json`
//! (`{"skills": {"<name>": {"active": false}}}`, missing entry = active), and
//! uploads are ZIP archives either holding `SKILL.md` at the root (the skill
//! name comes from the ZIP file name) or one folder per skill. Plugin-provided
//! skills stay under their plugin's `skills/` directory and are managed with
//! the plugin, so they are not listed here. All edits hold the instance
//! lifecycle guard, matching the offline data management convention.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::archive::{extract_zip_mapped, validate_zip_archive};
use crate::error::{AppError, Result};
use crate::instance::plugins::validate_plugin_dir_name;
use crate::utils::archive_path::parse_entry_rel_path;
use crate::utils::fs_ops::replace_file_atomically;

const SKILLS_CONFIG_FILE: &str = "skills.json";
const STAGING_PREFIX: &str = ".skill-install-";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSkill {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub active: bool,
}

/// A read-only skill provided by a plugin under `data/plugins/<dir>/skills/`.
/// Upstream also treats a plugin's own `skills/SKILL.md` as a skill named
/// after the plugin directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePluginSkill {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub plugin_name: String,
}

/// Minimal `skills.json` shape written by upstream.
#[derive(Debug, Default, Clone)]
struct SkillsConfig {
    skills: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillZipInstall {
    /// Path of the picked ZIP archive.
    pub zip_path: String,
    /// ZIP file name without the extension; used for root-mode archives.
    #[serde(default)]
    pub name_hint: Option<String>,
}

/// Upstream accepts `[\w.-]+` skill names; the launcher additionally rejects
/// leading dots so traversal names (`..`) and staging dirs stay out.
fn skill_name_is_valid(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_alphanumeric() || first == '_' => (),
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Upstream replaces whitespace with underscores to derive a skill name.
fn normalize_skill_name(name: &str) -> String {
    name.trim().replace([' ', '\t'], "_")
}

fn validate_skill_name(name: &str) -> Result<()> {
    if skill_name_is_valid(name) {
        Ok(())
    } else {
        Err(AppError::io(format!(
            "技能名称不合法：{name:?}（仅允许字母、数字、下划线、点和中划线）"
        )))
    }
}

fn skills_root(data_dir: &Path) -> PathBuf {
    data_dir.join("skills")
}

fn skills_config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SKILLS_CONFIG_FILE)
}

fn skill_markdown(skill_dir: &Path) -> Option<PathBuf> {
    let canonical = skill_dir.join("SKILL.md");
    if canonical.is_file() {
        return Some(canonical);
    }
    let legacy = skill_dir.join("skill.md");
    legacy.is_file().then_some(legacy)
}

fn load_skills_config(data_dir: &Path) -> SkillsConfig {
    let Ok(raw) = fs::read_to_string(skills_config_path(data_dir)) else {
        return SkillsConfig::default();
    };
    let Ok(parsed) = serde_json::from_str::<JsonValue>(&raw) else {
        log::warn!("Ignoring malformed {}", SKILLS_CONFIG_FILE);
        return SkillsConfig::default();
    };
    let mut config = SkillsConfig::default();
    if let Some(entries) = parsed.get("skills").and_then(JsonValue::as_object) {
        for (name, value) in entries {
            let active = value.get("active").and_then(JsonValue::as_bool);
            if let Some(active) = active {
                config.skills.insert(name.clone(), active);
            }
        }
    }
    config
}

fn save_skills_config(data_dir: &Path, config: &SkillsConfig) -> Result<()> {
    let skills: BTreeMap<&str, JsonValue> = config
        .skills
        .iter()
        .map(|(name, active)| (name.as_str(), json!({ "active": active })))
        .collect();
    let raw = serde_json::to_string_pretty(&json!({ "skills": skills }))
        .map_err(|e| AppError::io(format!("序列化技能配置失败：{e}")))?;
    let dest = skills_config_path(data_dir);
    let temp = dest.with_extension("json.tmp-skill");
    fs::write(&temp, raw).map_err(|e| AppError::io(format!("写入技能配置失败：{e}")))?;
    replace_file_atomically(&temp, &dest)
}

/// Extract the `description` string from the SKILL.md YAML frontmatter.
fn parse_frontmatter_description(content: &str) -> String {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return String::new();
    }
    let mut frontmatter = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            let joined = frontmatter.join("\n");
            let Ok(parsed) = serde_yaml_ng::from_str::<JsonValue>(&joined) else {
                return String::new();
            };
            return parsed
                .get("description")
                .and_then(JsonValue::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .to_string();
        }
        frontmatter.push(line);
    }
    String::new()
}

/// List the local skills of an instance. Unknown entries in `skills.json`
/// default to enabled, matching upstream.
pub fn list_skills(data_dir: &Path) -> Vec<InstanceSkill> {
    let root = skills_root(data_dir);
    let mut skills = Vec::new();
    let Ok(entries) = fs::read_dir(&root) else {
        return skills;
    };
    let config = load_skills_config(data_dir);

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !skill_name_is_valid(&name) || !path.is_dir() {
            continue;
        }
        let Some(markdown) = skill_markdown(&path) else {
            continue;
        };
        let content = fs::read_to_string(&markdown).unwrap_or_default();
        let active = config.skills.get(&name).copied().unwrap_or(true);
        skills.push(InstanceSkill {
            name,
            description: parse_frontmatter_description(&content),
            active,
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn find_skill_dir(data_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_skill_name(name)?;
    let dir = skills_root(data_dir).join(name);
    if !dir.is_dir() {
        return Err(AppError::io(format!("技能 {name} 不存在。")));
    }
    Ok(dir)
}

pub fn read_skill(data_dir: &Path, name: &str) -> Result<String> {
    let dir = find_skill_dir(data_dir, name)?;
    let markdown =
        skill_markdown(&dir).ok_or_else(|| AppError::io(format!("技能 {name} 缺少 SKILL.md。")))?;
    fs::read_to_string(&markdown).map_err(|e| AppError::io(format!("读取技能失败：{e}")))
}

pub fn write_skill(data_dir: &Path, name: &str, content: &str) -> Result<()> {
    let dir = find_skill_dir(data_dir, name)?;
    let markdown =
        skill_markdown(&dir).ok_or_else(|| AppError::io(format!("技能 {name} 缺少 SKILL.md。")))?;
    fs::write(&markdown, content).map_err(|e| AppError::io(format!("写入技能失败：{e}")))
}

pub fn set_skill_active(data_dir: &Path, name: &str, active: bool) -> Result<()> {
    find_skill_dir(data_dir, name)?;
    let mut config = load_skills_config(data_dir);
    config.skills.insert(name.to_string(), active);
    save_skills_config(data_dir, &config)
}

pub fn delete_skill(data_dir: &Path, name: &str) -> Result<()> {
    let dir = find_skill_dir(data_dir, name)?;
    fs::remove_dir_all(&dir).map_err(|e| AppError::io(format!("删除技能失败：{e}")))?;
    let mut config = load_skills_config(data_dir);
    if config.skills.remove(name).is_some() {
        save_skills_config(data_dir, &config)?;
    }
    Ok(())
}

/// Install skills from a ZIP archive, mirroring upstream's two layouts:
/// `SKILL.md` at the archive root (name from the ZIP file name) or one
/// folder per skill. Existing skills are never overwritten.
pub fn install_skill_zip(data_dir: &Path, request: &SkillZipInstall) -> Result<Vec<String>> {
    let zip_path = Path::new(&request.zip_path);
    validate_zip_archive(zip_path)?;

    let staging = skills_root(data_dir).join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| AppError::io(format!("创建暂存目录失败：{e}")))?;
    let result = install_zip_staged(data_dir, zip_path, &staging, request.name_hint.as_deref());
    let _ = fs::remove_dir_all(&staging);
    result
}

fn install_zip_staged(
    data_dir: &Path,
    zip_path: &Path,
    staging: &Path,
    name_hint: Option<&str>,
) -> Result<Vec<String>> {
    let extracted = staging.join("extracted");
    extract_zip_mapped(zip_path, &extracted, |raw| {
        let relative = parse_entry_rel_path(raw)?;
        // macOS archives carry junk metadata at this path.
        if relative.components().next()?.as_os_str() == "__MACOSX" {
            return None;
        }
        Some(extracted.join(relative))
    })?;

    let mut installed: Vec<String> = Vec::new();
    if skill_markdown(&extracted).is_some() {
        let name = normalize_skill_name(
            name_hint
                .map(str::trim)
                .filter(|hint| !hint.is_empty())
                .unwrap_or("skill"),
        );
        validate_skill_name(&name)?;
        install_skill_dir(data_dir, &extracted, &name)?;
        installed.push(name);
    } else {
        let entries =
            fs::read_dir(&extracted).map_err(|e| AppError::io(format!("读取解压结果失败：{e}")))?;
        for entry in entries.flatten() {
            let dir_name = entry.file_name().to_string_lossy().into_owned();
            let dir = entry.path();
            if !dir.is_dir() || skill_markdown(&dir).is_none() {
                continue;
            }
            let name = normalize_skill_name(&dir_name);
            if !skill_name_is_valid(&name) {
                continue;
            }
            install_skill_dir(data_dir, &dir, &name)?;
            installed.push(name);
        }
    }

    if installed.is_empty() {
        return Err(AppError::io(
            "压缩包内没有包含 SKILL.md 的技能目录，无法安装。",
        ));
    }

    let mut config = load_skills_config(data_dir);
    for name in &installed {
        config.skills.insert(name.clone(), true);
    }
    save_skills_config(data_dir, &config)?;

    Ok(installed)
}

// === Plugin-provided skills (read-only) ======================================

/// List the read-only skills every installed plugin carries in its own
/// `skills/` directory, mirroring upstream's discovery rules.
pub fn list_plugin_skills(data_dir: &Path) -> Vec<InstancePluginSkill> {
    let mut skills = Vec::new();
    let plugins_dir = data_dir.join("plugins");
    let Ok(entries) = fs::read_dir(&plugins_dir) else {
        return skills;
    };
    for entry in entries.flatten() {
        let plugin_name = entry.file_name().to_string_lossy().into_owned();
        let plugin_dir = entry.path();
        if !plugin_dir.is_dir()
            || plugin_name.starts_with('.')
            || !skill_name_is_valid(&plugin_name)
        {
            continue;
        }
        let skills_dir = plugin_dir.join("skills");
        if !skills_dir.is_dir() {
            continue;
        }
        // A markdown directly under `skills/` makes the plugin itself a skill.
        if let Some(markdown) = skill_markdown(&skills_dir) {
            skills.push(InstancePluginSkill {
                name: plugin_name.clone(),
                description: description_of(&markdown),
                plugin_name: plugin_name.clone(),
            });
        }
        let Ok(children) = fs::read_dir(&skills_dir) else {
            continue;
        };
        for child in children.flatten() {
            let name = child.file_name().to_string_lossy().into_owned();
            let dir = child.path();
            if !dir.is_dir() || !skill_name_is_valid(&name) {
                continue;
            }
            let Some(markdown) = skill_markdown(&dir) else {
                continue;
            };
            skills.push(InstancePluginSkill {
                name,
                description: description_of(&markdown),
                plugin_name: plugin_name.clone(),
            });
        }
    }
    skills.sort_by(|a, b| (&a.plugin_name, &a.name).cmp(&(&b.plugin_name, &b.name)));
    skills
}

fn description_of(markdown: &Path) -> String {
    fs::read_to_string(markdown)
        .map(|content| parse_frontmatter_description(&content))
        .unwrap_or_default()
}

/// Read a plugin skill's markdown; the direct layout (the plugin itself as a
/// skill) is addressed by passing the plugin name as the skill name.
pub fn read_plugin_skill(data_dir: &Path, plugin_name: &str, name: &str) -> Result<String> {
    validate_plugin_dir_name(plugin_name)?;
    validate_skill_name(name)?;
    let skills_dir = data_dir.join("plugins").join(plugin_name).join("skills");
    if !skills_dir.is_dir() {
        return Err(AppError::io(format!("插件 {plugin_name} 没有技能目录。")));
    }
    // Direct layout: the plugin itself is a skill named after its directory.
    let skill_dir = if name == plugin_name {
        skills_dir
    } else {
        skills_dir.join(name)
    };
    let markdown = skill_markdown(&skill_dir)
        .ok_or_else(|| AppError::io(format!("插件 {plugin_name} 没有技能 {name}。")))?;
    fs::read_to_string(&markdown).map_err(|e| AppError::io(format!("读取技能失败：{e}")))
}

fn install_skill_dir(data_dir: &Path, source: &Path, name: &str) -> Result<()> {
    let dest = skills_root(data_dir).join(name);
    if dest.exists() {
        return Err(AppError::io(format!(
            "技能 {name} 已存在，上传不会覆盖现有技能。"
        )));
    }
    fs::rename(source, &dest).map_err(|e| AppError::io(format!("安装技能失败：{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Result<Self> {
            let dir = std::env::temp_dir()
                .join(format!("astrbot-go-skills-{tag}-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&dir).map_err(|error| AppError::io(error.to_string()))?;
            Ok(Self(dir))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_skill(data_dir: &Path, name: &str, markdown: &str) -> Result<()> {
        let dir = skills_root(data_dir).join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("SKILL.md"), markdown)?;
        Ok(())
    }

    fn write_zip(path: &Path, entries: &[(&str, &str)]) -> Result<()> {
        let file = fs::File::create(path)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, content) in entries {
            writer.start_file(*name, options)?;
            std::io::Write::write_all(&mut writer, content.as_bytes())?;
        }
        writer.finish()?;
        Ok(())
    }

    #[test]
    fn skill_names_are_validated() {
        for name in ["weekly-report", "a.b_c", "skill1"] {
            assert!(skill_name_is_valid(name), "{name}");
        }
        for name in [
            "",
            "a b",
            "a/b",
            "..",
            ".hidden",
            ".skill-install-x",
            "a\\b",
        ] {
            assert!(!skill_name_is_valid(name), "{name}");
        }
        assert_eq!(normalize_skill_name(" weekly report "), "weekly_report");
    }

    #[test]
    fn frontmatter_description_is_parsed() {
        let content = "---\nname: weekly-report\ndescription: 整理周报时使用。\n---\n\n# 周报\n";
        assert_eq!(parse_frontmatter_description(content), "整理周报时使用。");
        assert_eq!(parse_frontmatter_description("no frontmatter"), "");
        assert_eq!(parse_frontmatter_description("---\nbroken: ["), "");
    }

    #[test]
    fn list_reports_active_state_and_description() -> Result<()> {
        let dir = TempDir::new("list")?;
        write_skill(
            dir.path(),
            "on-skill",
            "---\ndescription: On duty.\n---\nbody",
        )?;
        write_skill(dir.path(), "off-skill", "body")?;
        set_skill_active(dir.path(), "off-skill", false)?;

        let skills = list_skills(dir.path());
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "off-skill");
        assert!(!skills[0].active);
        assert_eq!(skills[0].description, "");
        assert_eq!(skills[1].name, "on-skill");
        assert!(skills[1].active);
        assert_eq!(skills[1].description, "On duty.");
        Ok(())
    }

    #[test]
    fn config_round_trip_survives_reload() -> Result<()> {
        let dir = TempDir::new("config")?;
        write_skill(dir.path(), "a", "x")?;
        set_skill_active(dir.path(), "a", false)?;
        assert!(!list_skills(dir.path())[0].active);
        set_skill_active(dir.path(), "a", true)?;
        assert!(list_skills(dir.path())[0].active);
        assert!(fs::read_to_string(skills_config_path(dir.path()))?.contains("\"skills\""));
        Ok(())
    }

    #[test]
    fn write_skill_updates_the_same_file_it_reads() -> Result<()> {
        let dir = TempDir::new("write")?;
        write_skill(dir.path(), "a", "before")?;
        write_skill(dir.path(), "a", "after")?;
        assert_eq!(read_skill(dir.path(), "a")?, "after");
        Ok(())
    }

    #[test]
    fn delete_removes_directory_and_config_entry() -> Result<()> {
        let dir = TempDir::new("delete")?;
        write_skill(dir.path(), "a", "x")?;
        set_skill_active(dir.path(), "a", false)?;
        delete_skill(dir.path(), "a")?;
        assert!(!skills_root(dir.path()).join("a").exists());
        assert!(list_skills(dir.path()).is_empty());
        Ok(())
    }

    #[test]
    fn zip_with_root_markdown_installs_under_hint_name() -> Result<()> {
        let dir = TempDir::new("zip-root")?;
        let zip = dir.path().join("weekly-report.zip");
        write_zip(&zip, &[("SKILL.md", "---\ndescription: d\n---\nbody")])?;

        let installed = install_skill_zip(
            dir.path(),
            &SkillZipInstall {
                zip_path: zip.to_string_lossy().into_owned(),
                name_hint: Some("weekly-report".to_string()),
            },
        )?;
        assert_eq!(installed, vec!["weekly-report".to_string()]);
        assert_eq!(
            read_skill(dir.path(), "weekly-report")?,
            "---\ndescription: d\n---\nbody"
        );
        Ok(())
    }

    #[test]
    fn zip_with_wrapped_folders_installs_each_skill() -> Result<()> {
        let dir = TempDir::new("zip-folders")?;
        let zip = dir.path().join("pack.zip");
        write_zip(
            &zip,
            &[
                ("weekly/SKILL.md", "one"),
                ("weekly/refs/a.md", "ref"),
                ("table/SKILL.md", "two"),
                ("__MACOSX/junk", "junk"),
                ("loose.txt", "ignored"),
            ],
        )?;

        let installed = install_skill_zip(
            dir.path(),
            &SkillZipInstall {
                zip_path: zip.to_string_lossy().into_owned(),
                name_hint: None,
            },
        )?;
        assert_eq!(installed.len(), 2);
        assert!(installed.contains(&"weekly".to_string()));
        assert!(installed.contains(&"table".to_string()));
        Ok(())
    }

    #[test]
    fn existing_skill_blocks_install() -> Result<()> {
        let dir = TempDir::new("zip-conflict")?;
        write_skill(dir.path(), "weekly", "old")?;
        let zip = dir.path().join("weekly.zip");
        write_zip(&zip, &[("SKILL.md", "new")])?;

        let error = install_skill_zip(
            dir.path(),
            &SkillZipInstall {
                zip_path: zip.to_string_lossy().into_owned(),
                name_hint: Some("weekly".to_string()),
            },
        );
        assert!(error.is_err());
        assert_eq!(read_skill(dir.path(), "weekly")?, "old");
        Ok(())
    }

    #[test]
    fn traversal_names_are_rejected() -> Result<()> {
        let dir = TempDir::new("traversal")?;
        write_skill(dir.path(), "real", "x")?;
        for bad in ["..", "a/../b", "a/b"] {
            assert!(find_skill_dir(dir.path(), bad).is_err(), "{bad}");
        }
        Ok(())
    }

    fn write_plugin_skill(
        data_dir: &Path,
        plugin: &str,
        skill: Option<&str>,
        markdown: &str,
    ) -> Result<()> {
        let skills_dir = data_dir.join("plugins").join(plugin).join("skills");
        let skill_dir = match skill {
            Some(name) => skills_dir.join(name),
            None => skills_dir,
        };
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), markdown)?;
        Ok(())
    }

    #[test]
    fn plugin_skills_list_direct_and_nested_layouts() -> Result<()> {
        let dir = TempDir::new("plugin-skills")?;
        write_plugin_skill(dir.path(), "my_plugin", None, "---\ndescription: p\n---\nx")?;
        write_plugin_skill(
            dir.path(),
            "my_plugin",
            Some("inner"),
            "---\ndescription: i\n---\ny",
        )?;
        // Staging dirs without skills content stay invisible.
        fs::create_dir_all(
            dir.path()
                .join("plugins")
                .join(".plugin-install-x")
                .join("skills"),
        )?;

        let skills = list_plugin_skills(dir.path());
        assert_eq!(skills.len(), 2);
        // Sorted by (plugin, name): "inner" sorts before the plugin itself.
        assert_eq!(skills[0].name, "inner");
        assert_eq!(skills[0].plugin_name, "my_plugin");
        assert_eq!(skills[0].description, "i");
        assert_eq!(skills[1].name, "my_plugin");
        assert_eq!(skills[1].description, "p");
        Ok(())
    }

    #[test]
    fn plugin_skills_are_readable_in_both_layouts() -> Result<()> {
        let dir = TempDir::new("plugin-read")?;
        write_plugin_skill(dir.path(), "my_plugin", None, "direct")?;
        write_plugin_skill(dir.path(), "my_plugin", Some("inner"), "nested")?;

        assert_eq!(
            read_plugin_skill(dir.path(), "my_plugin", "my_plugin")?,
            "direct"
        );
        assert_eq!(
            read_plugin_skill(dir.path(), "my_plugin", "inner")?,
            "nested"
        );
        Ok(())
    }

    #[test]
    fn plugin_skill_reads_reject_bad_names() -> Result<()> {
        let dir = TempDir::new("plugin-bad")?;
        write_plugin_skill(dir.path(), "my_plugin", Some("inner"), "x")?;
        for plugin in ["..", "a/b"] {
            assert!(
                read_plugin_skill(dir.path(), plugin, "inner").is_err(),
                "{plugin}"
            );
        }
        for skill in ["..", "a/b", "missing"] {
            assert!(
                read_plugin_skill(dir.path(), "my_plugin", skill).is_err(),
                "{skill}"
            );
        }
        Ok(())
    }
}

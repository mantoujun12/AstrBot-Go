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

use crate::archive::{extract_zip_mapped, resolve_within_dir, validate_zip_archive};
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
/// after the plugin directory. Builtin-star skills live in the instance's
/// AstrBot core (`astrbot/builtin_stars/*/skills`) and are flagged as builtin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePluginSkill {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub plugin_name: String,
    #[serde(default)]
    pub builtin: bool,
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

// === Skill file browsing (mirrors the upstream skill editor) ================

/// A skill addressed through the viewer: a local skill, or one provided by a
/// plugin / builtin star. `path` is relative to the skill root and is a
/// directory when listing or a file when reading and writing.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillFileRequest {
    pub skill_name: String,
    /// `None` addresses a local skill; `Some` a plugin-provided one.
    #[serde(default)]
    pub plugin_name: Option<String>,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillDirEntry {
    pub name: String,
    /// Path relative to the skill root, using forward slashes.
    pub path: String,
    pub is_dir: bool,
    #[serde(default)]
    pub size: u64,
}

/// Text preview cap; the viewer never renders anything larger.
const SKILL_FILE_MAX_BYTES: u64 = 2 * 1024 * 1024;

fn skill_root_dir(
    data_dir: &Path,
    builtin_root: &Path,
    plugin_name: Option<&str>,
    builtin: bool,
    skill_name: &str,
) -> Result<PathBuf> {
    validate_skill_name(skill_name)?;
    let Some(plugin) = plugin_name else {
        let root = skills_root(data_dir).join(skill_name);
        if !root.is_dir() {
            return Err(AppError::io(format!("技能 {skill_name} 不存在。")));
        }
        return Ok(root);
    };
    validate_plugin_dir_name(plugin)?;
    let plugins_root = if builtin {
        builtin_root
    } else {
        &data_dir.join("plugins")
    };
    let skills_dir = plugins_root.join(plugin).join("skills");
    // Direct layout: the plugin itself is a skill named after its directory.
    let root = if skill_name == plugin {
        skills_dir
    } else {
        skills_dir.join(skill_name)
    };
    if !root.is_dir() {
        return Err(AppError::io(format!("技能 {skill_name} 不存在。")));
    }
    Ok(root)
}

/// Resolve a viewer path inside the skill root, rejecting traversal and
/// absolute paths through the archive path guards.
fn resolve_skill_target(root: &Path, raw: &str) -> Result<PathBuf> {
    let normalized = raw.replace('\\', "/");
    if normalized.trim().is_empty() {
        return Ok(root.to_path_buf());
    }
    let relative =
        parse_entry_rel_path(&normalized).ok_or_else(|| AppError::io("文件路径不合法。"))?;
    resolve_within_dir(root, &relative)
}

/// List one directory level of a skill, directories first.
pub fn list_skill_files(
    data_dir: &Path,
    builtin_root: &Path,
    request: &SkillFileRequest,
) -> Result<Vec<SkillDirEntry>> {
    let root = skill_root_dir(
        data_dir,
        builtin_root,
        request.plugin_name.as_deref(),
        request.builtin,
        &request.skill_name,
    )?;
    let dir = resolve_skill_target(&root, &request.path)?;
    if !dir.is_dir() {
        return Err(AppError::io("目录不存在。"));
    }
    let prefix = {
        let trimmed = request.path.trim();
        (!trimmed.is_empty()).then(|| trimmed.replace('\\', "/"))
    };

    let mut entries: Vec<SkillDirEntry> = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| AppError::io(format!("读取目录失败：{e}")))?
    {
        let entry = entry.map_err(|e| AppError::io(format!("读取目录失败：{e}")))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .map_err(|e| AppError::io(format!("读取目录失败：{e}")))?;
        let path = match prefix.as_deref() {
            Some(prefix) => format!("{prefix}/{name}"),
            None => name.clone(),
        };
        let size = if file_type.is_file() {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };
        entries.push(SkillDirEntry {
            name,
            path,
            is_dir: file_type.is_dir(),
            size,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// Read one text file of a skill; non-UTF-8 content is refused instead of
/// rendered as garbage.
pub fn read_skill_file(
    data_dir: &Path,
    builtin_root: &Path,
    request: &SkillFileRequest,
) -> Result<String> {
    let root = skill_root_dir(
        data_dir,
        builtin_root,
        request.plugin_name.as_deref(),
        request.builtin,
        &request.skill_name,
    )?;
    let file = resolve_skill_target(&root, &request.path)?;
    if !file.is_file() {
        return Err(AppError::io("文件不存在。"));
    }
    let size = file
        .metadata()
        .map_err(|e| AppError::io(format!("读取文件失败：{e}")))?
        .len();
    if size > SKILL_FILE_MAX_BYTES {
        return Err(AppError::io("文件过大，无法预览。"));
    }
    let bytes = fs::read(&file).map_err(|e| AppError::io(format!("读取文件失败：{e}")))?;
    String::from_utf8(bytes).map_err(|_| AppError::io("该文件不是 UTF-8 文本，无法预览。"))
}

/// Write one text file of a local skill; plugin and builtin skills are
/// read-only, matching upstream.
pub fn write_skill_file(
    data_dir: &Path,
    builtin_root: &Path,
    request: &SkillFileRequest,
    content: &str,
) -> Result<()> {
    if request.plugin_name.is_some() || request.builtin {
        return Err(AppError::io("插件与内置技能为只读，不能修改。"));
    }
    let root = skill_root_dir(data_dir, builtin_root, None, false, &request.skill_name)?;
    let file = resolve_skill_target(&root, &request.path)?;
    if !file.is_file() {
        return Err(AppError::io("文件不存在。"));
    }
    let temp = file.with_extension("tmp-skill-edit");
    fs::write(&temp, content).map_err(|e| AppError::io(format!("写入文件失败：{e}")))?;
    replace_file_atomically(&temp, &file)
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

fn plugin_skills_under(plugins_root: &Path, builtin: bool) -> Vec<InstancePluginSkill> {
    let mut skills = Vec::new();
    let Ok(entries) = fs::read_dir(plugins_root) else {
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
                builtin,
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
                builtin,
            });
        }
    }
    skills
}

/// List the read-only skills from installed plugins plus the builtin stars
/// shipped with the instance's AstrBot core.
pub fn list_plugin_skills(data_dir: &Path, builtin_root: &Path) -> Vec<InstancePluginSkill> {
    let mut skills = plugin_skills_under(&data_dir.join("plugins"), false);
    skills.extend(plugin_skills_under(builtin_root, true));
    skills.sort_by(|a, b| (&a.plugin_name, &a.name).cmp(&(&b.plugin_name, &b.name)));
    skills
}

fn description_of(markdown: &Path) -> String {
    fs::read_to_string(markdown)
        .map(|content| parse_frontmatter_description(&content))
        .unwrap_or_default()
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

    fn skill_file_request(skill: &str, path: &str) -> SkillFileRequest {
        SkillFileRequest {
            skill_name: skill.to_string(),
            plugin_name: None,
            builtin: false,
            path: path.to_string(),
        }
    }

    #[test]
    fn skill_files_round_trip_and_reject_traversal() -> Result<()> {
        let dir = TempDir::new("files")?;
        write_skill(dir.path(), "a", "before")?;
        let builtin_root = dir.path().join("builtin_stars");
        let request = skill_file_request("a", "SKILL.md");

        write_skill_file(dir.path(), &builtin_root, &request, "after")?;
        assert_eq!(
            read_skill_file(dir.path(), &builtin_root, &request)?,
            "after"
        );

        let bad = skill_file_request("a", "../../secret.txt");
        assert!(read_skill_file(dir.path(), &builtin_root, &bad).is_err());
        let bad = skill_file_request("a", "missing.txt");
        assert!(read_skill_file(dir.path(), &builtin_root, &bad).is_err());
        Ok(())
    }

    #[test]
    fn plugin_and_builtin_skills_stay_read_only() -> Result<()> {
        let dir = TempDir::new("readonly")?;
        let builtin_root = dir.path().join("astrbot").join("builtin_stars");
        write_plugin_skill_under(&builtin_root, "builtin_plugin", Some("helper"), "x")?;
        let mut request = SkillFileRequest {
            skill_name: "helper".to_string(),
            plugin_name: Some("builtin_plugin".to_string()),
            builtin: true,
            path: "SKILL.md".to_string(),
        };
        assert_eq!(read_skill_file(dir.path(), &builtin_root, &request)?, "x");
        assert!(write_skill_file(dir.path(), &builtin_root, &request, "y").is_err());

        write_plugin_skill(dir.path(), "my_plugin", Some("inner"), "nested")?;
        request.skill_name = "inner".to_string();
        request.plugin_name = Some("my_plugin".to_string());
        request.builtin = false;
        assert_eq!(
            read_skill_file(dir.path(), &builtin_root, &request)?,
            "nested"
        );
        assert!(write_skill_file(dir.path(), &builtin_root, &request, "z").is_err());
        Ok(())
    }

    #[test]
    fn skill_file_listing_orders_directories_first_and_walks_subdirs() -> Result<()> {
        let dir = TempDir::new("listing")?;
        write_skill(dir.path(), "a", "doc")?;
        let root = skills_root(dir.path()).join("a");
        fs::create_dir_all(root.join("scripts"))?;
        fs::write(root.join("scripts").join("run.py"), "print(1)")?;
        fs::write(root.join("notes.md"), "n")?;
        let builtin_root = dir.path().join("builtin_stars");

        let entries = list_skill_files(dir.path(), &builtin_root, &skill_file_request("a", ""))?;
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "scripts");

        let sub = list_skill_files(
            dir.path(),
            &builtin_root,
            &skill_file_request("a", "scripts"),
        )?;
        assert_eq!(sub.len(), 1);
        assert_eq!(sub[0].path, "scripts/run.py");
        assert_eq!(sub[0].size, 8);
        Ok(())
    }

    #[test]
    fn binary_skill_files_are_refused() -> Result<()> {
        let dir = TempDir::new("binary")?;
        write_skill(dir.path(), "a", "doc")?;
        fs::write(
            skills_root(dir.path()).join("a").join("SKILL.md"),
            [0xFF, 0xFE],
        )?;
        let builtin_root = dir.path().join("builtin_stars");
        assert!(read_skill_file(
            dir.path(),
            &builtin_root,
            &skill_file_request("a", "SKILL.md")
        )
        .is_err());
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
            read_skill_file(
                dir.path(),
                &dir.path().join("builtin_stars"),
                &skill_file_request("weekly-report", "SKILL.md"),
            )?,
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
        assert_eq!(
            read_skill_file(
                dir.path(),
                &dir.path().join("builtin_stars"),
                &skill_file_request("weekly", "SKILL.md"),
            )?,
            "old"
        );
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
        write_plugin_skill_under(&data_dir.join("plugins"), plugin, skill, markdown)
    }

    fn write_plugin_skill_under(
        plugins_root: &Path,
        plugin: &str,
        skill: Option<&str>,
        markdown: &str,
    ) -> Result<()> {
        let skills_dir = plugins_root.join(plugin).join("skills");
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
        let builtin_root = dir.path().join("builtin_stars");

        let skills = list_plugin_skills(dir.path(), &builtin_root);
        assert_eq!(skills.len(), 2);
        // Sorted by (plugin, name): "inner" sorts before the plugin itself.
        assert_eq!(skills[0].name, "inner");
        assert_eq!(skills[0].plugin_name, "my_plugin");
        assert_eq!(skills[0].description, "i");
        assert_eq!(skills[1].name, "my_plugin");
        assert_eq!(skills[1].description, "p");
        assert!(!skills[0].builtin);
        Ok(())
    }

    #[test]
    fn builtin_star_skills_are_listed_and_flagged() -> Result<()> {
        let dir = TempDir::new("builtin-skills")?;
        let builtin_root = dir.path().join("astrbot").join("builtin_stars");
        write_plugin_skill_under(&builtin_root, "builtin_plugin", Some("helper"), "x")?;

        let skills = list_plugin_skills(dir.path(), &builtin_root);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].plugin_name, "builtin_plugin");
        assert_eq!(skills[0].name, "helper");
        assert!(skills[0].builtin);

        assert_eq!(
            read_skill_file(
                dir.path(),
                &builtin_root,
                &SkillFileRequest {
                    skill_name: "helper".to_string(),
                    plugin_name: Some("builtin_plugin".to_string()),
                    builtin: true,
                    path: "SKILL.md".to_string(),
                },
            )?,
            "x"
        );
        Ok(())
    }

    #[test]
    fn plugin_skills_are_readable_in_both_layouts() -> Result<()> {
        let dir = TempDir::new("plugin-read")?;
        let builtin_root = dir.path().join("builtin_stars");
        write_plugin_skill(dir.path(), "my_plugin", None, "direct")?;
        write_plugin_skill(dir.path(), "my_plugin", Some("inner"), "nested")?;

        let direct = SkillFileRequest {
            skill_name: "my_plugin".to_string(),
            plugin_name: Some("my_plugin".to_string()),
            builtin: false,
            path: "SKILL.md".to_string(),
        };
        let nested = SkillFileRequest {
            skill_name: "inner".to_string(),
            plugin_name: Some("my_plugin".to_string()),
            builtin: false,
            path: "SKILL.md".to_string(),
        };
        assert_eq!(
            read_skill_file(dir.path(), &builtin_root, &direct)?,
            "direct"
        );
        assert_eq!(
            read_skill_file(dir.path(), &builtin_root, &nested)?,
            "nested"
        );
        Ok(())
    }

    #[test]
    fn plugin_skill_reads_reject_bad_names() -> Result<()> {
        let dir = TempDir::new("plugin-bad")?;
        let builtin_root = dir.path().join("builtin_stars");
        write_plugin_skill(dir.path(), "my_plugin", Some("inner"), "x")?;
        for plugin in ["..", "a/b"] {
            let request = SkillFileRequest {
                skill_name: "inner".to_string(),
                plugin_name: Some(plugin.to_string()),
                builtin: false,
                path: "SKILL.md".to_string(),
            };
            assert!(
                read_skill_file(dir.path(), &builtin_root, &request).is_err(),
                "{plugin}"
            );
        }
        for skill in ["..", "a/b", "missing"] {
            let request = SkillFileRequest {
                skill_name: skill.to_string(),
                plugin_name: Some("my_plugin".to_string()),
                builtin: false,
                path: "SKILL.md".to_string(),
            };
            assert!(
                read_skill_file(dir.path(), &builtin_root, &request).is_err(),
                "{skill}"
            );
        }
        Ok(())
    }
}

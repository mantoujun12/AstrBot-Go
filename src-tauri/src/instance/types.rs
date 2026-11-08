//! Instance-related type definitions.

use serde::{Deserialize, Serialize};

use crate::process::InstanceState;

/// Status information for an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceStatus {
    pub id: String,
    pub name: String,
    pub state: InstanceState,
    pub port: u16,
    pub version: String,
    pub dashboard_enabled: bool,
    pub pid_tracker_not_available: bool,
    pub configured_host: String,
    pub configured_port: u16,
    pub check_update_enabled: bool,
    pub auto_restart_enabled: bool,
    pub auto_restart_max_retries: u32,
    pub auto_restart_notify: bool,
    /// Recorded reason for the last unexpected exit (empty when unknown).
    pub last_exit_reason: String,
    /// Consecutive crash count against the auto-restart budget.
    pub restart_attempts: u32,
    /// Start time of the tracked process for `Live` or `Stopping` states, as
    /// Unix epoch milliseconds. `None` for all other states.
    pub started_at_ms: Option<i64>,
}

/// Deploy progress messages emitted to the frontend.
#[derive(Debug, Clone, Copy)]
pub enum ProgressMessage {
    ExtractingCode,
    CodeExtracted,
    CreatingVenv,
    VenvCreated,
    InstallingDeps,
    DepsInstalled,
    PreparingRepair,
    RestoringPreservedData,
    PreservedDataRestored,
    RepairDone,
    DownloadingWebui,
    ExtractingWebui,
    WebuiReplaced,
    StartingInstance,
    InstanceStarted,
    BackingUpData,
    DataBackupDone,
    RestoringData,
    DataRestored,
    UpdateDone,
}

impl ProgressMessage {
    /// Translation key shared with the frontend locale catalogs.
    pub fn key(self) -> &'static str {
        match self {
            Self::ExtractingCode => "progress.deploy.extractingCode",
            Self::CodeExtracted => "progress.deploy.codeExtracted",
            Self::CreatingVenv => "progress.deploy.creatingVenv",
            Self::VenvCreated => "progress.deploy.venvCreated",
            Self::InstallingDeps => "progress.deploy.installingDeps",
            Self::DepsInstalled => "progress.deploy.depsInstalled",
            Self::PreparingRepair => "progress.deploy.preparingRepair",
            Self::RestoringPreservedData => "progress.deploy.restoringPreservedData",
            Self::PreservedDataRestored => "progress.deploy.preservedDataRestored",
            Self::RepairDone => "progress.deploy.repairDone",
            Self::DownloadingWebui => "progress.deploy.downloadingWebui",
            Self::ExtractingWebui => "progress.deploy.extractingWebui",
            Self::WebuiReplaced => "progress.deploy.webuiReplaced",
            Self::StartingInstance => "progress.deploy.startingInstance",
            Self::InstanceStarted => "progress.deploy.instanceStarted",
            Self::BackingUpData => "progress.deploy.backingUpData",
            Self::DataBackupDone => "progress.deploy.dataBackupDone",
            Self::RestoringData => "progress.deploy.restoringData",
            Self::DataRestored => "progress.deploy.dataRestored",
            Self::UpdateDone => "progress.deploy.updateDone",
        }
    }

    /// Chinese fallback used when the frontend cannot resolve the key.
    pub fn fallback(self) -> &'static str {
        match self {
            Self::ExtractingCode => "正在解压代码…",
            Self::CodeExtracted => "代码解压完成",
            Self::CreatingVenv => "正在创建虚拟环境…",
            Self::VenvCreated => "虚拟环境创建完成",
            Self::InstallingDeps => "正在安装依赖…",
            Self::DepsInstalled => "依赖安装完成",
            Self::PreparingRepair => "正在准备修复实例…",
            Self::RestoringPreservedData => "正在恢复保留数据…",
            Self::PreservedDataRestored => "保留数据恢复完成",
            Self::RepairDone => "实例修复完成",
            Self::DownloadingWebui => "正在下载 WebUI…",
            Self::ExtractingWebui => "正在解压 WebUI…",
            Self::WebuiReplaced => "WebUI 替换完成",
            Self::StartingInstance => "正在启动实例…",
            Self::InstanceStarted => "实例已启动",
            Self::BackingUpData => "正在备份数据…",
            Self::DataBackupDone => "数据备份完成",
            Self::RestoringData => "正在还原数据…",
            Self::DataRestored => "数据还原完成",
            Self::UpdateDone => "更新完成",
        }
    }
}

/// Deployment progress event payload.
#[derive(Debug, Clone, Serialize)]
pub struct DeployProgress {
    pub instance_id: String,
    /// Step name: "extract", "venv", "deps", "start", "done", "error"
    pub step: String,
    pub message: String,
    /// Translation key for `message`.
    pub message_key: String,
    /// Progress percentage: 0-100
    pub progress: u8,
}

/// Dashboard config from cmd_config.json.
#[derive(Debug, Deserialize)]
pub(crate) struct CmdConfigDashboard {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CmdConfig {
    #[serde(default)]
    pub dashboard: Option<CmdConfigDashboard>,
}

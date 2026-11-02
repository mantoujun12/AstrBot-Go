use serde::Serialize;

/// Identifies a managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentId {
    Python,
    Nodejs,
    UV,
}

impl ComponentId {
    /// Directory name under `components/`.
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Nodejs => "nodejs",
            Self::UV => "uv",
        }
    }

    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Nodejs => "Node.js (LTS)",
            Self::UV => "uv",
        }
    }

    /// Parse a string id (e.g. from the frontend) into a `ComponentId`.
    pub fn from_str_id(s: &str) -> Option<Self> {
        match s {
            "python" => Some(Self::Python),
            "nodejs" => Some(Self::Nodejs),
            "uv" => Some(Self::UV),
            _ => None,
        }
    }
}

/// Status of a single component, sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentStatus {
    pub id: String,
    pub installed: bool,
    pub display_name: String,
    pub description: String,
}

/// Snapshot of all component statuses.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentsSnapshot {
    pub components: Vec<ComponentStatus>,
}

/// Result of a component action, with a key the frontend can translate.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentActionResult {
    pub key: String,
    pub versions: Vec<String>,
    pub message: String,
}

impl ComponentActionResult {
    pub fn new(key: &str, message: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            versions: Vec::new(),
            message: message.into(),
        }
    }

    pub fn with_versions(key: &str, versions: Vec<String>, message: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            versions,
            message: message.into(),
        }
    }
}

/// Translation keys for component action results.
pub mod result_keys {
    pub const ALREADY_INSTALLED: &str = "components.result.alreadyInstalled";
    pub const INSTALLED: &str = "components.result.installed";
    pub const UNINSTALLED: &str = "components.result.uninstalled";
    pub const NOT_INSTALLED: &str = "components.result.notInstalled";
    pub const REINSTALLED: &str = "components.result.reinstalled";
}

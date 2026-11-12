//! Instance source discovery framework.
//!
//! Instances may come from different deployment sources (launcher-managed
//! instances, Docker containers, uv tool environments, plain directory
//! scans). Each source registers an [`InstanceSource`] implementation in the
//! [`InstanceSourceRegistry`], and the unified discovery command returns the
//! merged instance list together with the registered source metadata.

use std::sync::LazyLock;

use serde::Serialize;

use crate::config::AppManifest;
use crate::error::Result;
use crate::process::{InstanceState, ProcessManager};

use super::list_instances;

/// Capability flags describing what the launcher may do with an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct InstanceCapabilities {
    /// Instances from external sources are observed only: start, stop, and
    /// update remain reserved for launcher-managed instances until dedicated
    /// integrations land.
    pub read_only: bool,
}

/// A unified instance record produced by a discovery source.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredInstance {
    /// Stable id of the source that reported this instance.
    pub source: String,
    /// Stable instance id within its source.
    pub id: String,
    pub name: String,
    /// WebUI origin, when the instance exposes a dashboard.
    pub webui_url: Option<String>,
    pub state: InstanceState,
    pub capabilities: InstanceCapabilities,
}

/// Metadata of a registered discovery source, used to render the settings.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceSourceInfo {
    pub id: String,
    /// i18n key resolving the source display name.
    pub title_key: String,
}

/// Snapshot returned by the unified discovery command.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DiscoveredInstancesSnapshot {
    pub sources: Vec<InstanceSourceInfo>,
    pub instances: Vec<DiscoveredInstance>,
}

/// Resources a discovery source may consult while scanning.
pub struct DiscoveryContext<'a> {
    pub process_manager: &'a ProcessManager,
    pub manifest: &'a AppManifest,
    /// Configured roots for the directory-scan source; empty = home dir.
    pub scan_paths: Vec<String>,
}

/// A provider of instances from one deployment mechanism.
pub trait InstanceSource: Send + Sync {
    /// Stable source id, also used as the settings toggle key.
    fn id(&self) -> &'static str;

    /// i18n key resolving the source display name.
    fn title_key(&self) -> &'static str;

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>>;
}

/// Registry of all instance sources; the list exit iterates it.
pub struct InstanceSourceRegistry {
    sources: Vec<Box<dyn InstanceSource>>,
}

impl InstanceSourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Append a source; later sources appear after earlier ones.
    pub fn register(&mut self, source: Box<dyn InstanceSource>) {
        self.sources.push(source);
    }

    pub fn source_infos(&self) -> Vec<InstanceSourceInfo> {
        self.sources
            .iter()
            .map(|source| InstanceSourceInfo {
                id: source.id().to_string(),
                title_key: source.title_key().to_string(),
            })
            .collect()
    }

    /// Run every enabled source. A failing source is logged and skipped so
    /// one broken integration cannot hide the remaining instances.
    pub fn discover_all(
        &self,
        ctx: &DiscoveryContext,
        is_enabled: &dyn Fn(&str) -> bool,
    ) -> Vec<DiscoveredInstance> {
        let mut all = Vec::new();
        for source in &self.sources {
            if !is_enabled(source.id()) {
                continue;
            }
            match source.discover(ctx) {
                Ok(instances) => all.extend(instances),
                Err(error) => {
                    log::warn!("Instance source {} failed: {}", source.id(), error);
                }
            }
        }
        all
    }
}

impl Default for InstanceSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The built-in source: instances deployed and managed by the launcher itself.
struct LauncherManagedSource;

impl InstanceSource for LauncherManagedSource {
    fn id(&self) -> &'static str {
        "launcher"
    }

    fn title_key(&self) -> &'static str {
        "settings.instances.sources.launcher"
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
        Ok(list_instances(ctx.process_manager, ctx.manifest)
            .into_iter()
            .map(|status| DiscoveredInstance {
                source: self.id().to_string(),
                id: status.id,
                name: status.name,
                webui_url: status.dashboard_enabled.then(|| {
                    format!(
                        "http://{}:{}",
                        status.configured_host, status.configured_port
                    )
                }),
                state: status.state,
                capabilities: InstanceCapabilities { read_only: false },
            })
            .collect())
    }
}

fn builtin_registry() -> InstanceSourceRegistry {
    let mut registry = InstanceSourceRegistry::new();
    registry.register(Box::new(LauncherManagedSource));
    registry.register(Box::new(super::discovery_dirscan::DirScanSource));
    registry
}

static GLOBAL_REGISTRY: LazyLock<InstanceSourceRegistry> = LazyLock::new(builtin_registry);

/// Registry used by the app; later integrations register additional sources
/// in [`builtin_registry`].
pub fn global_registry() -> &'static InstanceSourceRegistry {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    struct StaticSource {
        source_id: &'static str,
        fail: bool,
    }

    impl InstanceSource for StaticSource {
        fn id(&self) -> &'static str {
            self.source_id
        }

        fn title_key(&self) -> &'static str {
            "settings.instances.sources.launcher"
        }

        fn discover(&self, _ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
            if self.fail {
                return Err(AppError::other("probe failure"));
            }
            Ok(vec![DiscoveredInstance {
                source: self.source_id.to_string(),
                id: format!("{}-1", self.source_id),
                name: "Probe".to_string(),
                webui_url: None,
                state: InstanceState::Stopped,
                capabilities: InstanceCapabilities::default(),
            }])
        }
    }

    fn with_context<T>(run: impl FnOnce(&DiscoveryContext) -> T) -> T {
        let process_manager = ProcessManager::new();
        let manifest = AppManifest::default();
        let ctx = DiscoveryContext {
            process_manager: &process_manager,
            manifest: &manifest,
            scan_paths: Vec::new(),
        };
        run(&ctx)
    }

    fn registry_with(fail_second: bool) -> InstanceSourceRegistry {
        let mut registry = InstanceSourceRegistry::new();
        registry.register(Box::new(StaticSource {
            source_id: "alpha",
            fail: false,
        }));
        registry.register(Box::new(StaticSource {
            source_id: "beta",
            fail: fail_second,
        }));
        registry
    }

    #[test]
    fn discover_all_skips_disabled_and_failing_sources() {
        let (all, only_alpha, degraded) = with_context(|ctx| {
            let all = registry_with(false).discover_all(ctx, &|_| true);
            let only_alpha = registry_with(false).discover_all(ctx, &|id| id == "alpha");
            // A failing source is skipped instead of failing the whole scan.
            let degraded = registry_with(true).discover_all(ctx, &|_| true);
            (all, only_alpha, degraded)
        });
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].source, "alpha");
        assert_eq!(all[1].source, "beta");
        assert_eq!(only_alpha.len(), 1);
        assert_eq!(only_alpha[0].source, "alpha");
        assert_eq!(degraded.len(), 1);
        assert_eq!(degraded[0].source, "alpha");
    }

    #[test]
    fn source_infos_follow_registration_order() {
        let registry = registry_with(false);
        let infos = registry.source_infos();
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, "alpha");
        assert_eq!(infos[1].id, "beta");
        assert!(!infos[0].title_key.is_empty());
    }

    #[test]
    fn global_registry_exposes_builtin_sources() {
        let registry = global_registry();
        let infos = registry.source_infos();
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, "launcher");
        assert_eq!(infos[0].title_key, "settings.instances.sources.launcher");
        assert_eq!(infos[1].id, "dir_scan");
        assert_eq!(infos[1].title_key, "settings.instances.sources.dir_scan");

        let instances = with_context(|ctx| registry.discover_all(ctx, &|_| true));
        assert!(instances.is_empty());
    }
}

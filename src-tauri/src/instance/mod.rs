//! Instance management for AstrBot.
//!
//! New architecture:
//! - versions/ stores only zip files (core.zip)
//! - instances/{id}/core/ - extracted code for this instance
//! - instances/{id}/venv/ - virtual environment for this instance
//! - instances/{id}/core/data/ - instance data (including data/dist for webui)

mod cleanup;
mod crud;
mod deploy;
pub mod discovery;
mod discovery_dirscan;
mod discovery_docker;
pub mod discovery_uv;
mod health;
pub(crate) mod lifecycle;
mod rebuild;
mod types;

// Re-export types
pub use health::InstanceHealthReport;
pub use types::InstanceStatus;

pub(crate) use health::check_instance_health;

// Re-export CRUD operations
pub use crud::{create_instance, delete_instance, list_instances, update_instance};

// Re-export rebuild operations
pub use rebuild::{rebuild_instance_manifest_from_disk, RebuildInstanceManifestResult};

// Re-export cleanup
pub use cleanup::{clear_instance_data, clear_instance_venv, clear_pycache};
pub use deploy::{repair_instance, RepairPreserveScope};

// Re-export discovery framework
pub use discovery::{global_registry, DiscoveredInstancesSnapshot, DiscoveryContext};
pub use discovery_dirscan::validate_scan_paths;

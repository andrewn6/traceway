//! Per-project store manager for multi-tenant data isolation.
//!
//! In cloud mode, each project gets its own `PersistentStore` backed by
//! Turbopuffer namespaces prefixed with the org and project ID. This provides
//! complete data isolation between tenants and projects without any query-time filtering.
//!
//! In local mode, a single shared store is used (no isolation needed).

use std::collections::HashMap;
use std::sync::Arc;

use auth::{OrgId, ProjectId};
use storage::PersistentStore;
use tokio::sync::RwLock;
use tracing::{info, error};

use crate::AnyBackend;

pub type SharedStore = Arc<RwLock<PersistentStore<AnyBackend>>>;

/// Composite key for per-project store lookup.
type StoreKey = (OrgId, ProjectId);

/// Manages per-project PersistentStore instances.
///
/// - **Local mode**: wraps a single `SharedStore` returned for any project.
/// - **Cloud mode**: lazily creates and caches a `SharedStore` per project,
///   each with its own Turbopuffer namespace prefix (`tw_{org_short}_{project_short}`).
pub struct OrgStoreManager {
    mode: StoreMode,
}

enum StoreMode {
    /// Single store for local/dev mode. All projects share the same store.
    Single(SharedStore),

    /// Per-project stores for cloud mode with Turbopuffer.
    PerProject {
        /// Cache of (org_id, project_id) -> store. Lazily populated on first access.
        stores: RwLock<HashMap<StoreKey, SharedStore>>,
        /// Base Turbopuffer config to derive per-project configs from.
        base_config: storage_turbopuffer::TurbopufferConfig,
    },
}

impl OrgStoreManager {
    /// Create a manager for local mode (single store, no isolation).
    pub fn single(store: SharedStore) -> Self {
        Self {
            mode: StoreMode::Single(store),
        }
    }

    /// Create a manager for cloud mode with per-project Turbopuffer namespaces.
    pub fn per_org(base_config: storage_turbopuffer::TurbopufferConfig) -> Self {
        Self {
            mode: StoreMode::PerProject {
                stores: RwLock::new(HashMap::new()),
                base_config,
            },
        }
    }

    /// Get the store for a given org (backwards-compatible helper for single/local mode).
    /// In cloud mode, this should NOT be used — use `get_for_project` instead.
    pub async fn get(&self, org_id: OrgId) -> Result<SharedStore, String> {
        match &self.mode {
            StoreMode::Single(store) => Ok(store.clone()),
            StoreMode::PerProject { .. } => {
                // Fallback: use nil project_id (should not happen in well-behaved code)
                self.get_for_project(org_id, uuid::Uuid::nil()).await
            }
        }
    }

    /// Get the store for a given org + project.
    /// In local mode, always returns the same store.
    /// In cloud mode, lazily creates and caches per-project stores.
    pub async fn get_for_project(&self, org_id: OrgId, project_id: ProjectId) -> Result<SharedStore, String> {
        match &self.mode {
            StoreMode::Single(store) => Ok(store.clone()),

            StoreMode::PerProject { stores, base_config } => {
                let key = (org_id, project_id);

                // Fast path: check if already cached
                {
                    let cache = stores.read().await;
                    if let Some(store) = cache.get(&key) {
                        return Ok(store.clone());
                    }
                }

                // Slow path: create a new store for this project
                let org_short = &org_id.to_string()[..8];
                let project_short = &project_id.to_string()[..8];
                let namespace = format!("tw_{}_{}", org_short, project_short);

                let project_config = base_config.clone().with_namespace(&namespace);
                info!(
                    org_id = %org_id,
                    project_id = %project_id,
                    namespace = %namespace,
                    "Creating per-project Turbopuffer store"
                );

                let backend = storage_turbopuffer::TurbopufferBackend::new(project_config)
                    .map_err(|e| format!("Failed to create Turbopuffer backend for project {}: {}", project_id, e))?;

                let persistent = PersistentStore::open(AnyBackend::Turbopuffer(backend))
                    .await
                    .map_err(|e| {
                        error!(org_id = %org_id, project_id = %project_id, error = %e, "Failed to open store for project");
                        format!("Failed to open store for project {}: {}", project_id, e)
                    })?;

                let store: SharedStore = Arc::new(RwLock::new(persistent));

                // Cache it
                let mut cache = stores.write().await;
                cache.insert(key, store.clone());

                Ok(store)
            }
        }
    }

    /// Check if this manager is in per-org/per-project mode.
    pub fn is_per_org(&self) -> bool {
        matches!(self.mode, StoreMode::PerProject { .. })
    }

    /// List all currently-cached store keys and their stores.
    /// In single mode, returns an empty vec (no project-specific cleanup needed).
    pub async fn cached_stores(&self) -> Vec<(OrgId, SharedStore)> {
        match &self.mode {
            StoreMode::Single(_) => vec![],
            StoreMode::PerProject { stores, .. } => {
                let cache = stores.read().await;
                cache.iter().map(|((org_id, _), s)| (*org_id, s.clone())).collect()
            }
        }
    }
}

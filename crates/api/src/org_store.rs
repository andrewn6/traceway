//! Per-org store manager for multi-tenant data isolation.
//!
//! In cloud mode, each org gets its own `PersistentStore` backed by
//! Turbopuffer namespaces prefixed with the org ID. This provides
//! complete data isolation between tenants without any query-time filtering.
//!
//! In local mode, a single shared store is used (no org isolation needed).

use std::collections::HashMap;
use std::sync::Arc;

use auth::OrgId;
use storage::PersistentStore;
use tokio::sync::RwLock;
use tracing::{info, error};

use crate::AnyBackend;

pub type SharedStore = Arc<RwLock<PersistentStore<AnyBackend>>>;

/// Manages per-org PersistentStore instances.
///
/// - **Local mode**: wraps a single `SharedStore` returned for any org.
/// - **Cloud mode**: lazily creates and caches a `SharedStore` per org,
///   each with its own Turbopuffer namespace prefix (`tw_{org_id_short}`).
pub struct OrgStoreManager {
    mode: StoreMode,
}

enum StoreMode {
    /// Single store for local/dev mode. All orgs share the same store.
    Single(SharedStore),

    /// Per-org stores for cloud mode with Turbopuffer.
    PerOrg {
        /// Cache of org_id -> store. Lazily populated on first access.
        stores: RwLock<HashMap<OrgId, SharedStore>>,
        /// Base Turbopuffer config to derive per-org configs from.
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

    /// Create a manager for cloud mode with per-org Turbopuffer namespaces.
    pub fn per_org(base_config: storage_turbopuffer::TurbopufferConfig) -> Self {
        Self {
            mode: StoreMode::PerOrg {
                stores: RwLock::new(HashMap::new()),
                base_config,
            },
        }
    }

    /// Get the store for a given org. In local mode, always returns the same store.
    /// In cloud mode, lazily creates and caches per-org stores.
    pub async fn get(&self, org_id: OrgId) -> Result<SharedStore, String> {
        match &self.mode {
            StoreMode::Single(store) => Ok(store.clone()),

            StoreMode::PerOrg { stores, base_config } => {
                // Fast path: check if already cached
                {
                    let cache = stores.read().await;
                    if let Some(store) = cache.get(&org_id) {
                        return Ok(store.clone());
                    }
                }

                // Slow path: create a new store for this org
                let org_config = base_config.for_org(&org_id.to_string());
                info!(
                    org_id = %org_id,
                    namespace = %org_config.namespace,
                    "Creating per-org Turbopuffer store"
                );

                let backend = storage_turbopuffer::TurbopufferBackend::new(org_config)
                    .map_err(|e| format!("Failed to create Turbopuffer backend for org {}: {}", org_id, e))?;

                let persistent = PersistentStore::open(AnyBackend::Turbopuffer(backend))
                    .await
                    .map_err(|e| {
                        error!(org_id = %org_id, error = %e, "Failed to open store for org");
                        format!("Failed to open store for org {}: {}", org_id, e)
                    })?;

                let store: SharedStore = Arc::new(RwLock::new(persistent));

                // Cache it
                let mut cache = stores.write().await;
                cache.insert(org_id, store.clone());

                Ok(store)
            }
        }
    }

    /// Check if this manager is in per-org mode.
    pub fn is_per_org(&self) -> bool {
        matches!(self.mode, StoreMode::PerOrg { .. })
    }
}

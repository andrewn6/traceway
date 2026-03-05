use std::time::Duration;

use auth::{OrgId, ProjectId};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use trace::{
    CaptureRule, Datapoint, Dataset, EvalResult, EvalRun, ProviderConnection, ProviderConnectionId,
    QueueItem,
};

use crate::{
    ClaimRequest, CreateCaptureRuleRequest, CreateEvalRunRequest, DatapointKind,
    EvalRunDetailResponse, EnqueueRequest, SubmitRequest, UpdateCaptureRuleRequest,
    UpdateDatasetRequest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlPlaneMode {
    Off,
    Shadow,
    On,
}

impl ControlPlaneMode {
    pub fn from_env() -> Self {
        let mode = std::env::var("TRACEWAY_BACKEND_MODE")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_MODE"))
            .unwrap_or_else(|_| "off".to_string())
            .to_ascii_lowercase()
            ;
        match mode.as_str() {
            "on" => Self::On,
            "shadow" => Self::Shadow,
            _ => Self::Off,
        }
    }
}

#[derive(Clone)]
pub struct ControlPlaneClient {
    pub mode: ControlPlaneMode,
    base_url: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderConnectionsResponse {
    connections: Vec<ProviderConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatasetsResponse {
    datasets: Vec<Dataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatapointsResponse {
    items: Vec<Datapoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalRunsResponse {
    runs: Vec<EvalRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalResultsResponse {
    items: Vec<EvalResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueItemsResponse {
    items: Vec<QueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaptureRulesResponse {
    items: Vec<CaptureRule>,
}

#[derive(Debug, Clone, Serialize)]
struct ScopeQuery {
    org_id: String,
    project_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateDatasetBody {
    org_id: String,
    project_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateDatasetBody {
    org_id: String,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateDatapointBody {
    org_id: String,
    project_id: String,
    dataset_id: String,
    kind: DatapointKind,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateEvalRunBody {
    org_id: String,
    project_id: String,
    dataset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    config: trace::EvalConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    scoring: Option<trace::ScoringStrategy>,
}

#[derive(Debug, Clone, Serialize)]
struct CancelEvalBody {
    org_id: String,
    project_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct EnqueueBody {
    org_id: String,
    project_id: String,
    dataset_id: String,
    datapoint_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ClaimBody {
    org_id: String,
    project_id: String,
    claimed_by: String,
}

#[derive(Debug, Clone, Serialize)]
struct SubmitBody {
    org_id: String,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    edited_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateCaptureRuleBody {
    org_id: String,
    project_id: String,
    dataset_id: String,
    name: String,
    filters: trace::CaptureFilters,
    sample_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateCaptureRuleBody {
    org_id: String,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<trace::CaptureFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate: Option<f64>,
}

impl ControlPlaneClient {
    pub fn from_env() -> Self {
        let mode = ControlPlaneMode::from_env();
        let base_url = std::env::var("TRACEWAY_BACKEND_URL")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_URL"))
            .ok()
            .map(|s| s.trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());

        let mut headers = HeaderMap::new();
        if let Ok(token) = std::env::var("TRACEWAY_BACKEND_TOKEN")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_TOKEN"))
        {
            if !token.trim().is_empty() {
                if let Ok(v) = HeaderValue::from_str(token.trim()) {
                    headers.insert("x-traceway-control-token", v);
                }
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            mode,
            base_url,
            client,
        }
    }

    fn is_enabled(&self) -> bool {
        self.mode != ControlPlaneMode::Off
    }

    fn base(&self) -> Result<&str, String> {
        self.base_url
            .as_deref()
            .ok_or_else(|| "TRACEWAY_BACKEND_URL is not configured".to_string())
    }

    fn scope_query(&self, org_id: OrgId, project_id: ProjectId) -> ScopeQuery {
        ScopeQuery {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
        }
    }

    pub async fn list_provider_connections(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
    ) -> Result<Option<Vec<ProviderConnection>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/provider-connections", self.base()?);
        let res = self
            .client
            .get(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<ProviderConnectionsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.connections))
    }

    pub async fn get_provider_connection(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        conn_id: ProviderConnectionId,
    ) -> Result<Option<ProviderConnection>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/provider-connections/{}", self.base()?, conn_id);
        let res = self
            .client
            .get(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let conn = res
            .json::<ProviderConnection>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(conn))
    }

    pub async fn save_provider_connection(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        conn: &ProviderConnection,
    ) -> Result<(), String> {
        if !self.is_enabled() {
            return Ok(());
        }
        let url = format!("{}/internal/provider-connections/{}", self.base()?, conn.id);
        let res = self
            .client
            .put(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
            ])
            .json(conn)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        Ok(())
    }

    pub async fn delete_provider_connection(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        conn_id: ProviderConnectionId,
    ) -> Result<(), String> {
        if !self.is_enabled() {
            return Ok(());
        }
        let url = format!("{}/internal/provider-connections/{}", self.base()?, conn_id);
        let res = self
            .client
            .delete(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        Ok(())
    }

    pub async fn list_datasets(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
    ) -> Result<Option<Vec<Dataset>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets", self.base()?);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .get(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<DatasetsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.datasets))
    }

    pub async fn get_dataset(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
    ) -> Result<Option<Dataset>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}", self.base()?, dataset_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .get(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let dataset = res
            .json::<Dataset>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(dataset))
    }

    pub async fn create_dataset(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        name: String,
        description: Option<String>,
    ) -> Result<Option<Dataset>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets", self.base()?);
        let body = CreateDatasetBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            name,
            description,
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let dataset = res
            .json::<Dataset>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(dataset))
    }

    pub async fn update_dataset(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
        req: &UpdateDatasetRequest,
    ) -> Result<Option<Dataset>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}", self.base()?, dataset_id);
        let body = UpdateDatasetBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            name: req.name.clone(),
            description: req.description.clone(),
        };
        let res = self
            .client
            .patch(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let dataset = res
            .json::<Dataset>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(dataset))
    }

    pub async fn delete_dataset(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
    ) -> Result<Option<bool>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct OkResponse {
            ok: bool,
        }
        let url = format!("{}/internal/datasets/{}", self.base()?, dataset_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .delete(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<OkResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.ok))
    }

    pub async fn list_datapoints(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
    ) -> Result<Option<Vec<Datapoint>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}/datapoints", self.base()?, dataset_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .get(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<DatapointsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.items))
    }

    pub async fn create_datapoint(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
        kind: DatapointKind,
    ) -> Result<Option<Datapoint>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}/datapoints", self.base()?, dataset_id);
        let body = CreateDatapointBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            dataset_id: dataset_id.to_string(),
            kind,
            source: "manual".to_string(),
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let datapoint = res
            .json::<Datapoint>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(datapoint))
    }

    pub async fn list_eval_runs(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
    ) -> Result<Option<Vec<EvalRun>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/eval-runs", self.base()?);
        let res = self
            .client
            .get(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
                ("dataset_id", dataset_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<EvalRunsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.runs))
    }

    pub async fn create_eval_run(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
        req: CreateEvalRunRequest,
    ) -> Result<Option<EvalRun>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/eval-runs", self.base()?);
        let body = CreateEvalRunBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            dataset_id: dataset_id.to_string(),
            name: req.name,
            config: req.config,
            scoring: Some(req.scoring),
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let run = res
            .json::<EvalRun>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(run))
    }

    pub async fn get_eval_run_detail(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        run_id: trace::EvalRunId,
    ) -> Result<Option<EvalRunDetailResponse>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/eval-runs/{}", self.base()?, run_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .get(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let detail = res
            .json::<EvalRunDetailResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(detail))
    }

    pub async fn list_eval_results(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        run_id: trace::EvalRunId,
    ) -> Result<Option<Vec<EvalResult>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/eval-results", self.base()?);
        let res = self
            .client
            .get(url)
            .query(&[
                ("org_id", org_id.to_string()),
                ("project_id", project_id.to_string()),
                ("run_id", run_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<EvalResultsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.items))
    }

    pub async fn delete_eval_run(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        run_id: trace::EvalRunId,
    ) -> Result<Option<bool>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct OkResponse {
            ok: bool,
        }
        let url = format!("{}/internal/eval-runs/{}", self.base()?, run_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .delete(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<OkResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.ok))
    }

    pub async fn cancel_eval_run(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        run_id: trace::EvalRunId,
    ) -> Result<Option<EvalRun>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/eval-runs/{}/cancel", self.base()?, run_id);
        let body = CancelEvalBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let run = res
            .json::<EvalRun>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(run))
    }

    pub async fn list_queue_items(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: Option<trace::DatasetId>,
    ) -> Result<Option<Vec<QueueItem>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/queue", self.base()?);
        let mut query = vec![
            ("org_id".to_string(), org_id.to_string()),
            ("project_id".to_string(), project_id.to_string()),
        ];
        if let Some(dataset_id) = dataset_id {
            query.push(("dataset_id".to_string(), dataset_id.to_string()));
        }
        let res = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<QueueItemsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.items))
    }

    pub async fn enqueue_datapoints(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
        req: EnqueueRequest,
    ) -> Result<Option<Vec<QueueItem>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/queue/enqueue", self.base()?);
        let body = EnqueueBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            dataset_id: dataset_id.to_string(),
            datapoint_ids: req.datapoint_ids.into_iter().map(|id| id.to_string()).collect(),
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<QueueItemsResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.items))
    }

    pub async fn claim_queue_item(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        item_id: trace::QueueItemId,
        req: ClaimRequest,
    ) -> Result<Option<QueueItem>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/queue/{}/claim", self.base()?, item_id);
        let body = ClaimBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            claimed_by: req.claimed_by,
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let item = res
            .json::<QueueItem>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(item))
    }

    pub async fn submit_queue_item(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        item_id: trace::QueueItemId,
        req: SubmitRequest,
    ) -> Result<Option<QueueItem>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/queue/{}/submit", self.base()?, item_id);
        let body = SubmitBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            edited_data: req.edited_data,
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let item = res
            .json::<QueueItem>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(item))
    }

    pub async fn list_capture_rules(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
    ) -> Result<Option<Vec<CaptureRule>>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}/rules", self.base()?, dataset_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .get(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<CaptureRulesResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.items))
    }

    pub async fn create_capture_rule(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        dataset_id: trace::DatasetId,
        req: CreateCaptureRuleRequest,
    ) -> Result<Option<CaptureRule>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/datasets/{}/rules", self.base()?, dataset_id);
        let body = CreateCaptureRuleBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            dataset_id: dataset_id.to_string(),
            name: req.name,
            filters: req.filters,
            sample_rate: req.sample_rate,
        };
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let rule = res
            .json::<CaptureRule>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(rule))
    }

    pub async fn update_capture_rule(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        rule_id: trace::CaptureRuleId,
        req: UpdateCaptureRuleRequest,
    ) -> Result<Option<CaptureRule>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/rules/{}", self.base()?, rule_id);
        let body = UpdateCaptureRuleBody {
            org_id: org_id.to_string(),
            project_id: project_id.to_string(),
            name: req.name,
            filters: req.filters,
            sample_rate: req.sample_rate,
        };
        let res = self
            .client
            .patch(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let rule = res
            .json::<CaptureRule>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(rule))
    }

    pub async fn toggle_capture_rule(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        rule_id: trace::CaptureRuleId,
    ) -> Result<Option<CaptureRule>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let url = format!("{}/internal/rules/{}/toggle", self.base()?, rule_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .post(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let rule = res
            .json::<CaptureRule>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(rule))
    }

    pub async fn delete_capture_rule(
        &self,
        org_id: OrgId,
        project_id: ProjectId,
        rule_id: trace::CaptureRuleId,
    ) -> Result<Option<bool>, String> {
        if !self.is_enabled() {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct OkResponse {
            ok: bool,
        }
        let url = format!("{}/internal/rules/{}", self.base()?, rule_id);
        let q = self.scope_query(org_id, project_id);
        let res = self
            .client
            .delete(url)
            .query(&q)
            .send()
            .await
            .map_err(|e| format!("backend request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("backend returned HTTP {}", res.status()));
        }
        let payload = res
            .json::<OkResponse>()
            .await
            .map_err(|e| format!("backend decode failed: {e}"))?;
        Ok(Some(payload.ok))
    }
}

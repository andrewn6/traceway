//! Polar.sh webhook handler for billing events.
//!
//! Receives subscription lifecycle webhooks from Polar, verifies the signature
//! using the Standard Webhooks spec (HMAC-SHA256), and updates the org's plan
//! in the auth store.
//!
//! Polar products are expected to have a `metadata.traceway_plan` field set to
//! one of: "pro", "team", "enterprise". When a subscription becomes active the
//! org plan is upgraded; when revoked/canceled it resets to Free.
//!
//! The checkout link should include `?metadata[org_id]=<uuid>` so we know which
//! Traceway org to update. Alternatively, the customer's `external_id` can be
//! set to the org ID when creating the Polar customer.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::AppState;
use auth::Plan;

type HmacSha256 = Hmac<Sha256>;

// ── Polar webhook payload types (minimal, we only need subscription events) ──

#[derive(serde::Deserialize, Debug)]
struct PolarWebhookPayload {
    /// e.g. "subscription.created", "subscription.updated", "subscription.revoked"
    #[serde(rename = "type")]
    event_type: String,
    data: PolarSubscription,
}

#[derive(serde::Deserialize, Debug)]
struct PolarSubscription {
    status: String,
    #[serde(default)]
    cancel_at_period_end: bool,
    product: PolarProduct,
    customer: PolarCustomer,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(serde::Deserialize, Debug)]
struct PolarProduct {
    name: String,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(serde::Deserialize, Debug)]
struct PolarCustomer {
    email: String,
    #[serde(default)]
    external_id: Option<String>,
    #[serde(default)]
    metadata: serde_json::Value,
}

// ── Signature verification (Standard Webhooks spec) ──

fn verify_webhook_signature(
    body: &[u8],
    headers: &HeaderMap,
    secret: &str,
) -> Result<(), String> {
    // Standard Webhooks headers
    let msg_id = headers
        .get("webhook-id")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing webhook-id header")?;
    let msg_timestamp = headers
        .get("webhook-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing webhook-timestamp header")?;
    let msg_signature = headers
        .get("webhook-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing webhook-signature header")?;

    // Verify timestamp is within tolerance (5 min)
    let ts: i64 = msg_timestamp
        .parse()
        .map_err(|_| "Invalid webhook-timestamp")?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err("Webhook timestamp too old or too new".into());
    }

    // The secret from Polar is prefixed with "whsec_" and base64-encoded after that
    let secret_bytes = if let Some(stripped) = secret.strip_prefix("whsec_") {
        base64::engine::general_purpose::STANDARD
            .decode(stripped)
            .map_err(|e| format!("Invalid webhook secret encoding: {}", e))?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(secret)
            .map_err(|e| format!("Invalid webhook secret encoding: {}", e))?
    };

    // Construct the signed content: "{msg_id}.{msg_timestamp}.{body}"
    let signed_content = format!(
        "{}.{}.{}",
        msg_id,
        msg_timestamp,
        String::from_utf8_lossy(body)
    );

    let mut mac =
        HmacSha256::new_from_slice(&secret_bytes).map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(signed_content.as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(expected);

    // The signature header can contain multiple signatures separated by spaces,
    // each prefixed with "v1,"
    let valid = msg_signature.split(' ').any(|sig| {
        if let Some(sig_b64) = sig.strip_prefix("v1,") {
            sig_b64 == expected_b64
        } else {
            false
        }
    });

    if valid {
        Ok(())
    } else {
        Err("Invalid webhook signature".into())
    }
}

// ── Determine plan from Polar product ──

fn plan_from_product(product: &PolarProduct) -> Plan {
    // First check product metadata for explicit plan mapping
    if let Some(plan_str) = product.metadata.get("traceway_plan").and_then(|v| v.as_str()) {
        return match plan_str {
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            "enterprise" => Plan::Enterprise,
            _ => Plan::Free,
        };
    }

    // Fallback: infer from product name
    let name_lower = product.name.to_lowercase();
    if name_lower.contains("enterprise") {
        Plan::Enterprise
    } else if name_lower.contains("team") {
        Plan::Team
    } else if name_lower.contains("pro") {
        Plan::Pro
    } else {
        Plan::Free
    }
}

// ── Resolve org_id from subscription data ──

fn resolve_org_id(sub: &PolarSubscription) -> Option<uuid::Uuid> {
    // 1. Check subscription-level metadata (set via checkout link ?metadata[org_id]=...)
    if let Some(id_str) = sub.metadata.get("org_id").and_then(|v| v.as_str()) {
        if let Ok(id) = id_str.parse() {
            return Some(id);
        }
    }

    // 2. Check customer external_id (set when creating Polar customer)
    if let Some(ref ext_id) = sub.customer.external_id {
        if let Ok(id) = ext_id.parse() {
            return Some(id);
        }
    }

    // 3. Check customer metadata
    if let Some(id_str) = sub
        .customer
        .metadata
        .get("org_id")
        .and_then(|v| v.as_str())
    {
        if let Ok(id) = id_str.parse() {
            return Some(id);
        }
    }

    None
}

// ── Webhook handler ──

async fn handle_polar_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify signature
    if let Some(ref secret) = state.polar_webhook_secret {
        if let Err(e) = verify_webhook_signature(&body, &headers, secret) {
            tracing::warn!("Polar webhook signature verification failed: {}", e);
            return Err((StatusCode::FORBIDDEN, e));
        }
    } else {
        tracing::warn!("POLAR_WEBHOOK_SECRET not set, skipping signature verification");
    }

    // Parse payload
    let payload: PolarWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid payload: {}", e)))?;

    tracing::info!(
        "Polar webhook: type={}, product={}, customer={}, status={}",
        payload.event_type,
        payload.data.product.name,
        payload.data.customer.email,
        payload.data.status,
    );

    // Only handle subscription events
    if !payload.event_type.starts_with("subscription.") {
        return Ok(StatusCode::OK);
    }

    let auth_store = state
        .auth_store
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Resolve which org this subscription belongs to
    let org_id = resolve_org_id(&payload.data).ok_or_else(|| {
        tracing::error!(
            "Could not resolve org_id from Polar webhook for customer={}",
            payload.data.customer.email
        );
        (
            StatusCode::BAD_REQUEST,
            "Cannot determine org_id from webhook data".into(),
        )
    })?;

    // Load the org
    let mut org = auth_store
        .get_org(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Org {} not found", org_id)))?;

    // Determine the new plan based on event type + subscription status
    let new_plan = match payload.event_type.as_str() {
        "subscription.created" | "subscription.active" | "subscription.uncanceled" => {
            if payload.data.status == "active" || payload.data.status == "trialing" {
                plan_from_product(&payload.data.product)
            } else {
                org.plan // no change
            }
        }
        "subscription.updated" => {
            match payload.data.status.as_str() {
                "active" | "trialing" => plan_from_product(&payload.data.product),
                "canceled" if !payload.data.cancel_at_period_end => Plan::Free,
                "canceled" => org.plan, // still active until period end
                _ => org.plan,
            }
        }
        "subscription.revoked" | "subscription.canceled" => {
            // If revoked, downgrade to free. If just canceled (end-of-period),
            // keep current plan until revoked.
            if payload.data.status == "canceled"
                && payload.event_type == "subscription.revoked"
            {
                Plan::Free
            } else if payload.data.cancel_at_period_end {
                org.plan // keep until period end
            } else {
                Plan::Free
            }
        }
        _ => org.plan,
    };

    if new_plan != org.plan {
        tracing::info!(
            "Updating org {} plan: {:?} -> {:?}",
            org_id,
            org.plan,
            new_plan
        );
        org.plan = new_plan;
        org.updated_at = chrono::Utc::now();
        auth_store.save_org(&org).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update org plan: {}", e),
            )
        })?;
    }

    Ok(StatusCode::OK)
}

// ── Router ──

/// Public billing routes (no auth required — Polar calls these).
pub fn billing_router() -> Router<AppState> {
    Router::new().route("/billing/polar/webhook", post(handle_polar_webhook))
}

//! `x.ai/share_session` extension handler.
//!
//! Session sharing was removed. The method remains so an older client gets a
//! clear error instead of a missing-method fault.

use agent_client_protocol as acp;

use super::ExtResult;
use crate::agent::MvpAgent;

#[tracing::instrument(skip_all, fields(method = %args.method))]
pub async fn handle(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    match args.method.as_ref() {
        "x.ai/share_session" => {
            tracing::info!("handling share session request");
            handle_share_session(agent, args).await
        }
        _ => Err(acp::Error::method_not_found()),
    }
}

async fn handle_share_session(_agent: &MvpAgent, _args: &acp::ExtRequest) -> ExtResult {
    // Session sharing was removed. The method stays so older clients get a
    // clear error instead of a missing-method fault. The upload path is gone.
    Err(acp::Error::invalid_params().data("Session sharing has been removed."))
}

#[cfg(test)]
fn require_xai_auth_for_share(
    auth_manager: &xai_grok_login::AuthManager,
) -> Result<xai_grok_login::GrokAuth, acp::Error> {
    super::auth_gate::require_xai_auth(
        auth_manager,
        "Authentication required to share session",
        "Share session is disabled. Run `grok login` to authenticate.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::sync::Arc;
    use tempfile::tempdir;
    use xai_grok_login::GrokComConfig;
    use xai_grok_login::{AuthMode, GrokAuth};

    fn make_auth_manager_with_token_expiring_in(
        ttl: Duration,
    ) -> (Arc<xai_grok_login::AuthManager>, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir for share auth test");
        let mgr = Arc::new(xai_grok_login::AuthManager::new(
            dir.path(),
            GrokComConfig::default(),
        ));

        let expires_at = Utc::now() + ttl;

        // We must explicitly set oidc_issuer to a first-party xAI issuer.
        // Only OIDC tokens against https://auth.x.ai (or the local-dev equivalent) return true from is_xai_auth()
        // The share tests need that to exercise the happy path through require_xai_auth_for_share
        let auth = GrokAuth {
            auth_mode: AuthMode::Oidc,
            oidc_issuer: Some("https://auth.x.ai".to_string()),
            key: "test-key".into(),
            expires_at: Some(expires_at),
            create_time: Utc::now() - Duration::hours(1),
            ..Default::default()
        };
        mgr.hot_swap(auth);
        (mgr, dir)
    }

    #[test]
    fn share_works_outside_the_5m_early_invalidation_window() {
        let (mgr, _dir) = make_auth_manager_with_token_expiring_in(Duration::minutes(10));
        assert!(mgr.current().is_some());
        assert!(require_xai_auth_for_share(&mgr).is_ok());
    }

    #[test]
    fn share_succeeds_inside_the_5m_early_invalidation_window() {
        let (mgr, _dir) = make_auth_manager_with_token_expiring_in(Duration::seconds(1));
        // The regression state: the token sits inside the early-invalidation buffer
        assert!(
            mgr.current().is_none(),
            "current() drops the token inside the buffer"
        );
        assert!(mgr.expired_auth().is_some());

        // require_xai_auth_for_share reads current_or_expired(), so this passes
        let res = require_xai_auth_for_share(&mgr);
        assert!(
            res.is_ok(),
            "require_xai_auth_for_share must succeed for a still-valid buffered xAI token"
        );
    }

    #[test]
    fn share_fails_with_no_auth_at_all() {
        let dir = tempdir().expect("tempdir");
        let mgr = Arc::new(xai_grok_login::AuthManager::new(
            dir.path(),
            GrokComConfig::default(),
        ));
        assert!(require_xai_auth_for_share(&mgr).is_err());
    }

    #[test]
    fn share_rejects_non_xai_auth_with_actionable_grok_login_message() {
        let dir = tempdir().expect("tempdir");
        let mgr = Arc::new(xai_grok_login::AuthManager::new(
            dir.path(),
            GrokComConfig::default(),
        ));

        // API key is the simplest non-xAI credential (External and enterprise OIDC are also rejected the same way)
        let non_xai = GrokAuth {
            auth_mode: AuthMode::ApiKey,
            key: "xai-test-key".into(),
            create_time: Utc::now(),
            ..Default::default()
        };
        mgr.hot_swap(non_xai);

        let err = require_xai_auth_for_share(&mgr)
            .expect_err("non-xAI accounts (API key, External, enterprise IdP) must be rejected");

        // Test the *exact* actionable data string for the non-xAI path (distinct from the generic "Authentication required to share session" path)
        let serialized =
            serde_json::to_value(&err).expect("acp::Error serializes to JSON-RPC shape");
        let data = serialized
            .get("data")
            .and_then(|v| v.as_str())
            .expect("auth_required error carries a data string");

        assert_eq!(
            data,
            "Share session is disabled. Run `grok login` to authenticate."
        );
    }
}

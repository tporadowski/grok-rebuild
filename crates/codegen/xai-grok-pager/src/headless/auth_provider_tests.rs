use super::authenticate;
use agent_client_protocol as acp;
use xai_acp_lib::AcpAgentMessage;

fn auth_method(id: &str, name: &str, meta: Option<serde_json::Value>) -> acp::AuthMethod {
    let mut agent = acp::AuthMethodAgent::new(acp::AuthMethodId::new(id), name.to_string());
    if let Some(meta) = meta.and_then(|value| value.as_object().cloned()) {
        agent = agent.meta(meta);
    }
    acp::AuthMethod::Agent(agent)
}

/// No stored login: the only method is `grok.com` with `external_provider`.
/// Headless must send that authenticate request, with `headless: true`, so the shell runs
/// `auth_provider_command`.
#[tokio::test]
async fn headless_sends_provider_login_when_no_login_is_stored() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let methods = vec![auth_method(
        "grok.com",
        "Acme",
        Some(serde_json::json!({ "external_provider": true })),
    )];
    let pending = tokio::spawn(async move { authenticate(&tx, &methods, None).await });
    let message = rx.recv().await.expect("provider login must be sent");
    let AcpAgentMessage::Authenticate(args) = message else {
        panic!("expected an authenticate request, got {message:?}");
    };
    assert_eq!(args.request.method_id.0.as_ref(), "grok.com");
    assert_eq!(
        args.request
            .meta
            .as_ref()
            .and_then(|meta| meta.get("headless"))
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "the shell runs auth_provider_command only on a headless authenticate"
    );
    drop(args);
    let err = pending
        .await
        .expect("authenticate task")
        .expect_err("the test drops the response");
    assert!(
        !err.to_string().contains("Not signed in"),
        "a configured provider must not be refused before it runs, got {err}"
    );
}

/// A browser `grok.com` login, with no provider and no stored login, still stops before authenticate.
#[tokio::test]
async fn headless_still_refuses_browser_login_when_no_login_is_stored() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    drop(rx);
    let methods = vec![auth_method("grok.com", "Grok", None)];
    let err = authenticate(&tx, &methods, None)
        .await
        .expect_err("browser login is not a headless provider");
    assert!(
        err.to_string().contains("Not signed in"),
        "got {err}"
    );
}

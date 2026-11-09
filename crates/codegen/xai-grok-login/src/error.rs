use std::borrow::Cow;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthError {
    #[error("Not logged in. Run `grok login`.")]
    NotLoggedIn,

    /// The token expired and no refresh authority is available.
    #[error("Token expired. Run `grok login` to re-authenticate.")]
    TokenExpiredNoRefresh,

    /// Server rejected the token (401) with no recovery path.
    #[error("Authentication rejected by server. Run `grok login` to re-authenticate.")]
    ServerRejectedNoRecovery,

    /// All recovery strategies are exhausted.
    #[error("Auth recovery exhausted; re-authentication required.")]
    RecoveryExhausted,

    /// A session's team principal violates the `force_login_team_uuid` pin.
    /// `message` states which team is required and which was returned.
    #[error("{message} Run `grok login` to sign in with the required team.")]
    PinnedTeamMismatch { message: String },

    /// The cached API-key session was rejected because API-key auth is disabled.
    #[error("API-key auth is disabled by your administrator. Run `grok login` to authenticate.")]
    ApiKeyAuthDisabled,

    /// Outcome of a refresh-authority attempt.
    /// Recoverability (and, for permanent failures, the reason) lives in [`RefreshTokenError`].
    #[error(transparent)]
    Refresh(#[from] RefreshTokenError),
}

/// Whether a token-refresh failure is permanent or transient.
/// Deliberately not `#[non_exhaustive]`: "permanent or transient" is a decision every caller must make.
/// A third state should break consumers loudly.
#[derive(Debug, Error)]
pub enum RefreshTokenError {
    /// The credential is dead; the user must re-authenticate.
    #[error(transparent)]
    Permanent(#[from] RefreshTokenFailedError),
    /// A network error, a 5xx, or an unknown blip; safe to retry later, with the cause attached.
    #[error(transparent)]
    Transient(RefreshTransientError),
}

/// A retryable refresh failure, wrapping its cause. No public `From`: construct only via [`AuthError::transient`] or [`AuthError::transient_source`].
/// That way a stray `?` on some error cannot silently classify a permanent failure as retryable.
/// Display wraps the cause in "auth refresh failed" so internal messages (lock timeout, sleep defer) are not shown to the user bare.
#[derive(Debug, Error)]
#[error("auth refresh failed: {0}")]
pub struct RefreshTransientError(#[source] Box<dyn std::error::Error + Send + Sync>);

/// A terminal refresh failure.
/// `reason` is machine-readable; the user-facing copy is derived from it via [`RefreshTokenFailedReason::user_message`], so the two can never drift.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}", .reason.user_message())]
#[non_exhaustive]
pub struct RefreshTokenFailedError {
    pub reason: RefreshTokenFailedReason,
}

impl From<RefreshTokenFailedReason> for RefreshTokenFailedError {
    fn from(reason: RefreshTokenFailedReason) -> Self {
        Self { reason }
    }
}

/// Why a token refresh terminally failed, grounded in the OAuth2 error codes our IdP actually emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefreshTokenFailedReason {
    /// `invalid_grant`: the refresh token is no longer valid (expired, reused, or revoked; the IdP does not distinguish these).
    RefreshTokenRejected,
    /// `invalid_client`: the client/app credential was rejected.
    ClientRejected,
    /// The operator's `auth_provider_command` could not produce a credential in a headless run (`GROK_AUTH_EXPIRED=1`).
    ProviderInteractiveRequired,
    /// Escalation from repeated transient failures (OIDC).
    /// Never a raw IdP code: an unrecognized terminal code is classified transient, not `Other` (see `classify_terminal`).
    Other,
}

impl RefreshTokenFailedReason {
    /// Sticky until the credential changes (never ages out): a revoked refresh token never self-heals.
    /// Client rotation and transient escalation recover, so those age out past the TTL.
    pub fn is_sticky(self) -> bool {
        match self {
            Self::RefreshTokenRejected => true,
            Self::ClientRejected | Self::ProviderInteractiveRequired | Self::Other => false,
        }
    }

    /// Whether the verdict rules out an unattended retry for as long as it stands.
    /// Independent of [`Self::is_sticky`], which is about whether the verdict ever ages out.
    pub fn blocks_unattended_retry(self) -> bool {
        match self {
            Self::RefreshTokenRejected | Self::ProviderInteractiveRequired => true,
            Self::ClientRejected | Self::Other => false,
        }
    }

    /// User-facing copy for a terminal refresh failure; the raw IdP code stays in logs.
    pub fn user_message(self) -> Cow<'static, str> {
        match self {
            Self::RefreshTokenRejected => {
                "Your session has expired. Run `grok login` to sign in again.".into()
            }
            Self::ClientRejected => {
                "Authentication is temporarily unavailable. Run `grok login` if this persists."
                    .into()
            }
            Self::ProviderInteractiveRequired => provider_login_message(None),
            Self::Other => {
                "Authentication could not be refreshed. Run `grok login` to sign in again.".into()
            }
        }
    }
}

/// `label` is the operator's `auth_provider_label`, when the caller has one.
pub fn provider_login_message(label: Option<&str>) -> Cow<'static, str> {
    match label {
        Some(label) => format!(
            "Your session expired and {label} could not renew it in the background. \
             Run /login to sign in again."
        )
        .into(),
        None => "Your session expired and your sign-in helper could not renew it in the \
                 background. Run /login to sign in again."
            .into(),
    }
}

impl AuthError {
    /// A retryable refresh failure for the sites that have only a message (lock timeout, sleep/dark-wake defer, no refresher).
    /// Use [`Self::transient_source`] when a real error is in hand.
    pub fn transient(message: impl Into<String>) -> Self {
        Self::transient_source(message.into())
    }

    /// A retryable refresh failure that preserves `source` in the error chain (`Transient` carries the cause).
    /// Callers with a real error should use this rather than flattening it to a string.
    pub fn transient_source(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        AuthError::Refresh(RefreshTokenError::Transient(RefreshTransientError(
            source.into(),
        )))
    }

    /// A terminal refresh failure for an already-classified `reason`.
    pub fn permanent(reason: RefreshTokenFailedReason) -> Self {
        AuthError::Refresh(RefreshTokenError::Permanent(reason.into()))
    }

    /// True for a retryable refresh failure (network, 5xx, sleep/dark-wake defer, etc.).
    /// Permanent failures, NotLoggedIn, and policy rejects are not transient.
    pub fn is_transient(&self) -> bool {
        matches!(self, AuthError::Refresh(RefreshTokenError::Transient(_)))
    }
}

const SIGN_IN_CANCELLED: &str = "Sign-in was cancelled. Run `grok login` to try again.";
const SIGN_IN_TIMED_OUT: &str = "Sign-in timed out. Run `grok login` to try again.";
const SIGN_IN_DECLINED: &str = "Sign-in was declined. Run `grok login` to try again.";
const SIGN_IN_EXPIRED: &str = "Your sign-in expired. Run `grok login` to sign in again.";
const SIGN_IN_BIND: &str =
    "Couldn't open the local sign-in page. Close other sign-in windows and run `grok login` again.";
const SIGN_IN_CERTIFICATE: &str =
    "Couldn't verify the sign-in server. Check your network and run `grok login` again.";
const SIGN_IN_NETWORK: &str =
    "Couldn't reach the sign-in service. Check your network and run `grok login` again.";
const SIGN_IN_REJECTED: &str =
    "The sign-in server rejected this attempt. Run `grok login` to try again.";
const SIGN_IN_UNAVAILABLE: &str =
    "The sign-in service is unavailable. Wait a minute and run `grok login` again.";
const SIGN_IN_MISMATCH: &str =
    "This sign-in didn't match the attempt that started it. Run `grok login` to start over.";
const SIGN_IN_UNVERIFIED: &str =
    "The sign-in response couldn't be verified. Run `grok login` to try again.";
const SIGN_IN_SAVE: &str = "Couldn't save the sign-in. Check that your Grok home directory is writable, then run `grok login` again.";
const SIGN_IN_PASTE: &str =
    "That sign-in code couldn't be read. Paste the full callback link and run `grok login` again.";
const SIGN_IN_API_KEY: &str =
    "API-key sign-in isn't available. Set XAI_API_KEY, or run `grok login`.";
const SIGN_IN_INTERRUPTED: &str = "Sign-in was interrupted. Run `grok login` to try again.";
const SIGN_IN_FAILED: &str = "Sign-in failed. Run `grok login` to try again.";

/// The sentence a sign-in surface shows instead of a raw login dump.
/// A message that is already plain and names a next step is kept.
/// Protocol text, HTTP bodies, and URLs are not copied into the result.
pub fn sign_in_failure_message(raw: &str) -> String {
    let cleaned = first_sign_in_line(raw);
    if !looks_raw(&cleaned) && has_next_step(&cleaned) {
        return cleaned;
    }
    if !looks_raw(&cleaned) {
        if let Some(known) = known_plain_failure(&cleaned) {
            return known.to_string();
        }
        if !cleaned.is_empty() {
            return with_next_step(&cleaned);
        }
    }
    classify_sign_in_failure(raw).to_string()
}

/// Replace a login error with the sentence [`sign_in_failure_message`] produces.
/// The raw chain stays in the log. The error the user sees does not carry it.
pub(crate) fn sign_in_failure_anyhow(err: anyhow::Error) -> anyhow::Error {
    tracing::warn!(error = %err, "sign-in failed");
    anyhow::Error::msg(sign_in_failure_message(&format!("{err:#}")))
}

fn first_sign_in_line(raw: &str) -> String {
    let line = raw.lines().next().unwrap_or("").trim();
    let mut text = line.to_string();
    loop {
        let stripped = text
            .strip_prefix("Authentication required: ")
            .or_else(|| text.strip_prefix("Authentication failed: "))
            .or_else(|| text.strip_prefix("ACP error: "));
        match stripped {
            Some(rest) => text = rest.trim().to_string(),
            None => break,
        }
    }
    if text.starts_with('"')
        && text.ends_with('"')
        && let Ok(parsed) = serde_json::from_str::<String>(&text)
    {
        text = parsed;
    }
    text.trim().to_string()
}

fn has_next_step(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    const CUES: &[&str] = &[
        "grok login",
        "/login",
        "try again",
        "xai_api_key",
        "api key",
        "contact your administrator",
        "wait a minute",
        "check your network",
    ];
    CUES.iter().any(|cue| lower.contains(cue))
}

fn looks_raw(text: &str) -> bool {
    if text.chars().count() > 240 {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "http://",
        "https://",
        "oidc",
        "reqwest",
        "os error",
        "invalid_grant",
        "error_description",
        "id_token",
        "jwks",
        "preferred_method",
        "token exchange error",
        "device code request failed",
        "failed to bind",
        "failed to parse",
        "json-rpc",
        "json rpc",
        "error sending request",
        "channel closed",
        "stack backtrace",
    ];
    if MARKERS.iter().any(|marker| lower.contains(marker)) {
        return true;
    }
    if text.contains(['{', '}', '[', ']']) {
        return true;
    }
    http_status_in(text).is_some()
}

fn known_plain_failure(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("cancelled") || lower.contains("canceled") {
        return Some(SIGN_IN_CANCELLED);
    }
    if lower.contains("denied") || lower.contains("rejected") {
        return Some(SIGN_IN_DECLINED);
    }
    if lower.contains("expired") {
        return Some(SIGN_IN_EXPIRED);
    }
    None
}

fn with_next_step(sentence: &str) -> String {
    let trimmed = sentence.trim().trim_end_matches('.').trim();
    if trimmed.is_empty() {
        SIGN_IN_FAILED.to_string()
    } else {
        format!("{trimmed}. Run `grok login` to try again.")
    }
}

fn classify_sign_in_failure(raw: &str) -> &'static str {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("cancelled") || lower.contains("canceled") {
        return SIGN_IN_CANCELLED;
    }
    if lower.contains("access_denied")
        || lower.contains("authorization denied")
        || lower.contains("rejected the request")
        || lower.contains("user rejected")
    {
        return SIGN_IN_DECLINED;
    }
    if lower.contains("address already in use")
        || lower.contains("failed to bind")
        || lower.contains("loopback")
    {
        return SIGN_IN_BIND;
    }
    if lower.contains("certificate") || lower.contains("unknown issuer") || lower.contains("ssl") {
        return SIGN_IN_CERTIFICATE;
    }
    if lower.contains("error sending request")
        || lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("name or service not known")
        || lower.contains("network is unreachable")
        || lower.contains("dns error")
    {
        return SIGN_IN_NETWORK;
    }
    if let Some(status) = http_status_in(raw) {
        return match status {
            408 | 504 => SIGN_IN_TIMED_OUT,
            404 | 429 | 500..=503 | 505..=599 => SIGN_IN_UNAVAILABLE,
            _ => SIGN_IN_REJECTED,
        };
    }
    if lower.contains("state mismatch") || lower.contains("nonce mismatch") {
        return SIGN_IN_MISMATCH;
    }
    if lower.contains("id_token")
        || lower.contains("validation failed")
        || lower.contains("audience mismatch")
        || lower.contains("issuer mismatch")
        || lower.contains("unsupported algorithm")
    {
        return SIGN_IN_UNVERIFIED;
    }
    if lower.contains("failed to save") || lower.contains("permission denied") {
        return SIGN_IN_SAVE;
    }
    if lower.contains("pasted") || lower.contains("no 'code'") || lower.contains("empty input") {
        return SIGN_IN_PASTE;
    }
    if lower.contains("preferred_method") || lower.contains("api-key auth is not allowed") {
        return SIGN_IN_API_KEY;
    }
    if lower.contains("timed out") || lower.contains("timeout") {
        return SIGN_IN_TIMED_OUT;
    }
    if lower.contains("expired") {
        return SIGN_IN_EXPIRED;
    }
    if lower.contains("channel closed") {
        return SIGN_IN_INTERRUPTED;
    }
    SIGN_IN_FAILED
}

/// First HTTP status in 400..600, so a year or a short count is not a status.
fn http_status_in(raw: &str) -> Option<u16> {
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index + 2 < bytes.len() {
        let digits = [
            bytes.get(index).copied(),
            bytes.get(index + 1).copied(),
            bytes.get(index + 2).copied(),
        ];
        if let [Some(a), Some(b), Some(c)] = digits
            && a.is_ascii_digit()
            && b.is_ascii_digit()
            && c.is_ascii_digit()
        {
            let prev_is_digit = index > 0 && bytes.get(index - 1).is_some_and(u8::is_ascii_digit);
            let next_is_digit = bytes.get(index + 3).is_some_and(u8::is_ascii_digit);
            if !prev_is_digit
                && !next_is_digit
                && let Some(slice) = raw.get(index..index + 3)
                && let Ok(code) = slice.parse::<u16>()
                && (400..600).contains(&code)
            {
                return Some(code);
            }
            index += 3;
            continue;
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod sign_in_failure_tests {
    use super::{sign_in_failure_anyhow, sign_in_failure_message};

    fn hides_dump(raw: &str) {
        let message = sign_in_failure_message(raw);
        assert!(
            !message.contains("https://") && !message.contains("http://"),
            "url leaked: {message}"
        );
        assert!(
            !message.contains('{') && !message.contains('}'),
            "body leaked: {message}"
        );
        assert!(
            !message.to_ascii_lowercase().contains("oidc"),
            "protocol leaked: {message}"
        );
        assert!(
            message.contains("grok login") || message.contains("XAI_API_KEY"),
            "missing next step: {message}"
        );
    }

    #[test]
    fn sign_in_failure_message_replaces_a_token_exchange_dump() {
        let raw = "OIDC token exchange failed: HTTP 500 — {\"error\":\"server_error\",\"error_description\":\"boom\"} from https://idp.example/token";
        let message = sign_in_failure_message(raw);
        hides_dump(raw);
        assert!(!message.contains("boom"), "{message}");
        assert!(!message.contains("server_error"), "{message}");
        assert_eq!(
            message,
            "The sign-in service is unavailable. Wait a minute and run `grok login` again."
        );
    }

    #[test]
    fn sign_in_failure_message_replaces_a_rejected_grant() {
        let raw = "OIDC token exchange failed: HTTP 401 — {\"error\":\"invalid_grant\"}";
        hides_dump(raw);
        assert_eq!(
            sign_in_failure_message(raw),
            "The sign-in server rejected this attempt. Run `grok login` to try again."
        );
    }

    #[test]
    fn sign_in_failure_message_replaces_a_connection_dump() {
        let raw =
            "error sending request for url (https://auth.example/oauth2/token): connection refused";
        hides_dump(raw);
        assert_eq!(
            sign_in_failure_message(raw),
            "Couldn't reach the sign-in service. Check your network and run `grok login` again."
        );
    }

    #[test]
    fn sign_in_failure_message_keeps_a_plain_sentence_that_already_has_a_next_step() {
        let timeout = "Login timed out after 10 minutes. Please try again.";
        let expired = "Your session has expired. Run `grok login` to sign in again.";
        let api_key = "Set XAI_API_KEY or add api_key/env_key to config.toml.";
        let device = "Device code expired. Run `grok login --device-auth` again.";
        let withheld = "This build has no account to sign in to or out of: it uses only the API key it was given.";
        for sentence in [timeout, expired, api_key, device, withheld] {
            assert_eq!(sign_in_failure_message(sentence), sentence);
        }
        let chained = format!(
            "{timeout}\n\nCaused by:\n    error sending request for url (https://idp.example/token)"
        );
        assert_eq!(sign_in_failure_message(&chained), timeout);
    }

    #[test]
    fn sign_in_failure_message_reads_a_quoted_acp_detail() {
        let shown =
            "Authentication required: \"Set XAI_API_KEY or add api_key/env_key to config.toml.\"";
        assert_eq!(
            sign_in_failure_message(shown),
            "Set XAI_API_KEY or add api_key/env_key to config.toml."
        );
    }

    #[test]
    fn sign_in_failure_message_adds_a_next_step_to_a_cancel() {
        assert_eq!(
            sign_in_failure_message("Authentication cancelled"),
            "Sign-in was cancelled. Run `grok login` to try again."
        );
    }

    #[test]
    fn sign_in_failure_message_hides_preferred_method_jargon() {
        let raw =
            "Authentication required: \"preferred_method=oidc; API-key auth is not allowed.\"";
        let message = sign_in_failure_message(raw);
        assert!(!message.contains("preferred_method"), "{message}");
        assert_eq!(
            message,
            "API-key sign-in isn't available. Set XAI_API_KEY, or run `grok login`."
        );
    }

    #[test]
    fn sign_in_failure_message_is_idempotent() {
        let raw = "OIDC discovery failed: HTTP 503 from https://idp.example/.well-known/openid-configuration";
        let once = sign_in_failure_message(raw);
        assert_eq!(sign_in_failure_message(&once), once);
        hides_dump(raw);
    }

    #[test]
    fn sign_in_failure_anyhow_drops_the_raw_chain() {
        let err = anyhow::anyhow!(
            "OIDC token exchange failed: HTTP 500 — {{\"error\":\"boom\"}} https://idp.example/token"
        );
        let shown = sign_in_failure_anyhow(err);
        let text = format!("{shown:#}");
        assert!(!text.contains("https://"), "{text}");
        assert!(!text.contains("boom"), "{text}");
        assert!(!text.contains("OIDC"), "{text}");
        assert!(text.contains("grok login"), "{text}");
    }
}

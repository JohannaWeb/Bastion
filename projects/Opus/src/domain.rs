use crate::crypto;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Capability {
    ReadWorkspace,
    WriteWorkspace,
    RunTerminalCommand,
    NetworkAccess,
    ReviewOnly,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Capability::ReadWorkspace => "workspace.read",
            Capability::WriteWorkspace => "workspace.write",
            Capability::RunTerminalCommand => "terminal.run",
            Capability::NetworkAccess => "network.access",
            Capability::ReviewOnly => "review.only",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityKind {
    Human,
    Agent,
    Organization,
    Plugin,
}

impl Display for IdentityKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            IdentityKind::Human => "human",
            IdentityKind::Agent => "agent",
            IdentityKind::Organization => "organization",
            IdentityKind::Plugin => "plugin",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub did: String,
    pub label: String,
    pub kind: IdentityKind,
    pub issuer: Option<String>,
    pub default_capabilities: BTreeSet<Capability>,
}

impl Identity {
    pub fn new(
        did: impl Into<String>,
        label: impl Into<String>,
        kind: IdentityKind,
        default_capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            did: did.into(),
            label: label.into(),
            kind,
            issuer: None,
            default_capabilities: default_capabilities.into_iter().collect(),
        }
    }

    pub fn issued_by(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct ActionRequest {
    pub actor_did: String,
    pub summary: String,
    pub justification: String,
    pub requested_capabilities: BTreeSet<Capability>,
    pub target: String,
}

impl ActionRequest {
    pub fn new(
        actor_did: impl Into<String>,
        summary: impl Into<String>,
        justification: impl Into<String>,
        target: impl Into<String>,
        requested_capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            actor_did: actor_did.into(),
            summary: summary.into(),
            justification: justification.into(),
            requested_capabilities: requested_capabilities.into_iter().collect(),
            target: target.into(),
        }
    }

    pub fn contract_text(&self) -> String {
        let capabilities = self
            .requested_capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "actor={} target={} summary={} justification={} capabilities=[{}]",
            self.actor_did, self.target, self.summary, self.justification, capabilities
        )
    }
}

#[derive(Clone, Debug)]
pub enum PolicyDecision {
    Allow,
    RequireApproval(String),
    Deny(String),
}

impl Display for PolicyDecision {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyDecision::Allow => f.write_str("allow"),
            PolicyDecision::RequireApproval(reason) => {
                write!(f, "require-approval: {reason}")
            }
            PolicyDecision::Deny(reason) => write!(f, "deny: {reason}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Policy {
    pub owner_did: String,
    pub auto_allow: BTreeSet<Capability>,
    pub approval_required: BTreeSet<Capability>,
    pub deny: BTreeSet<Capability>,
}

impl Policy {
    pub fn new(owner_did: impl Into<String>) -> Self {
        Self {
            owner_did: owner_did.into(),
            auto_allow: BTreeSet::new(),
            approval_required: BTreeSet::new(),
            deny: BTreeSet::new(),
        }
    }

    pub fn evaluate(&self, identity: &Identity, request: &ActionRequest) -> PolicyDecision {
        if request
            .requested_capabilities
            .iter()
            .any(|cap| self.deny.contains(cap))
        {
            let blocked = request
                .requested_capabilities
                .iter()
                .find(|cap| self.deny.contains(*cap))
                .map(ToString::to_string)
                .unwrap_or_else(|| "unknown".to_string());
            return PolicyDecision::Deny(format!("{blocked} is blocked by policy"));
        }

        if request
            .requested_capabilities
            .iter()
            .any(|cap| !identity.default_capabilities.contains(cap))
        {
            return PolicyDecision::Deny("requested capability exceeds actor grant".to_string());
        }

        if request
            .requested_capabilities
            .iter()
            .any(|cap| self.approval_required.contains(cap))
        {
            let caps = request
                .requested_capabilities
                .iter()
                .filter(|cap| self.approval_required.contains(*cap))
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            return PolicyDecision::RequireApproval(format!("{caps} needs human sign-off"));
        }

        if request
            .requested_capabilities
            .iter()
            .all(|cap| self.auto_allow.contains(cap))
        {
            return PolicyDecision::Allow;
        }

        PolicyDecision::RequireApproval("capability not covered by auto-allow list".to_string())
    }
}

#[derive(Clone, Debug)]
pub struct SignedEvent {
    pub actor_did: String,
    pub event_type: String,
    pub payload: String,
    pub signature: String,
}

impl SignedEvent {
    pub fn new(
        actor_did: impl Into<String>,
        event_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        let actor_did = actor_did.into();
        let event_type = event_type.into();
        let payload = payload.into();
        let signature = crypto::sign(&actor_did, &format!("{event_type}:{payload}"));
        Self {
            actor_did,
            event_type,
            payload,
            signature,
        }
    }

    pub fn verifies(&self) -> bool {
        crypto::verify(
            &self.actor_did,
            &format!("{}:{}", self.event_type, self.payload),
            &self.signature,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Ledger {
    events: Vec<SignedEvent>,
}

impl Ledger {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: SignedEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[SignedEvent] {
        &self.events
    }
}

#[derive(Clone, Debug)]
pub struct TrustGraph {
    identities: BTreeMap<String, Identity>,
}

impl TrustGraph {
    pub fn new() -> Self {
        Self {
            identities: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, identity: Identity) {
        self.identities.insert(identity.did.clone(), identity);
    }

    pub fn get(&self, did: &str) -> Option<&Identity> {
        self.identities.get(did)
    }

    pub fn identities(&self) -> impl Iterator<Item = &Identity> {
        self.identities.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_capability_outside_grant() {
        let identity = Identity::new(
            "did:agent:test",
            "test",
            IdentityKind::Agent,
            [Capability::ReadWorkspace],
        );
        let policy = Policy::new("did:human:owner");
        let request = ActionRequest::new(
            &identity.did,
            "modify file",
            "need to patch code",
            "src/main.rs",
            [Capability::WriteWorkspace],
        );
        assert!(matches!(
            policy.evaluate(&identity, &request),
            PolicyDecision::Deny(_)
        ));
    }

    #[test]
    fn event_signature_verifies() {
        let event = SignedEvent::new("did:agent:test", "action.executed", "patch=1");
        assert!(event.verifies());
    }
}

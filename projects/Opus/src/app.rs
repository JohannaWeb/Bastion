use crate::domain::{
    ActionRequest, Capability, Identity, IdentityKind, Ledger, Policy, PolicyDecision, SignedEvent,
    TrustGraph,
};
use serde::Serialize;
use std::fmt::Write;

#[derive(Clone, Debug, Serialize)]
pub struct IdentityView {
    pub did: String,
    pub label: String,
    pub kind: String,
    pub issuer: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyView {
    pub owner_did: String,
    pub auto_allow: Vec<String>,
    pub approval_required: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventView {
    pub index: usize,
    pub actor_did: String,
    pub event_type: String,
    pub payload: String,
    pub signature: String,
    pub verifies: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionCatalogEntry {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub default_human_approval: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeSnapshot {
    pub owner_did: String,
    pub identities: Vec<IdentityView>,
    pub policy: PolicyView,
    pub actions: Vec<ActionCatalogEntry>,
    pub ledger: Vec<EventView>,
}

pub struct IdeRuntime {
    trust_graph: TrustGraph,
    policy: Policy,
    ledger: Ledger,
    owner_did: String,
    agent_did: String,
}

impl IdeRuntime {
    pub fn seeded_demo() -> Self {
        let owner_did = "did:key:johanna".to_string();
        let agent_did = "did:agent:opus-builder".to_string();
        let org_did = "did:org:opus".to_string();
        let plugin_did = "did:plugin:terminal".to_string();

        let owner = Identity::new(
            &owner_did,
            "Johanna",
            IdentityKind::Human,
            [
                Capability::ReadWorkspace,
                Capability::WriteWorkspace,
                Capability::RunTerminalCommand,
                Capability::NetworkAccess,
            ],
        )
        .issued_by(&org_did);

        let agent = Identity::new(
            &agent_did,
            "Opus Builder",
            IdentityKind::Agent,
            [
                Capability::ReadWorkspace,
                Capability::WriteWorkspace,
                Capability::RunTerminalCommand,
            ],
        )
        .issued_by(&org_did);

        let org = Identity::new(&org_did, "Opus", IdentityKind::Organization, []);

        let plugin = Identity::new(
            &plugin_did,
            "Terminal Runner",
            IdentityKind::Plugin,
            [Capability::RunTerminalCommand],
        )
        .issued_by(&org_did);

        let mut trust_graph = TrustGraph::new();
        trust_graph.insert(owner);
        trust_graph.insert(agent);
        trust_graph.insert(org);
        trust_graph.insert(plugin);

        let mut policy = Policy::new(&owner_did);
        policy.auto_allow.insert(Capability::ReadWorkspace);
        policy.auto_allow.insert(Capability::ReviewOnly);
        policy.approval_required.insert(Capability::WriteWorkspace);
        policy
            .approval_required
            .insert(Capability::RunTerminalCommand);
        policy.deny.insert(Capability::NetworkAccess);

        Self {
            trust_graph,
            policy,
            ledger: Ledger::new(),
            owner_did,
            agent_did,
        }
    }

    pub fn render_overview(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "Opus: sovereign AI IDE prototype");
        let _ = writeln!(out, "Owner DID: {}", self.owner_did);
        let _ = writeln!(out, "Policy owner: {}", self.policy.owner_did);
        let _ = writeln!(out, "Known identities:");

        for identity in self.trust_graph.identities() {
            let caps = identity
                .default_capabilities
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let issuer = identity.issuer.as_deref().unwrap_or("self");
            let _ = writeln!(
                out,
                "  - {} [{}] issuer={} caps={}",
                identity.label, identity.kind, issuer, caps
            );
        }

        out
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            owner_did: self.owner_did.clone(),
            identities: self
                .trust_graph
                .identities()
                .map(|identity| IdentityView {
                    did: identity.did.clone(),
                    label: identity.label.clone(),
                    kind: identity.kind.to_string(),
                    issuer: identity
                        .issuer
                        .clone()
                        .unwrap_or_else(|| "self".to_string()),
                    capabilities: identity
                        .default_capabilities
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                })
                .collect(),
            policy: PolicyView {
                owner_did: self.policy.owner_did.clone(),
                auto_allow: self
                    .policy
                    .auto_allow
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                approval_required: self
                    .policy
                    .approval_required
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                deny: self.policy.deny.iter().map(ToString::to_string).collect(),
            },
            actions: self.action_catalog(),
            ledger: self
                .ledger
                .events()
                .iter()
                .enumerate()
                .map(|(index, event)| EventView {
                    index: index + 1,
                    actor_did: event.actor_did.clone(),
                    event_type: event.event_type.clone(),
                    payload: event.payload.clone(),
                    signature: event.signature.clone(),
                    verifies: event.verifies(),
                })
                .collect(),
        }
    }

    pub fn reset(&mut self) {
        self.ledger = Ledger::new();
    }

    pub fn action_catalog(&self) -> Vec<ActionCatalogEntry> {
        vec![
            ActionCatalogEntry {
                id: "inspect".to_string(),
                label: "Inspect Workspace".to_string(),
                summary: "Agent reads local Rust source. Auto-allowed.".to_string(),
                default_human_approval: false,
            },
            ActionCatalogEntry {
                id: "patch".to_string(),
                label: "Apply Patch".to_string(),
                summary: "Agent requests workspace.write to modify code.".to_string(),
                default_human_approval: true,
            },
            ActionCatalogEntry {
                id: "test".to_string(),
                label: "Run Cargo Test".to_string(),
                summary: "Agent requests terminal.run for verification.".to_string(),
                default_human_approval: true,
            },
            ActionCatalogEntry {
                id: "network".to_string(),
                label: "Fetch Remote Model".to_string(),
                summary: "Agent requests network access and is denied by policy.".to_string(),
                default_human_approval: false,
            },
        ]
    }

    pub fn execute_catalog_action(
        &mut self,
        action_id: &str,
        human_approves: bool,
    ) -> Result<String, String> {
        let request = match action_id {
            "inspect" => ActionRequest::new(
                &self.agent_did,
                "inspect Rust source",
                "need context before proposing a patch",
                "src/main.rs",
                [Capability::ReadWorkspace],
            ),
            "patch" => ActionRequest::new(
                &self.agent_did,
                "apply patch",
                "fix diagnostics in src/main.rs",
                "src/main.rs",
                [Capability::WriteWorkspace],
            ),
            "test" => ActionRequest::new(
                &self.agent_did,
                "run cargo test",
                "verify patch and ledger semantics",
                "cargo test",
                [Capability::RunTerminalCommand],
            ),
            "network" => ActionRequest::new(
                &self.agent_did,
                "download model",
                "attempt to fetch a remote foundation model",
                "registry://models/latest",
                [Capability::NetworkAccess],
            ),
            other => return Err(format!("unknown action {other}")),
        };

        let summary = format!("{} -> {}", request.summary, request.target);
        self.process_request(request, human_approves)?;
        Ok(summary)
    }

    pub fn simulate_demo_session(&mut self) -> Result<String, String> {
        self.execute_catalog_action("inspect", false)?;
        self.execute_catalog_action("patch", true)?;
        self.execute_catalog_action("test", true)?;
        self.execute_catalog_action("network", false)?;

        self.render_ledger_report()
    }

    fn process_request(
        &mut self,
        request: ActionRequest,
        human_approves: bool,
    ) -> Result<(), String> {
        let actor = self
            .trust_graph
            .get(&request.actor_did)
            .ok_or_else(|| format!("unknown actor {}", request.actor_did))?;

        let contract = request.contract_text();
        self.ledger.record(SignedEvent::new(
            &request.actor_did,
            "action.requested",
            contract.clone(),
        ));

        match self.policy.evaluate(actor, &request) {
            PolicyDecision::Allow => {
                self.ledger.record(SignedEvent::new(
                    &request.actor_did,
                    "action.executed",
                    contract,
                ));
            }
            PolicyDecision::RequireApproval(reason) => {
                self.ledger.record(SignedEvent::new(
                    &self.owner_did,
                    "approval.requested",
                    format!("{contract} reason={reason}"),
                ));

                if human_approves {
                    self.ledger.record(SignedEvent::new(
                        &self.owner_did,
                        "approval.granted",
                        request.actor_did.clone(),
                    ));
                    self.ledger.record(SignedEvent::new(
                        &request.actor_did,
                        "action.executed",
                        contract,
                    ));
                } else {
                    self.ledger.record(SignedEvent::new(
                        &self.owner_did,
                        "approval.rejected",
                        request.actor_did.clone(),
                    ));
                }
            }
            PolicyDecision::Deny(reason) => {
                self.ledger.record(SignedEvent::new(
                    &self.owner_did,
                    "action.denied",
                    format!("{contract} reason={reason}"),
                ));
            }
        }

        Ok(())
    }

    fn render_ledger_report(&self) -> Result<String, String> {
        let mut out = String::new();
        let _ = writeln!(out, "Session ledger:");

        for (index, event) in self.ledger.events().iter().enumerate() {
            if !event.verifies() {
                return Err(format!("ledger signature failed for event {}", index + 1));
            }

            let _ = writeln!(
                out,
                "  {}. {} by {} [{}]",
                index + 1,
                event.event_type,
                event.actor_did,
                event.signature
            );
            let _ = writeln!(out, "     {}", event.payload);
        }

        Ok(out)
    }
}

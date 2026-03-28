const invoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke ||
  (() => Promise.reject(new Error("Tauri invoke bridge is unavailable")));

const ownerDid = document.querySelector("#owner-did");
const identitiesRoot = document.querySelector("#identities");
const policyRoot = document.querySelector("#policy");
const actionsRoot = document.querySelector("#actions");
const ledgerRoot = document.querySelector("#ledger");
const ledgerSummary = document.querySelector("#ledger-summary");
const refreshButton = document.querySelector("#refresh");
const resetButton = document.querySelector("#reset");
const refreshWorkspaceButton = document.querySelector("#refresh-workspace");
const workspaceTreeRoot = document.querySelector("#workspace-tree");
const editorTabs = document.querySelector("#editor-tabs");
const editorState = document.querySelector("#editor-state");
const editorView = document.querySelector("#editor-view");
const editorTitle = document.querySelector("#editor-title");
const editorPath = document.querySelector("#editor-path");
const editorLines = document.querySelector("#editor-lines");
const editorContent = document.querySelector("#editor-content");

const identityTemplate = document.querySelector("#identity-template");
const actionTemplate = document.querySelector("#action-template");
const ledgerTemplate = document.querySelector("#ledger-template");

let workspaceTree = [];
let openTabs = [];
let activeTabPath = null;

async function loadSnapshot() {
  const snapshot = await invoke("get_snapshot");
  renderSnapshot(snapshot);
}

async function loadWorkspaceTree() {
  workspaceTreeRoot.textContent = "Loading workspace...";
  workspaceTree = await invoke("get_workspace_tree");
  renderWorkspaceTree();
}

function renderSnapshot(snapshot) {
  ownerDid.textContent = `Policy owner: ${snapshot.owner_did}`;

  identitiesRoot.replaceChildren(
    ...snapshot.identities.map((identity) => {
      const node = identityTemplate.content.firstElementChild.cloneNode(true);
      node.querySelector("[data-label]").textContent = identity.label;
      node.querySelector("[data-kind]").textContent = identity.kind;
      node.querySelector("[data-did]").textContent = identity.did;
      node.querySelector("[data-issuer]").textContent = `Issuer: ${identity.issuer}`;

      const caps = node.querySelector("[data-caps]");
      identity.capabilities.forEach((capability) => {
        const tag = document.createElement("span");
        tag.className = "tag";
        tag.textContent = capability;
        caps.append(tag);
      });

      return node;
    })
  );

  renderPolicy(snapshot.policy);
  renderActions(snapshot.actions);
  renderLedger(snapshot.ledger);
}

function renderWorkspaceTree() {
  if (!workspaceTree.length) {
    workspaceTreeRoot.textContent = "Workspace is empty.";
    return;
  }

  workspaceTreeRoot.replaceChildren(...workspaceTree.map(renderWorkspaceNode));
}

function renderWorkspaceNode(node) {
  if (node.kind === "directory") {
    const details = document.createElement("details");
    details.className = "tree-directory";
    details.open = node.path.split("/").length <= 1;

    const summary = document.createElement("summary");
    summary.textContent = node.name;
    details.append(summary, ...node.children.map(renderWorkspaceNode));
    return details;
  }

  const button = document.createElement("button");
  button.type = "button";
  button.className = "tree-file";
  button.textContent = node.name;
  if (node.path === activeTabPath) {
    button.classList.add("active");
  }

  button.addEventListener("click", () => {
    void openFile(node.path);
  });
  return button;
}

async function openFile(path) {
  const existing = openTabs.find((tab) => tab.path === path);
  if (existing) {
    activeTabPath = path;
    renderTabs();
    renderEditor();
    renderWorkspaceTree();
    return;
  }

  editorState.hidden = false;
  editorView.hidden = true;
  editorState.textContent = `Opening ${path}...`;

  try {
    const file = await invoke("open_workspace_file", { path });
    openTabs = [...openTabs, file];
    activeTabPath = file.path;
    renderTabs();
    renderEditor();
    renderWorkspaceTree();
  } catch (error) {
    editorState.hidden = false;
    editorView.hidden = true;
    editorState.textContent = String(error);
  }
}

function closeTab(path) {
  openTabs = openTabs.filter((tab) => tab.path !== path);
  if (activeTabPath === path) {
    activeTabPath = openTabs.at(-1)?.path ?? null;
  }
  renderTabs();
  renderEditor();
  renderWorkspaceTree();
}

function renderTabs() {
  if (!openTabs.length) {
    editorTabs.replaceChildren();
    return;
  }

  editorTabs.replaceChildren(
    ...openTabs.map((tab) => {
      const tabButton = document.createElement("button");
      tabButton.type = "button";
      tabButton.className = "editor-tab";
      if (tab.path === activeTabPath) {
        tabButton.classList.add("active");
      }

      const label = document.createElement("span");
      label.textContent = tab.name;

      const close = document.createElement("span");
      close.className = "editor-tab-close";
      close.textContent = "×";
      close.addEventListener("click", (event) => {
        event.stopPropagation();
        closeTab(tab.path);
      });

      tabButton.append(label, close);
      tabButton.addEventListener("click", () => {
        activeTabPath = tab.path;
        renderTabs();
        renderEditor();
        renderWorkspaceTree();
      });

      return tabButton;
    })
  );
}

function renderEditor() {
  const activeTab = openTabs.find((tab) => tab.path === activeTabPath) ?? null;
  if (!activeTab) {
    editorState.hidden = false;
    editorView.hidden = true;
    editorState.textContent = "Select a file from the workspace to open it in the editor.";
    return;
  }

  editorState.hidden = true;
  editorView.hidden = false;
  editorTitle.textContent = activeTab.name;
  editorPath.textContent = activeTab.path;
  editorLines.textContent = `${activeTab.lineCount} lines`;
  editorContent.textContent = activeTab.contents;
}

function renderPolicy(policy) {
  const groups = [
    ["Auto-Allow", policy.auto_allow],
    ["Approval Required", policy.approval_required],
    ["Denied", policy.deny],
  ];

  policyRoot.replaceChildren(
    ...groups.map(([title, items]) => {
      const card = document.createElement("article");
      card.className = "policy-card";
      const heading = document.createElement("h3");
      heading.textContent = title;
      const list = document.createElement("ul");
      items.forEach((item) => {
        const li = document.createElement("li");
        li.textContent = item;
        list.append(li);
      });
      card.append(heading, list);
      return card;
    })
  );
}

function renderActions(actions) {
  actionsRoot.replaceChildren(
    ...actions.map((action) => {
      const node = actionTemplate.content.firstElementChild.cloneNode(true);
      node.querySelector("[data-label]").textContent = action.label;
      node.querySelector("[data-mode]").textContent = action.default_human_approval
        ? "approval path"
        : "direct path";
      node.querySelector("[data-summary]").textContent = action.summary;

      const approval = node.querySelector("[data-approval]");
      approval.checked = action.default_human_approval;
      const runButton = node.querySelector("[data-run]");
      runButton.addEventListener("click", async () => {
        runButton.disabled = true;
        try {
          const snapshot = await invoke("run_action", {
            actionId: action.id,
            humanApproves: approval.checked,
          });
          renderSnapshot(snapshot);
        } catch (error) {
          window.alert(String(error));
        } finally {
          runButton.disabled = false;
        }
      });

      return node;
    })
  );
}

function renderLedger(events) {
  ledgerSummary.textContent = `${events.length} signed events captured`;
  ledgerRoot.replaceChildren(
    ...events.map((event) => {
      const node = ledgerTemplate.content.firstElementChild.cloneNode(true);
      node.querySelector("[data-type]").textContent = event.event_type;
      node.querySelector("[data-index]").textContent = `#${event.index}`;
      node.querySelector("[data-actor]").textContent = event.actor_did;
      node.querySelector("[data-payload]").textContent = event.payload;
      node.querySelector("[data-signature]").textContent = event.signature;

      const verify = node.querySelector("[data-verify]");
      verify.textContent = event.verifies ? "signature verified" : "signature failed";
      verify.className = event.verifies ? "verified" : "failed";

      return node;
    })
  );
}

refreshButton.addEventListener("click", loadSnapshot);
resetButton.addEventListener("click", async () => {
  const snapshot = await invoke("reset_runtime");
  renderSnapshot(snapshot);
});
refreshWorkspaceButton.addEventListener("click", () => {
  void loadWorkspaceTree();
});

Promise.all([loadSnapshot(), loadWorkspaceTree()]).catch((error) => {
  ledgerSummary.textContent = "frontend failed to connect to Tauri runtime";
  ledgerRoot.textContent = String(error);
  workspaceTreeRoot.textContent = String(error);
});

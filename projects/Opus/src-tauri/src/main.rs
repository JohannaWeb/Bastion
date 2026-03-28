use opus::app::IdeRuntime;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct RuntimeState {
    runtime: Mutex<IdeRuntime>,
    workspace_root: PathBuf,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceNode {
    name: String,
    path: String,
    kind: WorkspaceNodeKind,
    children: Vec<WorkspaceNode>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum WorkspaceNodeKind {
    Directory,
    File,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceFile {
    path: String,
    name: String,
    contents: String,
    line_count: usize,
}

#[tauri::command]
fn get_snapshot(
    state: tauri::State<'_, RuntimeState>,
) -> Result<opus::app::RuntimeSnapshot, String> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime mutex poisoned".to_string())?;
    Ok(runtime.snapshot())
}

#[tauri::command]
fn reset_runtime(
    state: tauri::State<'_, RuntimeState>,
) -> Result<opus::app::RuntimeSnapshot, String> {
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime mutex poisoned".to_string())?;
    runtime.reset();
    Ok(runtime.snapshot())
}

#[tauri::command]
fn run_action(
    action_id: String,
    human_approves: bool,
    state: tauri::State<'_, RuntimeState>,
) -> Result<opus::app::RuntimeSnapshot, String> {
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime mutex poisoned".to_string())?;
    runtime.execute_catalog_action(&action_id, human_approves)?;
    Ok(runtime.snapshot())
}

#[tauri::command]
fn get_workspace_tree(state: tauri::State<'_, RuntimeState>) -> Result<Vec<WorkspaceNode>, String> {
    build_workspace_tree(&state.workspace_root)
}

#[tauri::command]
fn open_workspace_file(
    path: String,
    state: tauri::State<'_, RuntimeState>,
) -> Result<WorkspaceFile, String> {
    let candidate = state.workspace_root.join(&path);
    let resolved = candidate
        .canonicalize()
        .map_err(|err| format!("failed to resolve file path: {err}"))?;

    ensure_in_workspace(&resolved, &state.workspace_root)?;

    if !resolved.is_file() {
        return Err("requested path is not a file".to_string());
    }

    let bytes = fs::read(&resolved).map_err(|err| format!("failed to read file: {err}"))?;
    let contents = String::from_utf8(bytes).map_err(|_| {
        "file is not valid UTF-8 text, so it cannot be opened in the built-in editor".to_string()
    })?;

    Ok(WorkspaceFile {
        path,
        name: resolved
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        line_count: contents.lines().count(),
        contents,
    })
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn build_workspace_tree(root: &Path) -> Result<Vec<WorkspaceNode>, String> {
    let mut nodes = Vec::new();
    let entries = fs::read_dir(root).map_err(|err| format!("failed to read workspace: {err}"))?;

    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to inspect workspace entry: {err}"))?;
        let path = entry.path();
        if should_skip(&path) {
            continue;
        }

        let node = build_node(&path, root)?;
        nodes.push(node);
    }

    nodes.sort_by(node_sort_key);
    Ok(nodes)
}

fn build_node(path: &Path, root: &Path) -> Result<WorkspaceNode, String> {
    let metadata = fs::metadata(path).map_err(|err| format!("failed to inspect entry: {err}"))?;
    let relative = path
        .strip_prefix(root)
        .map_err(|err| format!("failed to normalize path: {err}"))?
        .to_string_lossy()
        .replace('\\', "/");
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&relative)
        .to_string();

    if metadata.is_dir() {
        let mut children = Vec::new();
        let entries = fs::read_dir(path).map_err(|err| format!("failed to read directory: {err}"))?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("failed to inspect directory entry: {err}"))?;
            let child_path = entry.path();
            if should_skip(&child_path) {
                continue;
            }
            children.push(build_node(&child_path, root)?);
        }
        children.sort_by(node_sort_key);

        Ok(WorkspaceNode {
            name,
            path: relative,
            kind: WorkspaceNodeKind::Directory,
            children,
        })
    } else {
        Ok(WorkspaceNode {
            name,
            path: relative,
            kind: WorkspaceNodeKind::File,
            children: Vec::new(),
        })
    }
}

fn ensure_in_workspace(path: &Path, workspace_root: &Path) -> Result<(), String> {
    let root = workspace_root
        .canonicalize()
        .map_err(|err| format!("failed to resolve workspace root: {err}"))?;
    if path.starts_with(&root) {
        Ok(())
    } else {
        Err("requested path is outside the workspace".to_string())
    }
}

fn should_skip(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| matches!(name, ".git" | "target" | "node_modules"))
        .unwrap_or(false)
}

fn node_sort_key(left: &WorkspaceNode, right: &WorkspaceNode) -> std::cmp::Ordering {
    match (&left.kind, &right.kind) {
        (WorkspaceNodeKind::Directory, WorkspaceNodeKind::File) => std::cmp::Ordering::Less,
        (WorkspaceNodeKind::File, WorkspaceNodeKind::Directory) => std::cmp::Ordering::Greater,
        _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(RuntimeState {
            runtime: Mutex::new(IdeRuntime::seeded_demo()),
            workspace_root: workspace_root(),
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            reset_runtime,
            run_action,
            get_workspace_tree,
            open_workspace_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to launch Opus desktop shell");
}

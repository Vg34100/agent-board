# Feature Pattern Template

Use this template when adding new capabilities. It standardizes the flow from UI → Tauri command → optional HTTP mapping → persistence → events.

## Pattern Steps

1) Define/extend a Tauri command (backend)
- Location: `src-tauri/src/lib.rs`
- Prefer typed args and return `Result<T, String>`; convert to/from `serde_json::Value` near the store layer.
- Register in `invoke_handler!`.

2) Map command in the HTTP router (LAN/browser)
- Location: `src-tauri/src/web.rs`
- Add a case in `POST /api/invoke` that extracts args and calls the Tauri command.
- Use helpers like `str_arg_from()` to support both camelCase and snake_case.

3) Call from the UI (Leptos)
- Use `invoke(cmd, serde_wasm_bindgen::to_value(&args)?)`.
- Update reactive state on success and handle errors visibly (console + UI as needed).

4) Persist if needed (Store)
- Use store commands in `lib.rs` to save data (`app.store("file.json")`).
- File conventions:
  - `projects.json` → `projects`
  - `tasks_{project_id}.json` → `tasks`
  - `agent_messages_{task_id}.json` → `messages`
  - `agent_processes.json` → `processes`
  - `agent_settings.json` → `settings`

5) Broadcast events (real‑time updates)
- Use `web::broadcast_to_http(event_name, payload)` for SSE events consumed by browser clients.
- UI listens via the shim (`AGENT_EVENT_LISTEN`) in `index.html`.

6) Log & guardrails
- Keep noisy logs behind `AGENT_BOARD_DEBUG=1`.
- Include error bubbling to surface issues in UI console and Rust logs.

## Checklist

- [ ] Backend command implemented in `lib.rs`
- [ ] Command registered in `invoke_handler!`
- [ ] HTTP mapping added to `/api/invoke` (if needed for browser clients)
- [ ] UI call wired with `invoke()` and state updates
- [ ] Persistence via store (if applicable)
- [ ] SSE events broadcast + UI listeners (if applicable)
- [ ] Docs updated (README, architecture, troubleshooting if OS‑specific)

---

## Worked Example: Agent Reply (Multi‑Turn Conversation)

Note: This UI is not yet implemented (currently tasks only send the initial message on start). Use this pattern to add a reply box so users can continue the conversation.

What already exists
- Backend:
  - `start_agent_process(app, task_id, task_title, task_description, worktree_path, profile)`
  - `send_agent_message(app, process_id, message, worktree_path)` → spawns a new process using the context of the given `process_id` and returns the new process id.
  - Message/process persistence, SSE events.
- HTTP mapping (web.rs): cases for `start_agent_process`, `send_agent_message`, `get_process_list`, `get_agent_messages`, etc.

What to add (UI)
1) Add an input + send button to the Task Sidebar (`src/components/task_sidebar.rs`).
- Keep a `message_input` signal and a `sending` guard.
- On send:
  - Require a current/last `process_id` for this task (the sidebar already derives a current id; update to ensure it’s set after start or after each reply).
  - Call `invoke("send_agent_message", { processId, message, worktreePath })`.
  - The command returns a NEW `process_id`; update `current_process_id` to this value.
  - Clear the input, and optionally optimistically append a “user” message to the UI before SSE updates arrive.
  - Persist messages via `save_task_agent_messages` after updates (the sidebar already demonstrates this pattern).

2) Ensure message stream refreshes
- The SSE event `agent_message_update` will arrive; merge messages for the current process and save.
- If needed, poll `get_agent_messages(process_id)` immediately after sending to avoid initial latency.

3) Edge cases
- If no prior process exists for the task (user closed the app), consider falling back to `start_agent_process` or showing a prompt to (re)start.
- `worktree_path` must still exist; disable send if it’s missing and show a hint to recreate it.

Minimal code sketch (UI wiring)
- Add to `TaskSidebar`:
  - Signals: `let (message_input, set_message_input) = signal(String::new()); let (is_sending, set_is_sending) = signal(false);`
  - Button handler:
    - Read `current_process_id` and `task.worktree_path`.
    - `invoke("send_agent_message", to_value(&json!({ processId, message: message_input, worktreePath }))?).await`
    - Update `current_process_id` with result string.
    - Clear input; keep `is_sending` guards.

Testing checklist
- Start a task, verify first agent messages appear.
- Enter a reply, verify:
  - `send_agent_message` returns a new `process_id` and UI updates it
  - New agent output streams in via SSE
  - Messages persist and rehydrate on reload
- Kill a process, verify UI state and persistence aren’t corrupted.


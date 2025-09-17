# Agent Events Reference

This guide explains the events emitted while agent processes run and how the UI consumes them.

## Events (SSE)

- `agent_message_update`
  - Emitted for every parsed message from the agent process
  - Payload
    - `process_id`: string
    - `task_id`: string
    - `message`: `AgentMessage` (id, sender, content, timestamp, message_type, metadata)

- `agent_process_status`
  - Emitted when a process transitions to `completed`, `failed`, or is `killed`
  - Payload
    - `process_id`: string
    - `status`: string (`completed` | `failed` | `killed`)

## Where It’s Implemented

- Broadcast: `src-tauri/src/web.rs::broadcast_to_http`
- Parsed messages and status transitions: `src-tauri/src/agent.rs`
- Frontend listener: `src/components/task_sidebar.rs`

## Debugging Tips

- Enable verbose logs: set `AGENT_BOARD_DEBUG=1`
- Check console for `SSE connection established` and event logs
- Inspect process list via command `get_process_list` and message history via `get_agent_messages`


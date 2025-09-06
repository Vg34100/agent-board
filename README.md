# Tauri + Leptos

This template should help get you started developing with Tauri and Leptos.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

# Agent Board - AI Kanban Management Tool

## Project Overview
A desktop application for managing AI coding agents (specifically Claude Code) through a kanban-style interface. Built with Rust (Leptos) frontend and integrated backend for git workflow automation.

## Core Concept
Replace traditional development workflow with AI-orchestrated tasks:
- Create coding tasks in kanban columns (TODO → IN PROGRESS → IN REVIEW → DONE)
- Each task spawns Claude Code in isolated git worktree
- Automated environment setup (venv, dependencies)
- Streamlined review and merge workflow
- System tray application for easy access

## Technical Architecture
$$$
Agent Board (Tauri + Leptos)
├── Frontend: Rust/Leptos kanban UI
├── Backend: Rust subprocess management
├── Git Integration: Worktree per task
├── Claude Code: Spawned processes with session management
└── Distribution: Single executable with system tray
$$$

## MVP Requirements

### Phase 1: Basic Infrastructure
- [ ] Tauri app with Leptos frontend running
- [ ] System tray integration (show/hide window)
- [ ] Basic kanban board with 4 columns (TODO, IN PROGRESS, IN REVIEW, DONE)
- [ ] Project creation (new or existing git repo)
- [ ] Task creation with title/description

### Phase 2: Git Integration
- [ ] Git worktree creation per task
- [ ] Branch management (feature/task-{id})
- [ ] Automated environment setup detection (requirements.txt → venv creation)
- [ ] Cleanup workflow (remove worktree after completion)

### Phase 3: Claude Code Integration
- [ ] Spawn Claude Code subprocess in task worktree
- [ ] Capture and display Claude Code session logs
- [ ] Real-time output streaming to UI
- [ ] Handle permission requests and user interactions
- [ ] Session persistence and recovery

### Phase 4: Review Workflow
- [ ] File diff viewer for task changes
- [ ] Manual testing options (open temp directory)
- [ ] Git merge functionality
- [ ] Pull request creation (GitHub API)
- [ ] Worktree cleanup after PR creation

### Phase 5: Polish
- [ ] Drag-and-drop task management between columns
- [ ] Task state persistence (SQLite database)
- [ ] Error handling and recovery
- [ ] Configuration management
- [ ] Performance optimization

## Key Features
- **Isolated Development**: Each task runs in separate git worktree
- **Automated Setup**: Auto-detect and create virtual environments
- **Real-time Monitoring**: Live Claude Code session streaming
- **Clean Testing**: Remove worktrees, test PRs in main repo
- **Native Feel**: System tray app, not just localhost port

## Success Criteria
MVP is complete when:
1. Can create projects and tasks through UI
2. Tasks automatically spawn Claude Code in worktrees
3. Can view live Claude Code output
4. Can create PRs and cleanup worktrees
5. Runs as system tray application

## Current Status
- [x] Tauri project scaffolded with Leptos
- [ ] Basic dependencies installed (Tauri CLI, Trunk)
- [ ] First kanban UI prototype

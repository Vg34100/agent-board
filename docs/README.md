# Agent Board Documentation

## 📚 Documentation Index

This documentation captures the complete development process, current status, and future roadmap for Agent Board - an AI Kanban management tool built with Tauri + Leptos.

### 📖 Documentation Files

1. **[immediate-next-steps.md](./immediate-next-steps.md)** - 🔥 START HERE
   - Critical TaskModal callback issue that blocks progress
   - Exact files to fix and approaches to try
   - Phase 1 tasks and success criteria

2. **[development-log.md](./development-log.md)** - Learning History
   - Complete development history and mistakes made
   - What's working vs what failed
   - Technical decisions and lessons learned

3. **[roadmap.md](./roadmap.md)** - Long-term Vision
   - Complete feature roadmap from MVP to production
   - Phase-by-phase development plan
   - Claude Code integration strategy

4. **[troubleshooting.md](./troubleshooting.md)** - Problem Solving
   - Common build errors and solutions
   - Hot reload issues and fixes
   - Component and CSS troubleshooting

5. **[store.md](./store.md)** - Data Persistence Guide ⭐ NEW
   - Complete Tauri store API implementation guide
   - What was broken vs what works now
   - Critical fix for project/task persistence

## 🚀 Current Status: FULLY FUNCTIONAL ✨

### ✅ Completed Features
- Tauri + Leptos app running successfully
- Projects page with navigation to kanban boards  
- 5-column kanban layout (ToDo → InProgress → InReview → Done → Cancelled)
- **Complete data persistence** - projects and tasks save between sessions
- **Working project creation** - create projects with git initialization
- **Working task creation** - add tasks that persist to storage
- **Project editing** - edit existing projects with pre-populated data
- **Task editing and management** - edit/delete/cancel tasks with dropdown menus
- Square, minimal, programmer-focused UI design
- Full-width layout with monospace fonts
- Context-based navigation system
- **Proper Tauri store API implementation** ⭐ MAJOR FIX

### 🎉 Major System Fix Complete
**Store API Modernization** - Fixed the critical persistence system that was completely broken. Projects now save, tasks persist between sessions, and the "Loading..." header issue is resolved. See [store.md](./store.md) for technical details.

### 🎯 Next Phase Goals
1. ✅ COMPLETED - Project persistence and task storage  
2. Add drag & drop for task status changes
3. Implement git worktree integration
4. Add system tray functionality
5. Implement agent chat interface

## 🛠️ Quick Start Commands

```bash
# Start development server
cargo tauri dev

# Check for compilation errors
cargo check

# Clean rebuild if needed
cargo clean && cargo tauri dev
```

## 📁 Project Structure

```
agent-board/
├── src/
│   ├── app.rs              # Main navigation logic
│   ├── main.rs             # Entry point
│   ├── models/             # Data structures (Project, Task, TaskStatus)
│   ├── pages/              # Main views (Projects, Kanban)
│   └── components/         # Reusable UI (TaskModal - broken)
├── src-tauri/              # Tauri backend configuration
├── docs/                   # This documentation
├── styles.css              # Global styles (square, minimal design)
└── target/                 # Build artifacts
```

## 🎨 Design Philosophy

**Square, Minimal, Programmer-Focused**
- Monospace fonts (Cascadia Code, Fira Code, JetBrains Mono)
- No rounded corners anywhere
- Full-width layout utilizing entire screen
- Dark theme with professional coding environment feel
- Minimal padding and tight spacing between elements

## 💡 Key Architectural Decisions

### ✅ What's Working
- **Context Navigation**: Enums + signals instead of complex routing
- **CSS-First Design**: Custom styles over component libraries
- **Component Separation**: Clear pages/ and components/ structure
- **Tauri Integration**: Desktop app with web frontend

### ❌ Lessons Learned
- **Avoid leptos_router complexity**: Simple context navigation works better
- **Component callbacks are tricky**: Need simpler patterns for parent-child communication
- **Start simple**: Add complexity incrementally, not all at once

## 🔄 Development Workflow

1. **Read [immediate-next-steps.md](./immediate-next-steps.md)** for current priorities
2. **Check [troubleshooting.md](./troubleshooting.md)** if you hit build errors
3. **Update [development-log.md](./development-log.md)** with new learnings
4. **Reference [roadmap.md](./roadmap.md)** for long-term planning

## 📞 Emergency Recovery

If development gets completely stuck:

1. **Minimal Working State**: Comment out broken features, get basic app running
2. **Incremental Development**: Add one feature at a time, test each step  
3. **Reference Examples**: Look at official Leptos examples for working patterns
4. **Documentation**: All problems and solutions should be documented here

---

**This documentation is living** - update it as development progresses to maintain a complete development history and troubleshooting guide.
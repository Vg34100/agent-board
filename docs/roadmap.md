# Agent Board Development Roadmap

## Current Status (✅ MVP Complete)
- ✅ Basic Tauri + Leptos app running
- ✅ Projects page with navigation
- ✅ 5-column Kanban board (ToDo → InProgress → InReview → Done → Cancelled)
- ✅ Sample data displaying correctly
- ✅ Square, minimal, programmer-focused UI design
- ✅ Full-width layout utilizing entire screen space
- ✅ Monospace fonts (Cascadia Code, Fira Code, JetBrains Mono)
- ✅ Context-based navigation (simplified from router approach)

## Phase 1: Core Functionality (Next Immediate Priority)

### 🔥 Critical Issues to Fix First
1. **TaskModal Callback Types** (BLOCKING)
   - **Problem**: Component callback types not working (`Box<dyn Fn>` vs `Callback<T>`)
   - **Location**: `src/components/task_modal.rs` and `src/pages/kanban.rs`
   - **Research Needed**: Leptos component callback patterns
   - **References**: Check [Leptos examples](https://github.com/leptos-rs/leptos/tree/main/examples) for callback patterns

### 🎯 Phase 1 Tasks (Complete for Working App)
2. **Task Creation Modal** (High Priority)
   - Fix callback integration in TaskModal
   - Connect "+" button to modal
   - Form validation and submission
   - Add new tasks to ToDo column

3. **Project Creation Modal** (High Priority)
   - Create ProjectModal component
   - Connect "CREATE PROJECT" button
   - Handle both "New Project" and "Existing Repository" options
   - Form fields: name, git path (optional), setup/cleanup scripts

4. **Data Persistence** (High Priority)
   - Implement localStorage integration
   - Save/load projects and tasks
   - Auto-save on all changes
   - Data restoration on app startup

## Phase 2: Enhanced User Experience

5. **Task Management Features**
   - Inline editing of task titles/descriptions
   - Delete task functionality with confirmation
   - Move tasks between columns (dropdown/buttons initially)
   - Task timestamps and metadata display

6. **Project Management**
   - Edit existing projects
   - Delete projects with confirmation
   - Project statistics (task counts, last activity)
   - Project settings/configuration

7. **UI Polish**
   - Loading states for all operations
   - Error handling and user feedback
   - Keyboard shortcuts (Esc to close modals, etc.)
   - Empty state messages for projects/tasks

## Phase 3: Tauri Backend Integration

8. **File System Operations**
   - Create project directories
   - Copy template files
   - Read/write configuration files
   - Directory picker for existing repositories

9. **Git Integration Foundation**
   - Initialize git repositories for new projects
   - Validate existing git repositories
   - Basic git status checking
   - Prepare for worktree integration (Phase 4)

10. **Process Management**
    - Execute setup/cleanup scripts
    - Environment detection (Python venv, Node.js, etc.)
    - Background process monitoring

## Phase 4: Advanced Features (Claude Code Integration)

11. **Git Worktree Management**
    - Create worktrees for each task
    - Branch management (feature/task-{id})
    - Cleanup workflow after task completion

12. **Claude Code Integration**
    - Spawn Claude Code subprocess in task worktree
    - Capture and display session logs
    - Real-time output streaming to UI
    - Handle permission requests and interactions

13. **Review Workflow**
    - File diff viewer for task changes
    - Manual testing options (open directories)
    - Git merge functionality
    - Pull request creation (GitHub API integration)

## Phase 5: Production Polish

14. **System Integration**
    - System tray functionality
    - Auto-start with OS
    - Window state persistence
    - Native notifications

15. **Performance & Reliability**
    - Database integration (SQLite)
    - Drag-and-drop task management
    - Undo/redo operations
    - Data export/import

16. **Configuration & Settings**
    - User preferences
    - Theme customization
    - Keyboard shortcut configuration
    - Default project templates

## Technical Debt & Code Quality

### Immediate Cleanup (Should be done in Phase 1)
- **Unused Imports**: Clean up all warning messages
- **Error Handling**: Add proper error boundaries and user feedback
- **Type Safety**: Resolve all TypeScript-like type issues
- **Component Organization**: Split large components into smaller, focused ones

### Code Quality Improvements
- **Testing**: Add unit tests for components and utilities
- **Documentation**: Add inline code documentation
- **Performance**: Profile and optimize re-renders
- **Security**: Input validation and sanitization

## Dependencies & External Integrations

### Required Crates/Libraries
- ✅ `leptos`, `leptos_router` (already added)
- ✅ `tauri`, `tauri-plugin-fs` (already added)
- ✅ `uuid`, `chrono`, `serde_json` (already added)
- 🔲 `tokio` for async operations
- 🔲 `sqlite` for database (Phase 5)
- 🔲 `git2` for git operations (Phase 3)
- 🔲 `notify` for file watching (Phase 4)

### External APIs
- GitHub API for pull request creation
- Claude API for advanced integrations (future)

## Success Metrics

### Phase 1 Complete When:
- Can create and manage projects through UI
- Can add, edit, and move tasks between columns
- Data persists between app sessions
- No critical bugs or type errors

### Phase 2 Complete When:
- Full CRUD operations on projects and tasks
- Polished user experience with error handling
- Keyboard shortcuts and accessibility features

### Final MVP Complete When:
- Tasks automatically spawn Claude Code in worktrees
- Can view live Claude Code output
- Can create PRs and cleanup worktrees
- Runs as stable system tray application

## Getting Started (For Future Development)

1. **Setup**: `cargo tauri dev` to start development server
2. **Check Current Issues**: Review `docs/development-log.md` for recent problems
3. **Priority**: Start with TaskModal callback type resolution
4. **Testing**: Test each feature in both light and dark mode
5. **Documentation**: Update this roadmap as features are completed

## Emergency Recovery

If development gets stuck:
1. **Check Documentation**: Review `docs/troubleshooting.md`
2. **Minimal Working State**: Comment out broken features, get basic app running
3. **Incremental Development**: Add features one at a time, test each step
4. **Reference Implementation**: Look at official Leptos examples for patterns
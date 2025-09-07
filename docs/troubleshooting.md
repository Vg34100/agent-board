# Troubleshooting Guide

## Common Build Errors

### Error: `leptos_router` features not found
```
error: failed to select a version for `leptos_router`.
package `leptos_router` does not have feature `csr`
```
**Solution**: Remove feature flags from leptos_router:
```toml
leptos_router = { version = "0.7" }  # Remove features = ["csr"]
```

### Error: Component callback types
```
error[E0277]: the trait bound `Callback<String, String>: From<{closure}>` is not satisfied
```
**Current Status**: UNRESOLVED - Need to research Leptos component callback patterns
**Workaround**: Comment out complex component usage until resolved

### Error: `cargo tauri dev` not found
**Problem**: Trying to run `npm run tauri dev` in a Rust project
**Solution**: Use `cargo tauri dev` instead

### Error: Build pipeline failures
```
error from build pipeline
cargo call returned bad status: exit code 101
```
**Solution**: Fix all Rust compilation errors first, then restart dev server

## Hot Reload Issues

### Problem: Changes not reflecting
**Solution**: 
1. Kill the dev server (Ctrl+C)
2. Run `cargo check` to ensure no compilation errors
3. Restart with `cargo tauri dev`

### Problem: WASM build failures  
**Solution**: Check for:
- Incorrect import paths
- Missing feature flags in Cargo.toml
- Type mismatches in components

## CSS Issues

### Problem: Layout not full screen
**Solution**: Use these CSS properties:
```css
body {
  margin: 0;
  padding: 0;
  height: 100vh;
  overflow: hidden;
}

.app {
  height: 100vh;
  width: 100vw;
}
```

### Problem: Columns not equal width
**Solution**: Use flexbox:
```css
.kanban-board {
  display: flex;
  flex: 1;
}

.kanban-column {
  flex: 1;  /* Equal width columns */
}
```

## Component Issues

### Problem: Event handlers not working
**Check**:
- Correct event name (`on:click` not `onclick`)
- Handler function signature matches event type
- Component is properly mounted

### Problem: Signals not updating
**Check**:
- Using `set_signal.set(value)` not `set_signal(value)`
- Signal is used inside reactive context (`move ||`)
- No circular dependencies between signals

## Build Performance

### Problem: Slow compilation
**Solutions**:
- Use `cargo check` for fast syntax checking
- Enable incremental compilation (default in dev)
- Use `--release` flag only for final builds

## Project Persistence Issues

### Problem: Projects not saving/loading
**Symptoms**: 
- Projects disappear after navigation
- Kanban shows "PROJECT: LOADING..." permanently
- Empty project list despite creating projects

**Solutions**:
1. **Check Tauri command registration**: Ensure all commands are in `generate_handler![]`
2. **Verify parameter naming**: Frontend JSON must match backend function parameters exactly
3. **Check store permissions**: Tauri store plugin needs proper setup in `Cargo.toml`
4. **Debug with browser dev tools**: Check console for JS errors from Tauri calls

### Problem: Tasks not persisting per project
**Symptoms**:
- All projects show the same tasks
- Tasks disappear when switching projects
- Hardcoded sample data appears

**Solutions**:
1. **Verify file naming**: Should use `tasks_{project_id}.json` format
2. **Check task loading logic**: Load tasks on component mount with project-specific filename
3. **Ensure save callback**: TaskModal creation should trigger save operation

## When All Else Fails

1. **Clean rebuild**: `cargo clean && cargo tauri dev`
2. **Check Cargo.lock**: Delete and regenerate if dependency issues
3. **Minimal reproduction**: Comment out problematic code, add back gradually
4. **Check examples**: Look at official Leptos examples for patterns
5. **Read the error**: Rust error messages are usually helpful - read them carefully
6. **Verify Tauri setup**: Check `src-tauri/tauri.conf.json` for proper plugin configuration
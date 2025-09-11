# WASM Map Serialization Fix

## Problem Summary
Tasks created from browser (localhost) were being saved as empty objects `{}` instead of full task data, causing complete data loss. The HTTP argument serialization worked perfectly, but task objects were getting corrupted during JavaScript→JSON serialization.

## Root Cause Discovery
**WASM objects are JavaScript Maps, not regular objects.** When serde_wasm_bindgen serializes Rust structs to JavaScript, it creates Map objects, not plain JavaScript objects. `JSON.stringify()` cannot serialize Maps properly - it turns them into empty objects `{}`.

### Evidence from Logs:
```javascript
// Before fix:
✅ Task serialization SUCCESS: Object {"created_at": String("..."), ...}  // Rust serialization works
Normalized args: Object { projectId: "...", tasks: (1) […] }              // Array exists  
Args string: {"projectId":"...","tasks":[{}]}                             // JSON.stringify fails on Maps

// After fix:
🔥 DETECTED EMPTY OBJECTS IN JSON! Attempting direct serialization...
Processing task: Map(8) { created_at → "...", description → "A", ... }    // It's a Map!
🎯 FOUND THE PROBLEM: Task is a Map! Converting to Object...
✅ Manual serialization result: {"projectId":"...","tasks":[{"created_at":"...","description":"A",...}]}
```

## The Fix
Modified the JavaScript shim in `index.html` to detect when `JSON.stringify()` produces empty objects for non-empty arrays, then manually convert WASM Maps to plain JavaScript objects:

```javascript
// Detection
if (argsString.includes('[{}') && normalizedArgs.tasks && normalizedArgs.tasks.length > 0) {
  // Manual Map-to-Object conversion
  const manualTasks = normalizedArgs.tasks.map(task => {
    const obj = {};
    if (task instanceof Map) {
      for (const [key, value] of task.entries()) {
        obj[key] = value;
      }
    }
    return obj;
  });
  // Re-serialize with converted objects
  argsString = JSON.stringify({...normalizedArgs, tasks: manualTasks});
}
```

## Key Insights
1. **WASM-JS boundary creates Maps**: `serde_wasm_bindgen` serializes Rust structs as JavaScript Maps
2. **Maps don't JSON.stringify()**: `JSON.stringify(map)` returns `"{}"` instead of object data
3. **Detection is crucial**: Check for `[{}` pattern when arrays should contain data
4. **Manual conversion works**: `Map.entries()` → regular object → `JSON.stringify()` succeeds

## Files Modified
- `index.html`: Added Map detection and conversion in the JavaScript shim's `invoke` function

## Why Other Attempts Failed
- **Custom DateTime serialization**: Wrong layer - problem was in JavaScript, not Rust
- **Error handling improvements**: Good for debugging but didn't fix core issue  
- **String-based DateTime**: Helped with Rust serialization but JS still couldn't handle Maps
- **Enhanced logging**: Critical for discovering the Map issue, but not the fix itself

## Prevention
- Always test WASM struct serialization through the complete JS→HTTP pipeline
- Watch for empty objects `{}` in JSON strings when objects should have data
- Remember that WASM objects may not be regular JavaScript objects
- Use `instanceof Map` checks when dealing with WASM-generated data

## Result
✅ Tasks created from browser now persist correctly  
✅ No more data loss between desktop and browser views  
✅ HTTP argument serialization works perfectly for all operations  
✅ WASM Map objects properly converted to serializable JavaScript objects
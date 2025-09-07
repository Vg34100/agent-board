# Agent Board UI Style Guide

## Visual Philosophy
Agent Board follows a **programmer-centric aesthetic** designed for developers and technical users.

## Typography
- **Font Family**: `'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'SF Mono', Monaco, 'Consolas', monospace`
- **Font Size**: 14px base
- **Line Height**: 1.4
- **Weight**: 400 (normal), 500 (emphasis), 600 (headers)

## Color Palette
- **Background**: `#1a1a1a` (primary), `#2a2a2a` (secondary)
- **Text**: `#e0e0e0` (primary), `#aaa` (secondary), `#888` (muted)
- **Borders**: `#333` (primary), `#555` (hover)
- **Accent**: `#007acc` (primary), `#0099ff` (hover)
- **Actions**: `#28a745` (success), `#ff4444` (danger)

## Icon System
**NO EMOJIS** - Use simple ASCII/Unicode characters that maintain the programmer aesthetic:

### Navigation Icons
- **Folder**: `▶` or `[folder]`
- **Home**: `~` or `[home]` 
- **Up/Back**: `↑` or `◀`
- **Down**: `↓` or `▼`
- **Forward**: `▶` or `→`
- **Add/Create**: `+` or `[+]`
- **Close**: `×` or `[x]`

### Action Icons
- **Edit**: `[edit]` or `✎`
- **Delete**: `[x]` or `⌫`
- **Settings**: `[...]` or `⚙`
- **Search**: `[?]` or `🔍` (exception for clarity)

## Modal Design
- **Background**: Transparent with `rgba(0, 0, 0, 0.8)` backdrop
- **Content**: `#2a2a2a` background, `#333` border
- **Header**: `#1a1a1a` background, white text
- **Width**: 400px default, 600px for complex forms
- **No rounded corners** - keep rectangular/sharp edges

## Button Styles
- **Primary**: `#007acc` background, white text
- **Secondary**: `#444` background, `#e0e0e0` text
- **Border**: `1px solid #333`, no rounded corners
- **Padding**: `12px 16px`
- **Hover**: Slight color shift, no animations beyond 0.1s

## Form Elements
- **Inputs**: `#2a2a2a` background, `#333` border, white text
- **Labels**: White text, 14px, slightly above inputs
- **Help Text**: `#888` color, 12px, italic
- **Placeholders**: `#666` color

## Layout Principles
1. **Sharp edges** - no border-radius except where functionally necessary
2. **Consistent spacing** - 8px, 12px, 16px, 20px increments
3. **Minimal animations** - max 0.1s transitions
4. **High contrast** - ensure readability in dark theme
5. **Monospace consistency** - all text uses monospace fonts

## Directory Browser Specific
- **Fixed height**: 300px minimum for folder list
- **Scrollbar**: Always visible when content overflows
- **Navigation bar**: Home, Up, Current Path, Select buttons
- **Folder items**: Simple text with folder icon prefix
- **No fancy hover effects** - simple background color change

This guide ensures consistency across all UI components while maintaining the technical, no-nonsense aesthetic suitable for developer tools.
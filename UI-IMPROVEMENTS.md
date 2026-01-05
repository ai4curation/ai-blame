# UI Improvements Roadmap

## Overview

Based on comprehensive UI review and testing, this document outlines specific improvements needed to move the AI Blame UI from prototype to production-ready state.

## Quick Reference

### Component Status Legend
- ✅ **Fully Functional** - Working as intended
- ⚠️ **Partially Functional** - Works but has limitations
- ❌ **Stub/Not Implemented** - Placeholder only
- 🔧 **Needs Improvement** - Works but could be better

## Critical Issues (Must Fix Before Release)

### 1. Stub Components Need Implementation

#### Export HTML (Blame Viewer) ❌
**Current:** Shows "not implemented" message  
**Required:**
```rust
#[tauri::command]
async fn export_blame_html(
    project_dir: String,
    file_path: String,
    output_path: Option<String>
) -> Result<String, String>
```

**Frontend work:**
- Trigger save file dialog
- Stream HTML to disk
- Show success notification

**Priority:** Medium  
**Effort:** 4-6 hours

---

#### Save Config (Settings) ❌
**Current:** Shows "not implemented" message  
**Required:**
```rust
#[tauri::command]
async fn save_config(
    project_dir: String,
    config: Config
) -> Result<(), String>

#[tauri::command]
async fn load_config(
    project_dir: String
) -> Result<Config, String>
```

**Frontend work:**
- Validate form inputs
- Serialize config object
- Handle save errors
- Show save confirmation

**Priority:** High  
**Effort:** 6-8 hours

---

#### Add Rule (Settings) ❌
**Current:** Shows "not implemented" message  
**Required:**
- Rule input dialog/modal
- Pattern validation (glob)
- Policy selection
- Update rules list dynamically
- Reflect in YAML preview

**Priority:** High  
**Effort:** 4-6 hours

---

### 2. Real Data Integration

#### Trace Explorer Mock Data ⚠️
**Current:** Shows hardcoded sample traces  
**Required:**
```rust
#[tauri::command]
async fn list_traces(
    project_dir: String,
    filter: Option<TraceFilter>
) -> Result<Vec<TraceMetadata>, String>

#[derive(Serialize, Deserialize)]
struct TraceMetadata {
    id: String,
    timestamp: DateTime<Utc>,
    model: String,
    files_changed: Vec<String>,
    lines_added: usize,
    lines_modified: usize,
    lines_deleted: usize,
}
```

**Frontend work:**
- Parse trace .jsonl files
- Build trace list from real data
- Update summary statistics
- Make traces clickable
- Link to Blame Viewer

**Priority:** High  
**Effort:** 8-12 hours

---

#### Trace Detail View ❌
**Current:** Trace items are not clickable  
**Required:**
- Detail panel/modal when clicking trace
- Show full metadata
- Display file diffs
- Link to affected files in Blame Viewer
- Copy button for trace ID

**Priority:** Medium  
**Effort:** 6-8 hours

---

### 3. Loading & Error States

#### Loading Indicators 🔧
**Current:** No visual feedback during async operations  
**Required:**
- Spinner during file list load
- Skeleton screen for blame computation
- Progress bar for trace scanning
- Disable buttons during operations
- Timeout handling (show error after 30s)

**Priority:** High  
**Effort:** 4-6 hours

**Example CSS:**
```css
.spinner {
  border: 3px solid #f3f3f3;
  border-top: 3px solid #3498db;
  border-radius: 50%;
  width: 40px;
  height: 40px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
```

---

#### Error Handling 🔧
**Current:** Errors log to console, generic status message  
**Required:**
- User-friendly error dialogs
- Specific error messages
- "Try Again" button
- "Report Issue" button
- Stack trace in dev mode only
- Error categorization (user error vs system error)

**Priority:** High  
**Effort:** 6-8 hours

**Example dialog:**
```javascript
function showErrorDialog(title, message, actions) {
  const dialog = document.createElement('div');
  dialog.className = 'error-dialog';
  dialog.innerHTML = `
    <h3>${title}</h3>
    <p>${message}</p>
    <div class="dialog-actions">
      ${actions.map(a => `<button onclick="${a.handler}">${a.label}</button>`).join('')}
    </div>
  `;
  document.body.appendChild(dialog);
}
```

---

#### Empty States 🔧
**Current:** Shows generic messages or nothing  
**Required:**
- Helpful illustrations
- Actionable next steps
- Contextual guidance

**Examples:**
- **Blame Viewer (no project):** "Open a project to start exploring AI blame"
- **Blame Viewer (no traces):** "No traces found. Run your first AI coding session!"
- **Trace Explorer (no traces):** "No traces detected. Start coding with an AI assistant."

**Priority:** High  
**Effort:** 4-6 hours

---

## UX Improvements

### 4. Interactivity Enhancements

#### Keyboard Shortcuts 🔧
**Priority:** Medium  
**Effort:** 4-6 hours

**Shortcuts to add:**
```javascript
const shortcuts = {
  'cmd+o': () => openProjectPicker(),
  'cmd+r': () => refreshTraces(),
  'cmd+f': () => focusSearch(),
  'cmd+,': () => navigate('settings'),
  'cmd+1': () => navigate('home'),
  'cmd+2': () => navigate('blame'),
  'cmd+3': () => navigate('traces'),
  'cmd+4': () => navigate('settings'),
  'esc': () => closeDialog(),
};
```

**Implementation:**
```javascript
document.addEventListener('keydown', (e) => {
  const key = (e.metaKey || e.ctrlKey ? 'cmd+' : '') + e.key.toLowerCase();
  if (shortcuts[key]) {
    e.preventDefault();
    shortcuts[key]();
  }
});
```

---

#### File Search Improvements 🔧
**Current:** Basic substring match, case-sensitive  
**Priority:** Medium  
**Effort:** 2-4 hours

**Improvements:**
- Case-insensitive search
- Fuzzy matching (typo tolerance)
- Highlight matching text
- Search in file content (not just names)
- Recent searches dropdown

**Example:**
```javascript
function fuzzyMatch(str, pattern) {
  const p = pattern.toLowerCase();
  const s = str.toLowerCase();
  let pi = 0;
  for (let si = 0; si < s.length; si++) {
    if (s[si] === p[pi]) pi++;
    if (pi === p.length) return true;
  }
  return false;
}
```

---

#### Better Blame Details Panel 🔧
**Current:** Overlaps code, basic info  
**Priority:** Low  
**Effort:** 4-6 hours

**Improvements:**
- Slide-in animation from right
- Copy buttons for IDs
- Show full edit history for line
- Link to session trace
- Show related file changes in same session
- Resizable panel

---

### 5. Visual Polish

#### Animations & Transitions 🔧
**Priority:** Low  
**Effort:** 4-6 hours

**Add animations for:**
- View transitions (fade in/out)
- Button press feedback (scale)
- Dialog open/close (slide/fade)
- List item hover (subtle highlight)
- Status bar updates (slide in)

**Example CSS:**
```css
.view {
  opacity: 0;
  transition: opacity 0.2s ease-in-out;
}

.view.is-visible {
  opacity: 1;
}

button:active {
  transform: scale(0.98);
}
```

---

#### Toast Notifications 🔧
**Priority:** Medium  
**Effort:** 3-4 hours

**Replace status bar-only feedback with:**
- Toast notifications (top-right corner)
- Auto-dismiss after 3-5 seconds
- Color-coded (success=green, error=red, info=blue)
- Dismissible with X button
- Stack multiple toasts

**Example:**
```javascript
function showToast(message, type = 'info') {
  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  toast.textContent = message;
  document.body.appendChild(toast);
  
  setTimeout(() => toast.classList.add('show'), 10);
  setTimeout(() => {
    toast.classList.remove('show');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}
```

---

#### Dark Mode Support 🔧
**Priority:** Medium  
**Effort:** 6-8 hours

**Implementation:**
- CSS custom properties for colors
- Toggle button in header or settings
- Persist preference in localStorage
- Auto-detect system preference
- Smooth transition between modes

**Example:**
```css
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f5;
  --text-primary: #1a1a1a;
  --text-secondary: #666666;
}

:root[data-theme="dark"] {
  --bg-primary: #1a1a1a;
  --bg-secondary: #2d2d2d;
  --text-primary: #e8e8e8;
  --text-secondary: #b0b0b0;
}
```

---

## Performance Optimizations

### 6. Large Dataset Handling

#### Virtual Scrolling 🔧
**Priority:** Medium  
**Effort:** 8-12 hours

**When:** File list > 500 items, trace list > 100 items

**Benefits:**
- Render only visible items
- Smooth scrolling with thousands of items
- Lower memory usage

**Libraries:**
- react-window (if migrating to React)
- vanilla-virtual-list
- Custom implementation

---

#### Caching 🔧
**Priority:** Medium  
**Effort:** 4-6 hours

**Cache:**
- Blame data per file
- Project file list
- Trace metadata
- Config data

**Strategy:**
- In-memory cache with TTL
- Invalidate on file changes
- Persist to localStorage for recent projects

**Example:**
```javascript
class Cache {
  constructor(ttl = 300000) { // 5 minutes
    this.cache = new Map();
    this.ttl = ttl;
  }
  
  set(key, value) {
    this.cache.set(key, { value, timestamp: Date.now() });
  }
  
  get(key) {
    const item = this.cache.get(key);
    if (!item) return null;
    if (Date.now() - item.timestamp > this.ttl) {
      this.cache.delete(key);
      return null;
    }
    return item.value;
  }
}
```

---

#### Debouncing & Throttling 🔧
**Priority:** High  
**Effort:** 2-3 hours

**Apply to:**
- Search input (debounce 300ms)
- Window resize (throttle 100ms)
- File list filter (debounce 200ms)

**Example:**
```javascript
function debounce(func, wait) {
  let timeout;
  return function(...args) {
    clearTimeout(timeout);
    timeout = setTimeout(() => func.apply(this, args), wait);
  };
}

fileSearchInput.addEventListener('input', debounce(() => {
  renderFileList();
}, 300));
```

---

## Accessibility Improvements

### 7. WCAG 2.1 Compliance

#### Screen Reader Support 🔧
**Priority:** Medium  
**Effort:** 6-8 hours

**Add:**
- ARIA live regions for status updates
- ARIA labels for icon-only buttons
- ARIA expanded/collapsed for panels
- ARIA selected for list items
- Alt text for all images

**Example:**
```html
<div role="status" aria-live="polite" aria-atomic="true" id="sr-status"></div>

<button aria-label="Open project folder">
  <img src="folder-icon.svg" alt="" role="presentation" />
</button>
```

---

#### Keyboard Navigation 🔧
**Priority:** High  
**Effort:** 4-6 hours

**Requirements:**
- Tab order makes sense
- Focus visible on all interactive elements
- Focus trap in modals
- Arrow keys for lists
- Enter/Space to activate

**Example:**
```javascript
blameFileList.addEventListener('keydown', (e) => {
  const items = Array.from(blameFileList.querySelectorAll('li'));
  const current = items.indexOf(document.activeElement);
  
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    items[(current + 1) % items.length]?.focus();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    items[(current - 1 + items.length) % items.length]?.focus();
  }
});
```

---

#### Color Contrast 🔧
**Priority:** Medium  
**Effort:** 2-3 hours

**Audit:**
- Ensure 4.5:1 contrast ratio for normal text
- Ensure 3:1 for large text and UI components
- Test with color blindness simulators
- Don't rely on color alone to convey information

**Tools:**
- Chrome DevTools Accessibility Panel
- axe DevTools extension
- Contrast checker

---

## Testing & Quality Assurance

### 8. Automated Testing

#### Unit Tests 🔧
**Priority:** High  
**Effort:** 12-16 hours

**Framework:** Vitest or Jest

**Test coverage:**
- Utility functions (filter, search, normalize)
- State management
- Data transformations
- Cache logic

**Example:**
```javascript
import { describe, it, expect } from 'vitest';
import { fuzzyMatch, getVisibleFiles } from './app.js';

describe('fuzzyMatch', () => {
  it('should match exact strings', () => {
    expect(fuzzyMatch('hello', 'hello')).toBe(true);
  });
  
  it('should match with missing characters', () => {
    expect(fuzzyMatch('hello', 'hlo')).toBe(true);
  });
  
  it('should be case insensitive', () => {
    expect(fuzzyMatch('Hello', 'hello')).toBe(true);
  });
});
```

---

#### E2E Tests 🔧
**Priority:** Medium  
**Effort:** 16-24 hours

**Framework:** Playwright

**Test scenarios:**
- Navigation between views
- Open project workflow
- File selection and blame display
- Search and filter
- Settings changes
- Error handling

**Example:**
```javascript
import { test, expect } from '@playwright/test';

test('should navigate to Blame Viewer', async ({ page }) => {
  await page.goto('http://localhost:8080');
  await page.click('button:has-text("Blame Viewer")');
  await expect(page.locator('h1')).toHaveText('Blame Viewer');
});

test('should filter files by search', async ({ page }) => {
  await page.goto('http://localhost:8080');
  await page.click('button:has-text("Blame Viewer")');
  await page.fill('#file-search', 'test');
  const items = await page.locator('#blame-file-list li').count();
  expect(items).toBeGreaterThan(0);
});
```

---

#### Cross-Platform Testing 🔧
**Priority:** High  
**Effort:** 8-12 hours per platform

**Platforms:**
- Windows 10/11
- macOS 12+
- Ubuntu 20.04+

**Test:**
- Window management
- File paths (backslash vs forward slash)
- Native dialogs
- Keyboard shortcuts (Cmd vs Ctrl)
- Font rendering

---

## Documentation

### 9. User Documentation

#### User Guide 🔧
**Priority:** High  
**Effort:** 8-12 hours

**Sections:**
- Installation
- Getting Started
- Opening a Project
- Using Blame Viewer
- Understanding Traces
- Configuring Rules
- Keyboard Shortcuts
- Troubleshooting

---

#### Video Tutorials 🔧
**Priority:** Medium  
**Effort:** 16-24 hours

**Videos:**
1. Quick Start (2-3 min)
2. Deep Dive: Blame Viewer (5-7 min)
3. Deep Dive: Trace Explorer (5-7 min)
4. Configuration Tips (3-5 min)

---

#### Developer Documentation 🔧
**Priority:** Medium  
**Effort:** 6-8 hours

**Topics:**
- Architecture overview
- Adding new Tauri commands
- Frontend development setup
- Building from source
- Contributing guidelines

---

## Priority Matrix

### High Priority (Do First)
1. ✅ Loading states
2. ✅ Error handling
3. ✅ Empty states
4. ✅ Config save/load
5. ✅ Real trace data
6. ✅ Debouncing inputs
7. ✅ Keyboard navigation
8. ✅ Unit tests

### Medium Priority (Do Next)
9. Toast notifications
10. Trace detail view
11. Export HTML
12. Keyboard shortcuts
13. Dark mode
14. Virtual scrolling
15. Screen reader support
16. E2E tests

### Low Priority (Polish)
17. Animations
18. Better blame details panel
19. Video tutorials
20. Advanced search features

## Effort Summary

| Category | Total Effort |
|----------|-------------|
| Stub Implementation | 14-20 hours |
| Data Integration | 14-20 hours |
| Loading & Errors | 14-20 hours |
| UX Improvements | 18-28 hours |
| Performance | 14-21 hours |
| Accessibility | 12-17 hours |
| Testing | 36-52 hours |
| Documentation | 30-44 hours |
| **TOTAL** | **152-222 hours** |

**Estimated Calendar Time:** 8-12 weeks (1 developer working 20 hours/week)

## Quick Wins (< 4 hours each)

These can be done immediately for high impact:

1. ✅ Add loading spinner CSS (1 hour)
2. ✅ Debounce search input (1 hour)
3. ✅ Empty state messages (2 hours)
4. ✅ Basic error dialogs (3 hours)
5. ✅ Toast notification system (3 hours)
6. ✅ Color contrast audit (2 hours)
7. ✅ Basic keyboard shortcuts (3 hours)

**Total Quick Wins:** 15 hours

## Next Actions

### This Week
- [ ] Implement loading spinners
- [ ] Add error dialogs
- [ ] Create empty state designs
- [ ] Debounce search inputs

### Next Week
- [ ] Implement config save/load backend
- [ ] Integrate real trace data
- [ ] Add toast notifications
- [ ] Start unit test suite

### This Month
- [ ] Complete all high-priority items
- [ ] Begin medium-priority items
- [ ] Cross-platform testing
- [ ] User documentation

---

**Last Updated:** December 31, 2025  
**Status:** In Progress  
**Version:** 0.1.0-prototype

# App.tsx Refactoring

The large `App.tsx` file (1474 lines) has been successfully split into multiple focused modules for better maintainability and code organization.

## File Structure

### Core App Component
- **`App.tsx`** - Main application component with UI rendering and state management

### Utility Modules
- **`utils/storage.ts`** - LocalStorage functions for file system and file content management
- **`utils/fileSystemUtils.ts`** - File system navigation and manipulation utilities

### Hooks
- **`hooks/useFileOperations.ts`** - File operations handlers (create, move, rename, delete, upload)
- **`hooks/useEditorManager.ts`** - Editor state management and file tab handling

### Components
- **`components/ConfirmationDialogs.tsx`** - Move and delete confirmation dialog components

## Benefits of Refactoring

1. **Improved Maintainability** - Each module has a single responsibility
2. **Better Code Organization** - Related functions are grouped together
3. **Easier Testing** - Individual modules can be tested in isolation
4. **Reduced Complexity** - Main App component is now more focused on UI logic
5. **Code Reusability** - Utility functions can be easily reused across components

## Module Responsibilities

### `utils/storage.ts`
- File system serialization/deserialization
- File content storage and retrieval
- Default content generation for different file types

### `utils/fileSystemUtils.ts`
- File system tree navigation
- Entry finding by ID or path
- Path manipulation utilities

### `hooks/useFileOperations.ts` 
- File and folder creation
- Move and rename operations
- Delete operations
- File upload handling
- Conflict resolution with replacement

### `hooks/useEditorManager.ts`
- Open file tabs management
- Active file state
- Editor content synchronization
- File selection handling

### `components/ConfirmationDialogs.tsx`
- Move confirmation dialog
- Delete confirmation dialog
- Reusable dialog components

## Recent Changes

### File Organization Improvements (Latest)

1. **VMStatesDisplay Location**
   - Moved `vmStatesDisplay.tsx` from `components/execution/` to `components/graph/`
   - This makes more sense since VM states are visualized using graphs
   - Updated imports within the file to use relative paths within the graph folder

2. **Main CSS Location**
   - Moved `main.css` from `assets/styles/` to the root `app/` directory
   - Now it's alongside `main.tsx` for better organization
   - Updated import in `main.tsx` to use `'./main.css'`

3. **Import Aliases Exploration**
   - Attempted to set up `@components`, `@hooks`, `@utils`, etc. aliases
   - Found that Parcel 2's alias system needs specific configuration
   - For now, using relative imports which work reliably
   - TypeScript path mapping is configured in `tsconfig.json` for IDE support

### Import Aliases Implementation ✅

Successfully implemented TypeScript-based import aliases for cleaner, more maintainable imports:

**Configured Aliases:**
- `@components/*` → `./components/*`
- `@hooks/*` → `./hooks/*`
- `@utils/*` → `./utils/*`
- `@assets/*` → `./assets/*`
- `@tutorials/*` → `./tutorials/*`
- `@examples/*` → `./examples/*`

**Implementation Approach:**
- Used TypeScript path mapping in `tsconfig.json` (no build tool configuration needed)
- This provides excellent IDE/editor support with autocomplete and navigation
- Works seamlessly with Parcel without additional configuration
- All imports now use clean, absolute-style paths

**Example Usage:**
```typescript
// Old relative imports
import { STORAGE_KEYS } from './utils/storage';
import FileExplorer from './components/fileexplorer/FileExplorer';
import { useFileOperations } from './hooks/useFileOperations';

// New alias imports
import { STORAGE_KEYS } from '@utils/storage';
import FileExplorer from '@components/fileexplorer/FileExplorer';
import { useFileOperations } from '@hooks/useFileOperations';
```

**Benefits:**
- No more complex relative path navigation (`../../../`)
- Better IDE support and autocomplete
- Easier refactoring and maintenance
- Clear, readable import statements
- Consistent import style across the codebase

### Build Status
- ✅ All builds are working correctly
- ✅ All import paths are properly resolved
- ✅ No broken references remain

### Directory Structure (Updated)
```
app/
├── main.tsx
├── main.css                           // ← Moved here from assets/styles/
├── App.tsx
├── components/
│   ├── editor/
│   ├── fileexplorer/
│   ├── graph/
│   │   ├── Graph.tsx
│   │   ├── CommGraph.tsx
│   │   ├── Node.tsx
│   │   ├── GraphToolbar.tsx
│   │   ├── visHelpers.ts
│   │   ├── visOptions.ts
│   │   └── vmStatesDisplay.tsx        // ← Moved here from execution/
│   ├── tutorial/
│   └── dialogs/
├── hooks/
├── utils/
├── assets/
└── tutorials/
```

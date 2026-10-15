# Online editor

## Althread language server

`.alt` buffers use an LSP 3.17 client over JSON-RPC messages to a dedicated
browser worker. No backend service or network connection is needed for language
features. This worker is separate from execution and model checking.

Available features:

- Contextual completion (`Ctrl+Space`), local and shared bindings, parameters,
  functions, concurrent programs, imported namespaces and snippets.
- Type-aware list and tuple members, including chained calls.
- Types and signatures on hover; argument hints while typing a call.
- Definition navigation (`F12` or Ctrl/Cmd+click), including imported files.
- References (`Shift+F12`) and symbol rename (`F2`) across project files.
- Debounced diagnostics from the existing Althread WASM compiler.

The client overlays the active unsaved buffer on the virtual project files. It
synchronizes full document changes with monotonically increasing versions, closes
removed documents, rejects stale results and terminates workers when their editor
session ends. Navigation uses full file paths, including nested directories and
installed packages. Rename refuses name capture, collisions and edits to `deps/`.

The recovery-oriented semantic index in `app/lsp/analysis.ts` stays available when
code is incomplete. It currently infers types from annotations, literals, names,
function returns and supported member calls; expressions it cannot infer display
`unknown`. It is not a replacement for the compiler's type checker. Formatting,
code actions and a standalone stdio server are not implemented. Compiler diagnostics
currently report the first error, including the source path for imported errors.

## Validation

With Node.js 22 and project dependencies installed:

```sh
pnpm check:lsp
pnpm test:lsp
pnpm build
```

`check:lsp` checks the language service independently of existing type errors in
other parts of the application. Tests cover scope and shadowing, imports and
privacy, member types, incomplete code, references, rename conflicts, UTF-8 compiler
spans versus UTF-16 LSP positions, document versions and worker lifecycle.

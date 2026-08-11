# Use `yamalgam-lsp` in an editor

`yamalgam-lsp` is an LSP 3.17 server. It reads frames from stdin and writes
frames to stdout. It has no prompt.

Install both tools from a source checkout:

```sh
cargo install --path crates/yamalgam
```

Point the editor at this command with no arguments:

```text
yamalgam-lsp
```

Each workspace folder is a file root. Local schemas must stay in one of those
roots. So must links used by go to definition.

## Configure schema associations

Clients can pass this object at LSP start:

```json
{
  "yamalgam": {
    "schemas": [
      {
        "uri": "./schemas/service.yaml",
        "fileMatch": ["services/*.yaml", "service.yaml"]
      }
    ],
    "maxDiagnostics": 64
  }
}
```

| Setting | Default | Effect |
|---------|---------|--------|
| `yamalgam.schemas` | `[]` | Ordered local schema URI and file-glob associations. |
| `yamalgam.maxDiagnostics` | `64` | Maximum syntax and schema diagnostics for one open document. |

Schema paths are relative to the YAML file. The target must stay in a workspace
folder. Settings are read once at start. Restart the server after a change.

The first schema match wins. The order is pragma or modeline, top-level
`$schema`, config match, then SchemaStore. An editor has no `--schema` layer.
M12 turns off SchemaStore and network use in the server. Thus, a config match
is its last source.

## Neovim 0.11 or later

Add this LSP config. Then enable it:

```lua
vim.lsp.config('yamalgam', {
  cmd = { 'yamalgam-lsp' },
  filetypes = { 'yaml' },
  root_markers = { '.yamalgam.yaml', '.yamalgam.toml', '.git' },
  init_options = {
    yamalgam = {
      schemas = {
        {
          uri = './schemas/service.yaml',
          fileMatch = { 'services/*.yaml' },
        },
      },
      maxDiagnostics = 64,
    },
  },
})
vim.lsp.enable('yamalgam')
```

Open a YAML file. Run `:checkhealth vim.lsp` if it does not attach.

## VS Code-compatible clients

VS Code needs an extension to start a new server. In a
`vscode-languageclient` extension, use stdio. Select YAML files. Pass the same
start object:

```typescript
const serverOptions = {
  command: "yamalgam-lsp",
  transport: TransportKind.stdio,
};

const clientOptions = {
  documentSelector: [{ scheme: "file", language: "yaml" }],
  initializationOptions: {
    yamalgam: {
      schemas: [
        { uri: "./schemas/service.yaml", fileMatch: ["services/*.yaml"] },
      ],
      maxDiagnostics: 64,
    },
  },
};
```

M12 ships the server, not a VS Code extension. A generic LSP extension can use
the same command and start data.

## Zed

Zed needs a language extension for each server adapter. Once an adapter named
`yamalgam-lsp` is installed, select it for YAML. Then set its binary and start
data:

```jsonc
{
  "languages": {
    "YAML": {
      "language_servers": ["yamalgam-lsp", "!yaml-language-server", "..."]
    }
  },
  "lsp": {
    "yamalgam-lsp": {
      "binary": {
        "path": "/absolute/path/to/yamalgam-lsp",
        "arguments": []
      },
      "initialization_options": {
        "yamalgam": {
          "schemas": [
            {
              "uri": "./schemas/service.yaml",
              "fileMatch": ["services/*.yaml"]
            }
          ],
          "maxDiagnostics": 64
        }
      }
    }
  }
}
```

M12 does not ship that Zed adapter. Zed settings alone cannot point its built-in
YAML adapter at a new server.

## BBEdit and generic LSP clients

In BBEdit, open **Settings → Languages**, add custom settings for YAML, and use
the **Server** tab:

- Command: `yamalgam-lsp`
- Arguments: leave empty
- Language ID: `yaml`

BBEdit checks its Language Servers support folder and login-shell `PATH`. If it
cannot find the tool, put a symlink in that folder. This UI has no custom start
data. Use a document pragma or top-level `$schema` key instead.

For another LSP client, use stdio, language ID `yaml`, UTF-16 positions, and a
project root. Pass the start object above. Do not send logs through stdout. LSP
frames own that stream.

## Supported features

- Incremental document synchronization with recovering syntax diagnostics
- Draft 2020-12 schema diagnostics for every YAML stream document
- Property, enum, default, and type-aware completion
- Schema title, description, type, enum, default, deprecation, and example hover
- Go to definition for schema `$ref`, data `$ref`, and local `!include`
- UTF-16 client positions with byte-offset source tracking internally

Errors include the current file version. Their `data` object has
`documentIndex`, `instancePath`, `schemaPath`, and a safe `schemaUri`.

## M12 boundaries

The server does not format or edit files. It has no rename, references, code
actions, quick fixes, workspace symbols, or semantic tokens. It has no disk
schema cache, login flow, or network schema access.

Completion and hover need a schema from the workspace. Links for includes and
data refs are local-only. Path and symlink escapes are denied.

See the current editor guides for
[Neovim LSP](https://neovim.io/doc/user/lsp),
[Zed server settings](https://zed.dev/docs/configuring-languages),
[Zed language extensions](https://zed.dev/docs/extensions/languages), and
[BBEdit LSP setup](https://www.barebones.com/support/bbedit/lsp-notes.html).

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YAMALGAM_LSP_BIN:?set YAMALGAM_LSP_BIN to the yamalgam-lsp executable}"

initialize='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{}}}'
initialized='{"jsonrpc":"2.0","method":"initialized","params":{}}'
shutdown='{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}'
exit_message='{"jsonrpc":"2.0","method":"exit","params":null}'

frame() {
  printf 'Content-Length: %s\r\n\r\n%s' "${#1}" "$1"
}

output="$({
  frame "$initialize"
  frame "$initialized"
  frame "$shutdown"
  frame "$exit_message"
} | "$YAMALGAM_LSP_BIN")"

printf '%s\n' "$output" | grep -F 'yamalgam-lsp' >/dev/null
printf '%s\n' "$output" | grep -F 'textDocumentSync' >/dev/null
```
</script>-->

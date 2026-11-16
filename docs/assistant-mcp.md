# Using Golden from an AI assistant (MCP)

`golden --mcp` serves a **read-only** Model Context Protocol server on stdin/stdout, using the
same encrypted database as the app. Nothing is sent anywhere by Golden: the assistant you attach
it to runs on your machine and reads the answers it asks for.

Tools: `accounts`, `balance_sheet`, `cash_flow`, `categories`, `budget_status`, `forecast`,
`suggest_allocation`. There is deliberately no tool that imports, edits, deletes, restores or
activates anything, and a test fails if one is ever added with such a name.

Example client config (macOS):

```json
{ "mcpServers": { "golden": { "command": "/Applications/Golden.app/Contents/MacOS/golden", "args": ["--mcp"] } } }
```

Open the app and import a statement once first, so the database exists.

Known limitation: the Windows build is a GUI-subsystem program, so `--mcp` has no console to
attach to there yet; a small console launcher is the next step.

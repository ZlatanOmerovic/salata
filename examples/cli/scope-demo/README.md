# Scope Demo

Shows the difference between shared and isolated scope.

## Files

- `shared-scope.slt` — Two Python blocks, second sees variables from first (default behavior)
- `isolated-scope.slt` — Two Python blocks with `scope="isolated"`, second cannot see first's variables

## Run

```bash
salata --config config.toml shared-scope.slt
salata --config config.toml isolated-scope.slt
```

## What It Demonstrates

- Shared scope (default): all blocks of the same language run in one process
- Isolated scope (`scope="isolated"`): each block gets a fresh process
- The `NameError` in isolated mode proves isolation works

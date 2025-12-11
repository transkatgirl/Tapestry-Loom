# Tapestry Loom Migration Assistant

## Migrating weaves from the old Tapestry Loom Obsidian plugin

Run the following commands in the migration assistant folder:

```bash
cargo run --release -- --input $OLD_TAPESTRY_OBSIDIAN_VAULT --output ~/"Documents/Tapestry Loom/Migrated Weaves"
```

Where `$OLD_TAPESTRY_OBSIDIAN_VAULT` is set to the location of the vault used by the Obsidian plugin.
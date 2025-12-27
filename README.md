# Dev Skills

Developer Skills marketplace for [Claude Code](https://code.claude.com/) - providing performance diagnostics, code analysis, and more.

## Installation

```bash
# Add marketplace
/plugin marketplace add ly87ing/dev-skills

# Install java-perf plugin
/plugin install java-perf@dev-skills
```

## Available Plugins

| Plugin | Description | Version |
|--------|-------------|---------|
| [java-perf](./plugins/java-perf/) | Java performance diagnostics using AST analysis. Identifies N+1 queries, memory leaks, lock contention, and concurrency risks. | 8.1.0 |

## Version Management

Version is managed via the `VERSION` file. To update version across all config files:

```bash
./scripts/sync-version.sh
```

## Plugin Development

### Directory Structure

```
dev-skills/
├── .claude-plugin/
│   └── marketplace.json      # Marketplace definition
├── plugins/
│   └── java-perf/            # Individual plugin
│       ├── .claude-plugin/plugin.json
│       ├── skills/<name>/SKILL.md
│       ├── hooks/hooks.json
│       └── rust/             # Plugin-specific code
├── VERSION                   # Single source of truth for version
└── scripts/sync-version.sh   # Version sync utility
```

## References

- [Agent Skills](https://code.claude.com/docs/en/skills) - How to create and distribute Skills
- [Plugin Marketplaces](https://code.claude.com/docs/en/plugin-marketplaces) - How to create and host marketplaces
- [Plugins Reference](https://code.claude.com/docs/en/plugins-reference) - Complete technical reference

## License

MIT

# parqeye

[![CI][actions-badge]][actions-url]

[actions-badge]: https://github.com/kaushiksrini/parqeye/actions/workflows/ci.yaml/badge.svg
[actions-url]: https://github.com/kaushiksrini/parqeye/actions/workflows/ci.yaml

`parqeye` lets you _peek inside_ your Parquet files. Instantly inspect their contents, schema, and metadata — right from your terminal.

![Demo](.github/assets/demo.gif)

**Features**

- **Interactive Data Visualization** - Browse through your Parquet data in a table view with keyboard navigation.
- **Text search** - Press `/` to search across all columns; results are filtered to matching rows. Press Esc to clear the filter.
- **SQL tab** - Run SQL queries against the open Parquet file (table name: `parquet`). Results appear in a table; press `v` on a row to view full row detail.
- **Row detail view** - On the Visualize or SQL result view, press `v` on the selected row to see every column and value on one screen. Scroll with ↑↓ PgUp PgDn (vertical) and ←→ (horizontal). Esc to close.
- **Schema Explorer** - Inspect column types, nested structures, and field definitions.
- **File Metadata** - View Parquet file-level metadata including version, created by, encoding stats and more.
- **Row Group Statistics** - Examine row group-level metadata, statistics, and data distribution across groups.
- **Tab-based Interface** - Switch between Visualize, Schema, Metadata, Row Groups, and SQL views.
- **Terminal-native** - Works directly in your terminal.

# Usage

Run `parqeye` by providing the path to the `.parquet` file.

```
parqeye <path-to-parquet-file>
```

# Keyboard shortcuts

| Key | Action |
|-----|--------|
| **Tab** / **Shift+Tab** | Next / previous tab |
| **Ctrl+X** | Quit |
| **/** | Start search (type query, Enter to filter; Esc to cancel or clear filter) |
| **Esc** | Cancel search, clear SQL query, clear search filter, or close row detail view (context-dependent) |

**Visualize tab**

| Key | Action |
|-----|--------|
| **↑ / ↓** | Move row |
| **← / →** | Move column |
| **u / d** | Page up / down |
| **v** | Open row detail view for selected row |

**SQL tab**

| Key | Action |
|-----|--------|
| Type | Edit query (cursor at end of line) |
| **Enter** | Run query (table name: `parquet`) |
| **v** | Open row detail view for selected result row (when results are shown) |

**Row detail view** (after pressing `v`)

| Key | Action |
|-----|--------|
| **Esc** | Close and return to table |
| **↑ / ↓** | Scroll one line up / down |
| **PgUp / PgDn** | Scroll one page up / down |
| **← / →** | Scroll left / right (for long values) |

# Installation

## Direct Download

You can download the latest release from the [Releases](https://github.com/kaushiksrini/parqeye/releases) page.

## Build from Source

You can build from source by downloading the repository and running the following command:

```
cargo build --release
```

## Cargo

If you use Rust, build directly from [crates.io](https://crates.io/crates/parqeye)

```
cargo install parqeye
```

## Homebrew

If you have Homebrew, you can install using:

```sh
brew install kaushiksrini/parqeye/parqeye
```

# License

This package is released under the [MIT License](./LICENSE).

# Acknowledgements

- [csvlens](https://github.com/YS-L/csvlens) for the inspiration


# TODOs

- [ ] Lazy/streaming loading of parquet files.
- [ ] Read parquet files on the cloud (`s3://...`).

# BulkMorph

BulkMorph is a Rust-based tool designed to migrate or update document structures in a CouchDB database. It utilizes Lua scripts for transformation and validates data against a Proto file. The tool efficiently processes large datasets by batching data for processing.

It is still work in progress but usable nonetheless under correct condition.

## Features
- Batch processing for scalability
- Lua scripting for transformation logic
- Proto file validation
- Handles any dataset size

## Installation
Download the latest binary from the [releases page](https://github.com/xeonn/bulkmorph/releases) and extract it to your desired location.

## Usage
```sh
bulkmorph --url <CouchDB_URL> \
          --table <TABLE_NAME> \
          --proto <PROTO_FILE> \
          --include <PROTO_DIRECTORY> \
          --script <LUA_SCRIPT> \
          --limit <LIMIT> \
          [--dry-run]
```

## Parameters
- `--url, -u` : URL of the CouchDB database (Example: `http://localhost:5984`)
- `--table, -t` : Name of the table (or document type)
- `--proto, -p` : Path to the `.proto` file for validation (must be the same name as the table name, but can follow CamelCase as per Proto file convention)
- `--include, -i` : Directory containing `.proto` files
- `--script, -s` : Path to the Lua script for transformation (must exist in the specified script folder and have the same name as the table name in all lowercase)
- `--limit, -l` : Maximum number of documents to fetch per iteration (default: 1000)
- `--dry-run` : Enable dry-run mode to preview changes without modifying the database

## Configuration
The tool requires specifying database connection details, batch sizes, and Lua transformation scripts via command-line parameters. The Lua script file must match the table name in all lowercase and must exist in the specified script directory. The Proto file is compulsory and must have the same name as the table name, following Proto file naming conventions.

## License
MIT


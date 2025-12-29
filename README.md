# sqlift

Generate typed data access code from your database schema. No ORM just readable code with plain SQL.

## Features

- Introspects database schemas to extract tables, columns, and enums
- Generates typed dataclasses/structs for type-safe records
- Creates CRUD functions (`get_by_id`, `get_all`, `insert`, `update`, `delete`, `upsert`)
- Supports custom enum types
- Two output modes: library (one file per table) or flat (single file)
- Two function styles: standalone functions or repository classes
- Extensible architecture for adding databases and languages

## Supported Targets

### Databases

| Database   | Status    | Feature Flag |
| ---------- | --------- | ------------ |
| PostgreSQL | Supported | `postgres`   |
| MySQL      | Planned   | -            |
| SQLite     | Planned   | -            |

### Languages

| Language   | Status       |
| ---------- | ------------ |
| Python     | Supported |
| TypeScript | Planned   |
| Go         | Planned   |

## Installation

```bash
# Install with PostgreSQL support
cargo install sqlift --features postgres

# Install with multiple database support (when available)
cargo install sqlift --features postgres,mysql
```

## Quick Start

1. Create a `.env` file with your database credentials:

```env
DB_HOST=localhost
DB_PORT=5432
DB_NAME=myapp
DB_USER=postgres
DB_PASSWORD=secret
```

2. Generate code:

```bash
sqlift <database> <language>

# Example: PostgreSQL → Python
sqlift postgres python
```

3. Use the generated code (Python example):

```python
from psycopg import connect
from database import UserRecord, get_user_by_id, insert_user

with connect("dbname=myapp user=postgres") as conn:
    # Insert a new user
    user = insert_user(conn, email="alice@example.com", name="Alice")

    # Fetch by primary key
    user = get_user_by_id(conn, user.id)

    conn.commit()
```

## CLI Usage

```
sqlift <database> <language> [options]
```

### Arguments

| Argument   | Description     |
| ---------- | --------------- |
| `database` | Database type   |
| `language` | Target language |

### Options

| Option         | Description                             | Default      |
| -------------- | --------------------------------------- | ------------ |
| `-o, --output` | Output directory or file path           | `./database` |
| `--mode`       | Output mode: `library` or `flat`        | `library`    |
| `--style`      | Function style: `standalone` or `class` | `standalone` |
| `--schema`     | Database schema to introspect           | `public`     |
| `--env-file`   | Path to .env file                       | `./.env`     |
| `--tables`     | Comma-separated tables to include       | All tables   |
| `--exclude`    | Comma-separated tables to exclude       | None         |
| `-v`           | Verbose output (`-vv` for trace)        | Info level   |

### Examples

```bash
# Generate library with one file per table (default)
sqlift postgres python

# Generate a single flat file
sqlift postgres python --mode flat --output ./db.py

# Use repository classes instead of standalone functions
sqlift postgres python --style class

# Only generate code for specific tables
sqlift postgres python --tables users,orders,products

# Exclude certain tables
sqlift postgres python --exclude migrations,schema_versions
```

## Output Modes

### Library Mode (default)

Creates a package with one file per table:

```
database/
├── __init__.py
├── enums.py        # If you have custom enums
├── users.py
├── orders.py
└── products.py
```

### Flat Mode

Creates a single file with all code:

```
database.py
```

## Function Styles

### Standalone (default)

Functions accept a connection as the first parameter:

```python
def get_user_by_id(conn: Connection, id: int) -> UserRecord | None:
    ...

def insert_user(conn: Connection, email: str, name: str) -> UserRecord:
    ...
```

### Class

Methods on a repository class:

```python
class UserRepository:
    def __init__(self, conn: Connection) -> None:
        self.conn = conn

    def get_by_id(self, id: int) -> UserRecord | None:
        ...

    def insert(self, email: str, name: str) -> UserRecord:
        ...
```

## Generated Functions

For each table, sqlift generates:

| Function      | Description                     | Requires PK |
| ------------- | ------------------------------- | ----------- |
| `get_by_<pk>` | Fetch one record by primary key | Yes         |
| `get_all`     | Fetch all records               | No          |
| `insert`      | Create a new record             | No          |
| `update`      | Update a record by primary key  | Yes         |
| `delete`      | Delete a record by primary key  | Yes         |
| `upsert`      | Insert or update on conflict    | Yes\*       |

\*`upsert` is only generated for tables with non-auto-generated primary keys (e.g., UUID or natural keys).

## Documentation

- [PostgreSQL Type Mappings](docs/postgres.md)

## Roadmap

### New Operations

- [ ] Batch operations (`insert_many`, `update_many`, `delete_many`)
- [ ] Pagination helpers for `get_all` (limit/offset and cursor-based)
- [ ] Index-based lookups (`get_users_by_email` for unique indexes)
- [ ] Foreign key relationship helpers

### New Databases

- [ ] MySQL / MariaDB
- [ ] SQLite

### New Languages

- [ ] TypeScript
- [ ] Go

### Other

- [ ] Async function generation (e.g., asyncpg for Python)

## License

MIT

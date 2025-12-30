# sqlift

Generate typed data access code from your database schema. No ORM just readable code with plain SQL.

## Features

- Introspects database schemas to extract tables, columns, and enums
- Generates typed dataclasses/structs for type-safe records
- Creates CRUD functions (`get_by_id`, `get_all`, `insert`, `update`, `delete`, `upsert`)
- Supports partial updates with type-safe sentinel pattern
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

| Language   | Status    |
| ---------- | --------- |
| Python     | Supported |
| TypeScript | Planned   |
| Go         | Planned   |

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
from database import UserRecord, get_user_by_id, insert_user, update_user

with connect("dbname=myapp user=postgres") as conn:
    # Insert a new user
    user = insert_user(conn, email="alice@example.com", name="Alice")

    # Fetch by primary key
    user = get_user_by_id(conn, user.id)

    # Partial update - only change email, leave other fields unchanged
    user = update_user(conn, id=user.id, email="alice.new@example.com")

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
├── _types.py       # UNSET sentinel for partial updates
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

## Partial Updates

The `update` and `upsert` functions support **partial updates**, you only need to pass the fields you want to change. This is achieved using an `UNSET` sentinel value that distinguishes between "don't change this field" and "set this field to NULL".

```python
from database import update_user

# Only update email - all other fields remain unchanged
update_user(conn, id=1, email="new@example.com")

# Set nickname to NULL explicitly (for nullable columns)
update_user(conn, id=1, nickname=None)

# Update multiple fields at once
update_user(conn, id=1, email="new@example.com", name="New Name")
```

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

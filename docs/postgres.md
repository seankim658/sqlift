# Type Mappings

This document describes the PostgreSQL data types that sqlift supports and how they map to target language types.

## Supported PostgreSQL Types

### Python Mappings

| PostgreSQL Type                            | Python Type            | Import Required                 |
| ------------------------------------------ | ---------------------- | ------------------------------- |
| `smallint`, `int2`                         | `int`                  | -                               |
| `integer`, `int`, `int4`                   | `int`                  | -                               |
| `bigint`, `int8`                           | `int`                  | -                               |
| `boolean`, `bool`                          | `bool`                 | -                               |
| `text`                                     | `str`                  | -                               |
| `varchar(n)`, `character varying(n)`       | `str`                  | -                               |
| `char(n)`, `character(n)`                  | `str`                  | -                               |
| `real`, `float4`                           | `float`                | -                               |
| `double precision`, `float8`               | `float`                | -                               |
| `numeric`, `decimal`                       | `Decimal`              | `from decimal import Decimal`   |
| `timestamp`, `timestamp without time zone` | `datetime`             | `from datetime import datetime` |
| `timestamptz`, `timestamp with time zone`  | `datetime`             | `from datetime import datetime` |
| `date`                                     | `date`                 | `from datetime import date`     |
| `time`, `time without time zone`           | `time`                 | `from datetime import time`     |
| `timetz`, `time with time zone`            | `time`                 | `from datetime import time`     |
| `uuid`                                     | `UUID`                 | `from uuid import UUID`         |
| `json`                                     | `dict[str, Any]`       | `from typing import Any`        |
| `jsonb`                                    | `dict[str, Any]`       | `from typing import Any`        |
| `bytea`                                    | `bytes`                | -                               |
| Arrays (e.g., `integer[]`, `text[]`)       | `list[T]`              | (depends on element type)       |
| Custom enum types                          | Generated `Enum` class | `from enum import Enum`         |

### Nullable Handling

- Nullable columns use Python's union syntax: `T | None`
- Non-nullable columns use the base type directly

Example:

```python
@dataclass
class UserRecord:
    id: int              # NOT NULL
    email: str           # NOT NULL
    nickname: str | None # NULL allowed
```

### Array Types

PostgreSQL arrays are mapped to Python `list[T]` where `T` is the element type:

| PostgreSQL Array | Python Type  |
| ---------------- | ------------ |
| `integer[]`      | `list[int]`  |
| `text[]`         | `list[str]`  |
| `uuid[]`         | `list[UUID]` |
| `varchar(255)[]` | `list[str]`  |

### Custom Enum Types

PostgreSQL enum types are introspected and generated as Python `Enum` classes that inherit from both `str` and `Enum`:

```sql
-- PostgreSQL
CREATE TYPE order_status AS ENUM ('pending', 'confirmed', 'shipped', 'delivered');
```

```python
# Generated Python
class OrderStatus(str, Enum):
    PENDING = "pending"
    CONFIRMED = "confirmed"
    SHIPPED = "shipped"
    DELIVERED = "delivered"
```

The `str` inheritance allows the enum values to be used directly in SQL queries without conversion.

## Unsupported PostgreSQL Types

The following PostgreSQL types are **not currently supported**. If encountered, they will be treated as custom enum types (which will likely cause issues):

### Numeric Types

- `money`

### Date/Time Types

- `interval` - Duration type

### Network Types

- `cidr` - IPv4 or IPv6 network
- `inet` - IPv4 or IPv6 host address
- `macaddr` - MAC address
- `macaddr8` - MAC address (EUI-64 format)

### Geometric Types

- `point`
- `line`
- `lseg`
- `box`
- `path`
- `polygon`
- `circle`

### Text Search Types

- `tsvector`
- `tsquery`

### Binary Types

- `bit(n)`
- `bit varying(n)`

### Other Types

- `xml`
- `pg_lsn` - Log sequence number
- `txid_snapshot` - Transaction snapshot

### Composite Types

- User-defined composite types

### Range Types

- `int4range`, `int8range`
- `numrange`
- `tsrange`, `tstzrange`
- `daterange`
- Custom range types

### Domain Types

- User-defined domain types

## Edge Cases

### Tables Without Primary Keys

Tables without primary keys only generate `get_all` and `insert` functions. The following operations require a primary key and are skipped:

- `get_by_*` - No unique identifier to look up by
- `update` - No PK for WHERE clause
- `delete` - No PK for WHERE clause
- `upsert` - No PK for ON CONFLICT

### Tables With All Default/Auto-Generated Columns

When all columns have defaults or are auto-generated (e.g., a table with only `id SERIAL` and `created_at TIMESTAMP DEFAULT NOW()`), the `insert` function takes no parameters and uses `INSERT INTO table_name DEFAULT VALUES RETURNING *`.

```python
# Example: audit_log table with id SERIAL + created_at DEFAULT NOW()
def insert_audit_log(conn: Connection) -> AuditLogRecord:
    """Insert a new audit_log record."""
    cursor = conn.execute(
        "INSERT INTO audit_log DEFAULT VALUES RETURNING *",
    )
    row = cursor.fetchone()
    return AuditLogRecord(**dict(row))
```

## Adding Support for New Types

To add support for a new PostgreSQL type:

1. Add a new variant to the `DataType` enum in `src/schema.rs`
2. Update the `parse_data_type` function in `src/introspect/postgres.rs`
3. Add the Python type mapping in `src/codegen/python/mod.rs`:
   - Update `python_type()` function
   - Update `collect_type_imports()` if an import is needed
4. Update this documentation

## Notes

### Precision and Scale

- `numeric(p, s)` and `decimal(p, s)` precision/scale parameters are recognized during parsing but not used in the Python type (always maps to `Decimal`)
- `varchar(n)` length is captured but not enforced in Python types

### Time Zones

- Both `timestamp` and `timestamptz` map to Python's `datetime`
- Both `time` and `timetz` map to Python's `time`
- Time zone handling is left to the application layer

### JSON vs JSONB

- Both `json` and `jsonb` map to `dict[str, Any]`
- The difference (storage format, indexing) is a PostgreSQL implementation detail

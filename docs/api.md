# HTTP
| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| `POST` | `/create_login` | Admin code | Create a teacher login|
| `POST` | `/create_login` | None | name + password for JWT token (Valid for an hour) |
| `GET` | `/ws` | JWT in header | Create a teacher login|

** `POST /create_login` **

```json
{ "username": "senne", "password": "...", "admin_code": "..." }
```

** `Post /login` **
```json
// Request
{ "username": "senne", "password": "...", "admin_code": "..." }

// Response
{"token": "<jwt>"}
```


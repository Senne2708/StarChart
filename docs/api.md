# Star Chart API

The server exposes two public HTTP endpoints for authentication and one authenticated WebSocket endpoint for everything else.

Base URL (development): `http://127.0.0.1:3000`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| `POST` | `/login` | None | Exchange a teacher's username and password for a JWT (valid for 1 hour) |
| `POST` | `/create_login` | Admin code (in body) | Create a new teacher account |
| `GET` | `/ws` | JWT in `Authorization` header | Upgrade to a WebSocket for real-time data and updates |

---

## Authentication

### `POST /login`

Request body (JSON):

```json
{ "username": "senne", "password": "..." }
```

Response `200 OK`:

```json
{ "token": "<jwt>" }
```

| Status | Meaning |
| --- | --- |
| `200` | Success, token returned |
| `401` | Unknown username or wrong password (the two cases are indistinguishable by design) |
| `500` | Internal error |

**Token details**

- Algorithm: HS256, signed with `JWT_SECRET`.
- Lifetime: 1 hour from issue. There is no refresh endpoint; log in again when it expires.

### `POST /create_login`

Creates a teacher account. The `admin_code` must match the server's `ADMIN_PASSWORD`.

Request body (JSON):

```json
{ "username": "senne", "password": "...", "admin_code": "..." }
```

| Status | Meaning |
| --- | --- |
| `200` | Admin code accepted (see the note below about failures) |
| `401` | Wrong admin code |
| `500` | Hashing failed |

---

## WebSocket

### Connecting

`GET /ws` with a valid token:

```
Authorization: Bearer <jwt>
```

A missing, malformed or expired token is rejected with `401` before the upgrade happens.

> **Browser note:** the browser `WebSocket` constructor cannot set custom headers, so a browser client cannot currently authenticate against `/ws`. See `todo.md`.

### Message envelope

Every client message carries a client-chosen numeric `id`. The server's reply to that message echoes the same `id`, so the client can match replies to requests.

**Client → server**

```json
{ "id": 1, "type": "<message_type>", "data": { } }
```

`data` is omitted for messages that take no parameters.

**Server → client**

```json
{ "id": 1, "msg": { "type": "<message_type>", "data": { } } }
```

Note that the server wraps its payload in a `msg` field, whereas the client's `type`/`data` sit at the top level next to `id`.

### Request/reply and broadcast

- **Every client message gets exactly one reply**, carrying the same `id`. The reply is either the requested result or an `error` message.
- Messages that change data in the database (add_activity, add_student, etc.) will send out a broadcast message. Currently `id` will be set to null.
- If a message cannot be parsed, the reply has `"id": null` and error code `bad_format`.
- The broadcast buffer holds 50 messages (completely trivial). A client that falls too far behind silently skips the missed broadcasts, so clients should re-fetch state (`get_students`, `get_activities`) after reconnecting.

### Client messages

| `type` | `data` | Reply | Broadcast |
| --- | --- | --- | --- |
| `get_students` | none | `students` | no |
| `get_teachers` | none | `teachers` | no |
| `get_activities` | none | `activities` | no |
| `add_student` | `{ "student_name": string, "key_stage": "ks3" \| "ks4" }` | `student_added` | `student_added` |
| `add_activity` | `{ "student_id": int, "teacher_id": int, "reward_description": string, "star_diff": int }` | `activity_added` | `activity_added` |

**Validation**
- `student_name` is trimmed and must not be empty.
- `key_stage` is trimmed and lowercased, and must be `ks3` or `ks4`.
- `star_diff` is a signed 16-bit integer. Use a positive value to award stars and a negative value to cash in a reward.
- `add_activity` fails with `not_found` if the teacher or student does not exist. The star total update and the activity record are written in a single transaction.

**Examples**

```json
{ "id": 1, "type": "get_students" }

{ "id": 2, "type": "add_student",
  "data": { "student_name": "Alex Smith", "key_stage": "ks3" } }

{ "id": 3, "type": "add_activity",
  "data": { "student_id": 1, "teacher_id": 1,
            "reward_description": "Great homework", "star_diff": 2 } }
```

### Server messages

| `type` | `data` | Sent as |
| --- | --- | --- |
| `students` | `Student[]` | reply to `get_students` |
| `teachers` | `Teacher[]` | reply to `get_teachers` |
| `activities` | `Activity[]` (newest first) | reply to `get_activities` |
| `student_added` | `Student` | reply to `add_student`, and broadcast |
| `activity_added` | `{ "activity": Activity, "student": Student }` | reply to `add_activity`, and broadcast. `student` carries the updated `total_stars`. |
| `error` | `{ "code": string, "message": string }` | reply to any failed message |

**Examples**

```json
// Reply to get_students
{ "id": 1, "msg": { "type": "students", "data": [
  { "id": 1, "name": "Alex Smith", "ks": "ks3", "total_stars": 12 }
] } }

// Broadcast after add_activity
{ "id": null, "msg": { "type": "activity_added", "data": {
  "activity": {
    "id": "019012ab-3c4d-7e5f-8a9b-0c1d2e3f4a5b",
    "student_id": 1,
    "teacher_id": 1,
    "reward_description": "Great homework",
    "star_diff": 2,
    "timestamp": "2026-11-05T21:07:00Z"
  },
  "student": { "id": 1, "name": "Alex Smith", "ks": "ks3", "total_stars": 14 }
} } }

// Error reply
{ "id": 4, "msg": { "type": "error",
  "data": { "code": "not_found", "message": "student 99 not found" } } }
```

### Data types

**Student**

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | Auto-incremented |
| `name` | string | |
| `ks` | `"ks3"` \| `"ks4"` | Key stage |
| `total_stars` | integer | Running total (16-bit signed) |

**Teacher**

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | Auto-incremented |
| `name` | string | The login username. The password hash is never sent. |

**Activity**

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | UUID v7 |
| `student_id` | integer | |
| `teacher_id` | integer | |
| `reward_description` | string | |
| `star_diff` | integer | Positive = awarded, negative = cashed in |
| `timestamp` | string | RFC 3339, UTC |

### Error codes

| `code` | Meaning |
| --- | --- |
| `bad_format` | The message was not valid JSON for the protocol (`id` is `null`) |
| `invalid_input` | Validation failed; `message` explains why |
| `not_found` | A referenced student or teacher does not exist |
| `conflict` | A unique constraint was violated |
| `internal_error` | Something went wrong on the server; `message` is always `"internal error"` and details are in the server log |

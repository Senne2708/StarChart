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

Both endpoints take a JSON body with `Content-Type: application/json`. The body can be at most 4 KiB.

Both also return these errors for a bad request:

| Status | Meaning |
| --- | --- |
| `400` | The body is not valid JSON |
| `413` | The body is larger than 4 KiB |
| `415` | `Content-Type` is missing or not `application/json` |
| `422` | The JSON is missing a field or has a field of the wrong type |

### `POST /login`

Request body:

```json
{ "username": "teacher1", "password": "..." }
```

Response `200 OK`:

```json
{ "token": "<jwt>" }
```

| Status | Meaning |
| --- | --- |
| `200` | Success, token returned |
| `401` | Unknown username or wrong password. The two cases can't be told apart, by design: an unknown username still runs a full password check, so response times are similar. |
| `500` | Internal error |

**Token details**

- Algorithm: HS256, signed with `JWT_SECRET`.
- Lifetime: 1 hour from issue. There is no refresh endpoint, so log in again when it expires.

### `POST /create_login`

Creates a teacher account. The `admin_code` must match the server's `ADMIN_PASSWORD`.

Request body:

```json
{ "username": "teacher1", "password": "...", "admin_code": "..." }
```

The success response has an empty body. It does not return a token, so call `/login` afterwards.

| Status | Meaning |
| --- | --- |
| `201` | Account created |
| `401` | Wrong admin code |
| `409` | A teacher with that username already exists |
| `500` | Internal error (e.g. hashing or database failure) |

The username and password are not checked for empty values or length.

---

## WebSocket

### Connecting

`GET /ws` with a valid token:

```
Authorization: Bearer <jwt>
```

A missing, malformed or expired token is rejected with `401` before the upgrade happens.

> **Browser note:** the browser `WebSocket` constructor cannot set custom headers, so a browser client cannot currently authenticate against `/ws`. See [todo.md](todo.md).

**Limits:** a single WebSocket message or frame can be at most 16 KiB. Only text frames are handled. Binary frames are ignored, and get no reply.

### Message envelope

Both directions use the same shape. A numeric `id` sits next to a `msg` object, which holds the message `type` and its `data`.

**Client → server**

```json
{ "id": 1, "msg": { "type": "<message_type>", "data": { } } }
```

- `id` is chosen by the client and must be a non-negative integer.
- `data` is omitted for messages that take no parameters.

**Server → client**

```json
{ "id": 1, "msg": { "type": "<message_type>", "data": { } } }
```

- For a reply, `id` echoes the `id` of the client message it answers.
- For a broadcast, or a reply to a message that couldn't be parsed, `id` is `null`.
- `data` is omitted when the message has no payload (`ack`).

### Request/reply and broadcast

- **Every client message gets exactly one reply.** The reply is the requested data, an `ack`, or an `error`.
- **Messages that change data reply with `ack`, then broadcast the change** to every connected client, including the sender, with `"id": null`. The sender gets the `ack` first, and the new data arrives in the broadcast.
- If a message can't be parsed, the reply has `"id": null` and error code `bad_format`.
- The broadcast buffer holds 50 messages. A client that falls further behind skips the missed broadcasts without any notice. Clients should re-fetch state (`get_students`, `get_activities`) after reconnecting.

### Client messages

| `type` | `data` | Reply | Broadcast |
| --- | --- | --- | --- |
| `get_students` | none | `students` | no |
| `get_teachers` | none | `teachers` | no |
| `get_activities` | none | `activities` | no |
| `add_student` | `{ "student_name": string, "key_stage": "ks3" \| "ks4" }` | `ack` | `student_added` |
| `add_activity` | `{ "student_id": int, "teacher_id": int, "reward_description": string, "star_diff": int }` | `ack` | `activity_added` |

**Validation**

- `student_name` is trimmed and must not be empty.
- `key_stage` is trimmed and lowercased, and must be `ks3` or `ks4`.
- `star_diff` is a signed 16-bit integer (−32768 to 32767). Use a positive value to award stars and a negative value to cash in a reward. A value outside that range fails with `bad_format`, not `invalid_input`.
- `add_activity` fails with `not_found` if the teacher or student does not exist. The star total update and the activity record are written in one transaction.
- `add_activity` does not check that `teacher_id` belongs to the logged-in teacher. It also doesn't stop a student's `total_stars` from going below zero, and it accepts an empty `reward_description`.

**Examples**

```json
{ "id": 1, "msg": { "type": "get_students" } }

{ "id": 2, "msg": { "type": "add_student",
  "data": { "student_name": "Alex Smith", "key_stage": "ks3" } } }

{ "id": 3, "msg": { "type": "add_activity",
  "data": { "student_id": 1, "teacher_id": 1,
            "reward_description": "Great homework", "star_diff": 2 } } }
```

### Server messages

| `type` | `data` | Sent as |
| --- | --- | --- |
| `students` | `Student[]` (ordered by `id`) | reply to `get_students` |
| `teachers` | `Teacher[]` (ordered by `id`) | reply to `get_teachers` |
| `activities` | `Activity[]` (newest first) | reply to `get_activities` |
| `ack` | none | reply to `add_student` and `add_activity` |
| `student_added` | `Student` | broadcast after `add_student` |
| `activity_added` | `{ "activity": Activity, "student": Student }` | broadcast after `add_activity`. `student` carries the updated `total_stars`. |
| `error` | `{ "code": string, "message": string }` | reply to any failed message |

**Examples**

```json
// Reply to get_students
{ "id": 1, "msg": { "type": "students", "data": [
  { "id": 1, "name": "Alex Smith", "ks": "ks3", "total_stars": 12 }
] } }

// Reply to add_activity
{ "id": 3, "msg": { "type": "ack" } }

// Broadcast after add_activity
{ "id": null, "msg": { "type": "activity_added", "data": {
  "activity": {
    "id": "019012ab-3c4d-7e5f-8a9b-0c1d2e3f4a5b",
    "student_id": 1,
    "teacher_id": 1,
    "reward_description": "Great homework",
    "star_diff": 2,
    "timestamp": "2026-11-05T21:07:00.123456Z"
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
| `name` | string | Not unique |
| `ks` | `"ks3"` \| `"ks4"` | Key stage |
| `total_stars` | integer | Running total (64-bit signed). Starts at 0. |

**Teacher**

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | Auto-incremented |
| `name` | string | The login username (unique). The password hash is never sent. |

**Activity**

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | UUID v7 |
| `student_id` | integer | |
| `teacher_id` | integer | |
| `reward_description` | string | |
| `star_diff` | integer | 16-bit signed. Positive = awarded, negative = cashed in |
| `timestamp` | string | RFC 3339, UTC |

### Error codes

| `code` | Meaning |
| --- | --- |
| `bad_format` | The message was not valid JSON for the protocol: an unknown `type`, a missing field, a wrong type or a number out of range. `id` is `null`, and `message` holds the parser's error text. |
| `invalid_input` | Validation failed. `message` explains why. |
| `not_found` | A referenced student or teacher does not exist |
| `conflict` | A unique constraint was violated. No current WebSocket message can trigger it. |
| `internal_error` | Something went wrong on the server. `message` is always `"internal error"`, and the details are in the server log. |

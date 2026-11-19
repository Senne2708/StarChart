//! End-to-end tests. Each test starts its own server on a free port, with its own
//! in-memory database, and talks to it over HTTP and WebSocket like a real client.

mod login;
mod support;
mod websocket;

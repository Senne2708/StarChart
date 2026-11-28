import type { CommandError } from "./protocol";

/** Normalises whatever a Tauri `invoke` rejected with into a CommandError. */
export function toCommandError(err: unknown): CommandError {
  if (typeof err === "object" && err !== null && "message" in err) {
    const e = err as Partial<CommandError>;
    return { kind: e.kind ?? "unknown", code: e.code ?? null, message: String(e.message) };
  }
  return { kind: "unknown", code: null, message: String(err) };
}

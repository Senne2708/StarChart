import type { Subject } from "./protocol";

/** Every class, in display order. Matches `Subject` in the Rust protocol crate. */
export const SUBJECTS: { value: Subject; label: string }[] = [
  { value: "art", label: "Art" },
  { value: "ilp", label: "ILP" },
  { value: "maths", label: "Maths" },
  { value: "english", label: "English" },
  { value: "humanities", label: "Humanities" },
  { value: "citizenship", label: "Citizenship" },
  { value: "vocational", label: "Vocational" },
  { value: "options", label: "Options" },
];

export function subjectLabel(subject: Subject): string {
  return SUBJECTS.find((s) => s.value === subject)?.label ?? subject;
}

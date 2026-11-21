import { atom } from "jotai";
import type { Activity, KeyStage, Student, Teacher } from "./api/protocol";

// ---------- Login state ----------
function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
export const usernameAtom = atom<string | null>(null);

export type ConnectionStatus = "disconnected" | "connecting" | "connected";
export const connectionStatusAtom = atom<ConnectionStatus>("disconnected");
export const connectionErrorAtom = atom<string | null>(null);

// ---------- Data from the server ----------
export const studentsAtom = atom<Student[]>([]);
export const teachersAtom = atom<Teacher[]>([]);
export const activitiesAtom = atom<Activity[]>([]);


export const currentTeacherAtom = atom((get) => {
  const username = get(usernameAtom);
  return get(teachersAtom).find((t) => t.name === username) ?? null;
});

export const leaderboardFilterAtom = atom<"all" | KeyStage>("all");

export const leaderboardAtom = atom((get) => {
  const filter = get(leaderboardFilterAtom);
  return get(studentsAtom)
    .filter((s) => filter === "all" || s.ks === filter)
    .sort((a, b) => b.total_stars - a.total_stars);
});

export const activityFeedAtom = atom((get) => {
  const students = get(studentsAtom);
  const teachers = get(teachersAtom);
  return [...get(activitiesAtom)]
    .sort((a, b) => b.timestamp.localeCompare(a.timestamp))
    .map((a) => ({
      ...a,
      studentName: students.find((s) => s.id === a.student_id)?.name ?? "Unknown student",
      teacherName: teachers.find((t) => t.id === a.teacher_id)?.name ?? "Unknown teacher",
    }));
});
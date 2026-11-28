import { atom } from "jotai";
import { invoke } from "@tauri-apps/api/core";
import type { Activity, KeyStage, ServerEvent, Student, Teacher } from "./api/protocol";

export const usernameAtom = atom<string | null>(null);

export type ConnectionStatus = "disconnected" | "connecting" | "connected";
export const connectionStatusAtom = atom<ConnectionStatus>("disconnected");
export const connectionErrorAtom = atom<string | null>(null);

// ---------- Data from the server ----------
export const studentsAtom = atom<Student[]>([]);
export const teachersAtom = atom<Teacher[]>([]);
export const activitiesAtom = atom<Activity[]>([]);

export const loadServerDataAtom = atom(null, async (_get, set) => {
  const [students, teachers, activities] = await Promise.all([
    invoke<Student[]>("get_students"),
    invoke<Teacher[]>("get_teachers"),
    invoke<Activity[]>("get_activities"),
  ]);
  set(studentsAtom, students);
  set(teachersAtom, teachers);
  set(activitiesAtom, activities);
});

export const applyServerEventAtom = atom(null, async (_get, set, event: ServerEvent) => {
  switch (event.type) {
    case "student_added": {
      const student = event.data;
      set(studentsAtom, (prev) => [...prev.filter((s) => s.id !== student.id), student]);
      break;
    }
    case "activity_added": {
      const { activity, student } = event.data;
      set(activitiesAtom, (prev) => [...prev.filter((a) => a.id !== activity.id), activity]);
      set(studentsAtom, (prev) => [...prev.filter((s) => s.id !== student.id), student]);
      break;
    }
    case "activity_removed": {
      const { activity_id, student } = event.data;
      set(activitiesAtom, (prev) => prev.filter((a) => a.id !== activity_id));
      set(studentsAtom, (prev) => prev.map((s) => (s.id === student.id ? student : s)));
      break;
    }
    case "student_removed": {
      const { student_id } = event.data;
      set(studentsAtom, (prev) => prev.filter((s) => s.id !== student_id));
      set(activitiesAtom, (prev) => prev.filter((a) => a.student_id !== student_id));
      break;
    }
    case "resync":
      await set(loadServerDataAtom);
      break;
  }
});

export const leaderboardFilterAtom = atom<"all" | KeyStage>("all");

/** Students in the chosen key stage, ranked by stars earned across all classes */
export const leaderboardAtom = atom((get) => {
  const filter = get(leaderboardFilterAtom);
  return get(studentsAtom)
    .filter((s) => filter === "all" || s.ks === filter)
    .sort((a, b) => b.earned_stars - a.earned_stars || a.name.localeCompare(b.name));
});

export type FeedActivity = Activity & { studentName: string; teacherName: string };

export const activityFeedAtom = atom((get): FeedActivity[] => {
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

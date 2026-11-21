// Shapes of the data, matching star_chart_server/src/database/*.rs
export type KeyStage = "ks3" | "ks4";

export type Student = {
  id: number;
  name: string;
  ks: KeyStage;
  total_stars: number;
};

export type Teacher = {
  id: number;
  name: string;
};

export type Activity = {
  id: string; // UUID
  student_id: number;
  teacher_id: number;
  reward_description: string;
  star_diff: number;
  timestamp: string; // ISO date, e.g. "2026-11-02T10:30:00Z"
};

// Client → Server (matches ClientMsgType in messaging/protocol.rs)
export type ClientMsg =
  | { type: "get_students" }
  | { type: "get_teachers" }
  | { type: "get_activities" }
  | { type: "add_student"; data: { student_name: string; key_stage: KeyStage } }
  | {
      type: "add_activity";
      data: {
        student_id: number;
        teacher_id: number;
        reward_description: string;
        star_diff: number;
      };
    };

// Server → Client (matches ServerMsgType in messaging/protocol.rs)
export type ServerMsgType =
  | { type: "error"; data: { code: string; message: string } }
  | { type: "students"; data: Student[] }
  | { type: "teachers"; data: Teacher[] }
  | { type: "activities"; data: Activity[] }
  | { type: "student_added"; data: Student }
  | { type: "activity_added"; data: { activity: Activity; student: Student } };

export type ServerMsg = {
  id: number | null; // null = broadcast to everyone
  msg: ServerMsgType;
};
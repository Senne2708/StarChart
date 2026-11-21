
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
  timestamp: string; 
};



export type ServerEvent =
  | { type: "student_added"; data: Student }
  | { type: "activity_added"; data: { activity: Activity; student: Student } }
  | { type: "resync" };


export type CommandError = { kind: string; code: string | null; message: string };
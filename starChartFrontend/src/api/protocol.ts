
export type KeyStage = "ks3" | "ks4";

/** The classes every student takes. Labels and display order are in `subjects.ts`. */
export type Subject =
  | "art"
  | "ilp"
  | "maths"
  | "english"
  | "humanities"
  | "citizenship"
  | "vocational"
  | "options";

export type Student = {
  id: number;
  name: string;
  ks: KeyStage;
  /** Stars to spend in each class (awarded - taken away - cashed in). Has every class. */
  balances: Record<Subject, number>;
  /** Awarded - taken away, across all classes. Cash-ins don't lower it. Used for ranking. */
  earned_stars: number;
};

export type Teacher = {
  id: number;
  name: string;
};

/** What a star change was for. Cash-ins lower the balance but not the earned score. */
export type ActivityKind = "reward" | "take_away" | "cash_in";

export type Activity = {
  id: string; // UUID
  student_id: number;
  teacher_id: number;
  reward_description: string;
  star_diff: number;
  timestamp: string;
  kind: ActivityKind;
  subject: Subject;
};

export type ServerEvent =
  | { type: "student_added"; data: Student }
  | { type: "activity_added"; data: { activity: Activity; student: Student } }
  | { type: "activity_removed"; data: { activity_id: string; student: Student } }
  | { type: "student_removed"; data: { student_id: number } }
  | { type: "resync" };


export type CommandError = { kind: string; code: string | null; message: string };

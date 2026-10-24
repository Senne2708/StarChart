import { atom } from 'jotai';

export type Student = {
  id: string;
  name: string;
  keyStage: string
  stars: number
};

export type Teacher = {
  id: string;
  name: string;
};

export type Activity = {
  id: string;
  studentId: string;
  studentName: string;
  teacherName: string;
  stars: number;
  reward?: string;   // optional — not every award has one (e.g. "Sweets")
  time: string;
  
};


export const studentAtom = atom<Student | null>(null);
export const teacherAtom = atom<Teacher | null>(null);
export const activityAtom = atom<Activity | null>(null);

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::auth::password::hash_password;

#[derive(Serialize, Clone)]
pub struct Student {
    pub student_id: usize,
    pub student_name: String,
    pub total_stars: i16,
}

#[derive(Serialize, Clone)]
pub struct Teacher {
    pub teacher_id: usize,
    pub teacher_name: String,
}

#[derive(Serialize, Clone)]
pub struct Activity {
    pub id: usize,
    pub student_id: usize,
    pub teacher_id: usize,
    pub star_diff: i16,
}



pub struct Store {
    pub students: BTreeMap<usize, Student>,
    pub teachers: BTreeMap<usize, Teacher>,
    pub activities: Vec<Activity>,
    // Simply hashing username and password
    users: HashMap<String, String>,
    next_student: usize,
    next_teacher: usize,
    next_activity: usize,
}

impl Store {
    pub fn new() -> Self {
        let mut store = Self {
            students: BTreeMap::new(),
            teachers: BTreeMap::new(),
            activities: Vec::new(),
            users: HashMap::new(),
            next_student: 1,
            next_teacher: 1,
            next_activity: 1,
        };

        for name in ["Test Student 1", "Test Student 2", "Test Student 3"] {
            store.add_student(name);
        }
        for name in ["zanzi", "senne", "alex"] {
            store.add_teacher(name);
            store.add_user(name, "password");
        }

        store
    }

    pub fn add_user(&mut self, name: &str, password: &str) {
        self.users.insert(name.to_string(), hash_password(password).unwrap());
    }

    pub fn get_user(&self, name: &str) -> Option<&String> {
        self.users.get(name)
    }

    pub fn add_student(&mut self, student_name: impl Into<String>) -> Student {
        let student = Student {
            student_id: self.next_student,
            student_name: student_name.into(),
            total_stars: 0,
        };
        self.students.insert(student.student_id, student.clone());
        self.next_student += 1;
        student
    }

    pub fn add_teacher(&mut self, teacher_name: impl Into<String>) -> Teacher {
        let teacher = Teacher {
            teacher_id: self.next_teacher,
            teacher_name: teacher_name.into(),
        };
        self.teachers.insert(teacher.teacher_id, teacher.clone());
        self.next_teacher += 1;
        teacher
    }

    /// Records the activity and applies the star change to the student.
    /// Returns the new activity plus the updated student.
    pub fn add_activity(
        &mut self,
        student_id: usize,
        teacher_id: usize,
        star_diff: i16,
    ) -> Result<(Activity, Student), &'static str> {
        if !self.teachers.contains_key(&teacher_id) {
            return Err("teacher_not_found");
        }
        let student = self
            .students
            .get_mut(&student_id)
            .ok_or("student_not_found")?;

        student.total_stars = student.total_stars.saturating_add(star_diff);
        let student = student.clone();

        let activity = Activity {
            id: self.next_activity,
            student_id,
            teacher_id,
            star_diff,
        };
        self.activities.push(activity.clone());
        self.next_activity += 1;

        Ok((activity, student))
    }
}

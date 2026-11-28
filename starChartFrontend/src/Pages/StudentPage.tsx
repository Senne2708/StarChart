import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAtomValue } from "jotai";
import { ActionIcon, Alert, Paper, Text, Group, Stack, TextInput, Select, Button, Title, Grid } from "@mantine/core";

import { activitiesAtom, studentsAtom, usernameAtom } from "../atoms";
import { toCommandError } from "../api/errors";
import type { KeyStage, Student } from "../api/protocol";
import ConfirmModal from "../Modal/ConfirmModal";

const KEY_STAGES: { label: string; value: KeyStage }[] = [
  { label: "KS3", value: "ks3" },
  { label: "KS4", value: "ks4" },
];

// Matches the server's limit on student names
const MAX_NAME_LENGTH = 100;

function StudentPage() {
  const students = useAtomValue(studentsAtom);
  const username = useAtomValue(usernameAtom);

  const [newName, setNewName] = useState("");
  const [newKeyStage, setNewKeyStage] = useState<KeyStage | null>("ks3");
  const [loading, setLoading] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [toRemove, setToRemove] = useState<Student | null>(null);

  const activities = useAtomValue(activitiesAtom);
  const activityCount = toRemove ? activities.filter((a) => a.student_id === toRemove.id).length : 0;

  const trimmedName = newName.trim();
  const canSubmit = username !== null && trimmedName.length > 0 && newKeyStage !== null && !loading;

  const sortedStudents = [...students].sort((a, b) => a.name.localeCompare(b.name));

  const handleAddStudent = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;

    setLoading(true);
    setFormError(null);
    try {
      // The new student comes back as a server event and is added to the list then
      await invoke("add_student", { studentName: trimmedName, keyStage: newKeyStage });
      setNewName(""); // Reset form
    } catch (err) {
      setFormError(toCommandError(err).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Stack h="100%">
      <Title order={2} c="violet.9">🧑‍🎓 Manage Students</Title>

      {/* Add Student Form */}
      <Paper shadow="sm" p="md" radius="md" withBorder bg="violet.0">
        <Title order={4} mb="sm" c="violet.9">✨ Add New Student</Title>
        <form onSubmit={handleAddStudent}>
          {formError && <Alert color="red" variant="light" mb="sm">{formError}</Alert>}
          {!username && <Alert color="orange" variant="light" mb="sm">Log in to add students.</Alert>}
          <Grid align="flex-end">
            <Grid.Col span={{ base: 12, sm: 6 }}>
              <TextInput
                label="Student Name"
                placeholder="e.g. John Doe"
                value={newName}
                onChange={(e) => setNewName(e.currentTarget.value)}
                maxLength={MAX_NAME_LENGTH}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 12, sm: 4 }}>
              <Select
                label="Key Stage"
                placeholder="Select KS"
                data={KEY_STAGES}
                value={newKeyStage}
                onChange={(value) => setNewKeyStage(value as KeyStage | null)}
                allowDeselect={false}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 12, sm: 2 }}>
              <Button type="submit" fullWidth color="violet" loading={loading} disabled={!canSubmit}>
                Add ➕
              </Button>
            </Grid.Col>
          </Grid>
        </form>
      </Paper>

      {/* Student List/Grid */}
      <Title order={4} mt="md" c="gray.7">📋 Current Students</Title>
      {sortedStudents.length === 0 && (
        <Text c="dimmed" ta="center">No students yet 🎓</Text>
      )}
      <Grid>
        {sortedStudents.map((student) => (
          <Grid.Col span={{ base: 12, md: 6 }} key={student.id}>
            <Paper shadow="xs" p="sm" withBorder radius="md">
              <Group justify="space-between">
                <div>
                  <Text fw={600}>👤 {student.name}</Text>
                  <Text size="sm" c="dimmed">📚 Key Stage: {student.ks.toUpperCase()}</Text>
                </div>
                <Group gap="xs">
                  <Text fw={700} c="violet.7" title="Stars earned across all classes">⭐ {student.earned_stars}</Text>
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    disabled={!username}
                    onClick={() => setToRemove(student)}
                    aria-label={`Remove ${student.name}`}
                  >
                    🗑️
                  </ActionIcon>
                </Group>
              </Group>
            </Paper>
          </Grid.Col>
        ))}
      </Grid>

      <ConfirmModal
        opened={toRemove !== null}
        title="🗑️ Remove student"
        confirmLabel="Remove student"
        onClose={() => setToRemove(null)}
        onConfirm={async () => {
          if (toRemove) await invoke("remove_student", { studentId: toRemove.id });
        }}
      >
        {toRemove && (
          <Text size="sm">
            Remove <b>{toRemove.name}</b> ({toRemove.ks.toUpperCase()})? This also deletes their{" "}
            {activityCount} {activityCount === 1 ? "activity" : "activities"} and can't be undone.
          </Text>
        )}
      </ConfirmModal>
    </Stack>
  );
}

export default StudentPage;

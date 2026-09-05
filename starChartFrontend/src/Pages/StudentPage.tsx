import { useState } from "react";
import { Paper, Text, Group, Stack, TextInput, Select, Button, Title, Grid, ActionIcon } from "@mantine/core";

const INITIAL_STUDENTS = [
  { id: 1, name: "Alice Johnson", keyStage: "KS3" },
  { id: 2, name: "Tom Baker", keyStage: "KS4" },
];

function StudentPage() {
  const [students, setStudents] = useState(INITIAL_STUDENTS);
  const [newName, setNewName] = useState("");
  const [newKeyStage, setNewKeyStage] = useState<string | null>("KS3");

  const handleAddStudent = () => {
    if (!newName.trim() || !newKeyStage) return;
    
    const newStudent = {
      id: Date.now(),
      name: newName,
      keyStage: newKeyStage,
    };
    
    setStudents([...students, newStudent]);
    setNewName(""); // Reset form
  };

  const handleDelete = (id: number) => {
    setStudents(students.filter(student => student.id !== id));
  };

  return (
    <Stack h="100%">
      <Title order={2} c="violet.9">🧑‍🎓 Manage Students</Title>

      {/* Add Student Form */}
      <Paper shadow="sm" p="md" radius="md" withBorder bg="violet.0">
        <Title order={4} mb="sm" c="violet.9">✨ Add New Student</Title>
        <Grid align="flex-end">
          <Grid.Col span={{ base: 12, sm: 6 }}>
            <TextInput
              label="Student Name"
              placeholder="e.g. John Doe"
              value={newName}
              onChange={(e) => setNewName(e.currentTarget.value)}
            />
          </Grid.Col>
          <Grid.Col span={{ base: 12, sm: 4 }}>
            <Select
              label="Key Stage"
              placeholder="Select KS"
              data={["KS3", "KS4"]}
              value={newKeyStage}
              onChange={setNewKeyStage}
            />
          </Grid.Col>
          <Grid.Col span={{ base: 12, sm: 2 }}>
            <Button fullWidth color="violet" onClick={handleAddStudent}>
              Add ➕
            </Button>
          </Grid.Col>
        </Grid>
      </Paper>

      {/* Student List/Grid */}
      <Title order={4} mt="md" c="gray.7">📋 Current Students</Title>
      <Grid>
        {students.map((student) => (
          <Grid.Col span={{ base: 12, md: 6 }} key={student.id}>
            <Paper shadow="xs" p="sm" withBorder radius="md">
              <Group justify="space-between">
                <div>
                  <Text fw={600}>👤 {student.name}</Text>
                  <Text size="sm" c="dimmed">📚 Key Stage: {student.keyStage}</Text>
                </div>
                <ActionIcon color="red" variant="subtle" onClick={() => handleDelete(student.id)}>
                  🗑️
                </ActionIcon>
              </Group>
            </Paper>
          </Grid.Col>
        ))}
      </Grid>
    </Stack>
  );
}
export default StudentPage;

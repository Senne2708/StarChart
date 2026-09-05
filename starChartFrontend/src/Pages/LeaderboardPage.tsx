import { useState } from "react";
import { Paper, Text, Group, Stack, ActionIcon, SegmentedControl, Title, Badge } from "@mantine/core";

// Mock data for the leaderboard
const INITIAL_STUDENTS = [
  { id: 1, name: "Alice Johnson", keyStage: "KS3", stars: 45 },
  { id: 2, name: "Tom Baker", keyStage: "KS4", stars: 32 },
  { id: 3, name: "Sarah Lee", keyStage: "KS3", stars: 50 },
  { id: 4, name: "James Smith", keyStage: "KS4", stars: 28 },
];


function LeaderboardPage() {
  const [students, setStudents] = useState(INITIAL_STUDENTS);
  const [filter, setFilter] = useState("All");

  const handleUpdateStars = (id: number, amount: number) => {
    setStudents((prev) =>
      prev.map((student) =>
        student.id === id ? { ...student, stars: Math.max(0, student.stars + amount) } : student
      )
    );
  };

  const filteredStudents = students
    .filter((s) => filter === "All" || s.keyStage === filter)
    .sort((a, b) => b.stars - a.stars); // Sort by highest stars

  return (
    <Stack h="100%">
      <Title order={2} c="violet.9">🏆 Leaderboard</Title>

      <SegmentedControl
        value={filter}
        onChange={setFilter}
        data={["All", "KS3", "KS4"]}
        color="violet"
        fullWidth
      />

      <Stack gap="sm" mt="md">
        {filteredStudents.map((student, index) => (
          <Paper key={student.id} shadow="xs" p="sm" withBorder radius="md">
            <Group justify="space-between" wrap="nowrap">
              
              {/* Left Side: Rank, Name, and Key Stage */}
              <Group gap="sm">
                <Badge color={index === 0 ? "yellow" : "gray"} variant="filled" circle>
                  {index + 1}
                </Badge>
                <div>
                  <Text fw={600}>{student.name}</Text>
                  <Text size="xs" c="dimmed">📚 {student.keyStage}</Text>
                </div>
              </Group>

              {/* Right Side: Stars and Actions */}
              <Group gap="xs">
                <Text fw={700} c="violet.7" w={60} ta="right">
                  ⭐ {student.stars}
                </Text>
                
                <ActionIcon 
                  variant="light" 
                  color="red" 
                  onClick={() => handleUpdateStars(student.id, -1)}
                >
                  ➖
                </ActionIcon>
                <ActionIcon 
                  variant="light" 
                  color="green" 
                  onClick={() => handleUpdateStars(student.id, 1)}
                >
                  ➕
                </ActionIcon>
              </Group>
              
            </Group>
          </Paper>
        ))}
      </Stack>
    </Stack>
  );
}


export default LeaderboardPage;


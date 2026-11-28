import { useAtom, useAtomValue } from "jotai";
import { Paper, Text, Group, Stack, SegmentedControl, Title, Badge, SimpleGrid } from "@mantine/core";

import { leaderboardAtom, leaderboardFilterAtom } from "../atoms";
import { SUBJECTS } from "../api/subjects";
import type { KeyStage } from "../api/protocol";

const FILTERS: { label: string; value: "all" | KeyStage }[] = [
  { label: "All", value: "all" },
  { label: "KS3", value: "ks3" },
  { label: "KS4", value: "ks4" },
];

function LeaderboardPage() {
  // Already filtered by key stage and ranked by earned stars
  const students = useAtomValue(leaderboardAtom);
  const [filter, setFilter] = useAtom(leaderboardFilterAtom);

  return (
    <Stack h="100%">
      <Title order={2} c="violet.9">🏆 Leaderboard</Title>

      <SegmentedControl
        value={filter}
        onChange={(value) => setFilter(value as "all" | KeyStage)}
        data={FILTERS}
        color="violet"
        fullWidth
      />

      <Stack gap="sm" mt="md">
        {students.length === 0 && (
          <Text c="dimmed" ta="center" mt="xl">No students yet 🎓</Text>
        )}

        {students.map((student, index) => (
          <Paper key={student.id} shadow="xs" p="sm" withBorder radius="md">
            <Group justify="space-between" wrap="nowrap">

              {/* Rank, name and key stage */}
              <Group gap="sm">
                <Badge color={index === 0 ? "yellow" : "gray"} variant="filled" circle>
                  {index + 1}
                </Badge>
                <div>
                  <Text fw={600}>{student.name}</Text>
                  <Text size="xs" c="dimmed">📚 {student.ks.toUpperCase()}</Text>
                </div>
              </Group>

              {/* Earned across all classes, used for the ranking */}
              <Stack gap={0} align="flex-end">
                <Text fw={700} c="violet.7">⭐ {student.earned_stars}</Text>
                <Text size="xs" c="dimmed">Earned</Text>
              </Stack>

            </Group>

            {/* Stars left to spend in each class */}
            <SimpleGrid cols={4} spacing="xs" verticalSpacing={2} mt="xs">
              {SUBJECTS.map(({ value, label }) => {
                const balance = student.balances[value] ?? 0;
                return (
                  <Group key={value} gap={4} wrap="nowrap" justify="space-between">
                    <Text size="xs" c={balance === 0 ? "dimmed" : "gray.7"}>{label}</Text>
                    <Text size="xs" fw={700} c={balance === 0 ? "dimmed" : "orange.8"}>{balance}</Text>
                  </Group>
                );
              })}
            </SimpleGrid>
          </Paper>
        ))}
      </Stack>
    </Stack>
  );
}

export default LeaderboardPage;

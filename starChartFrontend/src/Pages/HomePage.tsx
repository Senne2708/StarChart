import { ScrollArea, Paper, Text, Group, Stack, Badge, Title } from "@mantine/core";

const RECENT_REWARDS = [
  { id: 1, student: "Alice Johnson", teacher: "Mrs. Gunton", stars: 5, time: "10:30 AM", reward: "Sweets" },
  { id: 2, student: "Tom Baker", teacher: "Mrs. Sarah", stars: 10, time: "11:15 AM", reward: "Cafeteria Voucher" },
  { id: 3, student: "Sarah Lee", teacher: "Mr. Smith", stars: 3, time: "01:00 PM", reward: "Stationery Pack" },
];

function HomePage() {
  return (
    <Stack h="100%">
      <Title order={2} c="violet.9">🏠 Recent Activity</Title>
      
      {/* ScrollArea allows the list to scroll if it gets too long */}
      <ScrollArea h={400} offsetScrollbars>
        <Stack gap="md">
          {RECENT_REWARDS.map((record) => (
            <Paper key={record.id} shadow="sm" radius="md" p="md" withBorder>
              <Group justify="space-between" mb="xs">
                <Text fw={700} size="lg">🎓 {record.student}</Text>
                <Badge color="violet" size="lg" variant="light">
                  ⭐ {record.stars} Stars
                </Badge>
              </Group>
              
              <Group justify="space-between">
                <Stack gap={0}>
                  <Text size="sm" c="dimmed">🧑‍🏫 Teacher: {record.teacher}</Text>
                  <Text size="sm" c="dimmed">⏰ Time: {record.time}</Text>
                </Stack>
                <Badge color="green" variant="outline">
                  🎁 {record.reward}
                </Badge>
              </Group>
            </Paper>
          ))}
        </Stack>
      </ScrollArea>
    </Stack>
  );
}

export default HomePage;

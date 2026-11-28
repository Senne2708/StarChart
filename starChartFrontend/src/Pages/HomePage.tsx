import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ScrollArea, Paper, Text, Group, Stack, Badge, Title, Button, ActionIcon } from "@mantine/core";
import { useAtomValue } from "jotai";
import { activityFeedAtom, usernameAtom, type FeedActivity } from "../atoms";
import StarsModal from "../Modal/StarsModal";
import ConfirmModal from "../Modal/ConfirmModal";
import { subjectLabel } from "../api/subjects";

/** Shows the time for today's activities, and the date as well for older ones. */
function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;

  const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (date.toDateString() === new Date().toDateString()) return time;
  return `${date.toLocaleDateString([], { day: "numeric", month: "short" })}, ${time}`;
}

function formatStars(starDiff: number): string {
  const sign = starDiff > 0 ? "+" : starDiff < 0 ? "−" : "";
  const count = Math.abs(starDiff);
  return `${sign}${count} ${count === 1 ? "Star" : "Stars"}`;
}

function HomePage() {
  // Newest first, with student and teacher names already looked up
  const activities = useAtomValue(activityFeedAtom);
  const username = useAtomValue(usernameAtom);
  const [starsOpen, setStarsOpen] = useState(false);
  const [toRemove, setToRemove] = useState<FeedActivity | null>(null);

  return (
    <Stack h="100%">
      <Group justify="space-between">
        <Title order={2} c="violet.9">🏠 Recent Activity</Title>
        <Button color="violet" disabled={!username} onClick={() => setStarsOpen(true)}>
          ⭐ Give stars
        </Button>
      </Group>

      {/* ScrollArea allows the list to scroll if it gets too long */}
      <ScrollArea flex={2} offsetScrollbars>
        <Stack gap="md">
          {activities.length === 0 && (
            <Text c="dimmed" ta="center" mt="xl">No activity yet ✨</Text>
          )}

          {activities.map((activity) => {
            const cashedIn = activity.kind === "cash_in";
            return (
              <Paper key={activity.id} shadow="sm" radius="md" p="md" withBorder>
                <Group justify="space-between" mb="xs">
                  <Group gap="xs">
                    <Text fw={700} size="lg">🎓 {activity.studentName}</Text>
                    <Badge color="blue" variant="light">📘 {subjectLabel(activity.subject)}</Badge>
                  </Group>
                  <Group gap="xs">
                    <Badge
                      color={cashedIn ? "orange" : activity.star_diff < 0 ? "red" : "violet"}
                      size="lg"
                      variant="light"
                    >
                      ⭐ {formatStars(activity.star_diff)}
                    </Badge>
                    <ActionIcon
                      variant="subtle"
                      color="gray"
                      disabled={!username}
                      onClick={() => setToRemove(activity)}
                      aria-label={`Remove this activity for ${activity.studentName}`}
                    >
                      🗑️
                    </ActionIcon>
                  </Group>
                </Group>

                <Group justify="space-between">
                  <Stack gap={0}>
                    <Text size="sm" c="dimmed">🧑‍🏫 Teacher: {activity.teacherName}</Text>
                    <Text size="sm" c="dimmed">⏰ Time: {formatTimestamp(activity.timestamp)}</Text>
                  </Stack>
                  <Badge color={cashedIn ? "orange" : "green"} variant="outline">
                    {cashedIn ? "🎁 Cashed in: " : "📝 "}{activity.reward_description}
                  </Badge>
                </Group>
              </Paper>
            );
          })}
        </Stack>
      </ScrollArea>

      <StarsModal opened={starsOpen} onClose={() => setStarsOpen(false)} />

      <ConfirmModal
        opened={toRemove !== null}
        title="🗑️ Remove activity"
        confirmLabel="Remove"
        onClose={() => setToRemove(null)}
        onConfirm={async () => {
          if (toRemove) await invoke("remove_activity", { activityId: toRemove.id });
        }}
        describeError={(error) =>
          error.code === "invalid_input"
            ? `${toRemove?.studentName} has already spent some of these ${toRemove ? subjectLabel(toRemove.subject) : ""} stars. Remove a later ${toRemove ? subjectLabel(toRemove.subject) : ""} cash-in or take-away first.`
            : error.message
        }
      >
        {toRemove && (
          <Text size="sm">
            Remove <b>{formatStars(toRemove.star_diff)}</b> in <b>{subjectLabel(toRemove.subject)}</b>{" "}
            ({toRemove.reward_description}) for <b>{toRemove.studentName}</b>? Their{" "}
            {subjectLabel(toRemove.subject)} balance will change by <b>{formatStars(-toRemove.star_diff)}</b>.
          </Text>
        )}
      </ConfirmModal>
    </Stack>
  );
}

export default HomePage;

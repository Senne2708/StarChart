import { useState } from "react";
import { AppShell, Group, UnstyledButton, Text, Title } from "@mantine/core";

import HomePage from "./Pages/HomePage";
import LeaderboardPage from "./Pages/LeaderboardPage";
import StudentPage from "./Pages/StudentPage";

const PAGES = ["home", "leaderboard", "student"];

const PAGE_EMOJIS: Record<string, string> = {
  home: "🏠",
  leaderboard: "🏆",
  student: "🎓",
};

export default function App() {
  const [activeIndex, setActiveIndex] = useState(0);
  const activePage = PAGES[activeIndex];

  const renderMainContent = () => {
    switch (activePage) {
      case "leaderboard": return <LeaderboardPage />;
      case "student": return <StudentPage />;
      default: return <HomePage />;
    }
  };

  return (
    <AppShell
      header={{ height: 60 }}
      footer={{ height: 60 }}
      padding="md"
      bg="gray.0" // Adds a subtle off-white background to the main app area
    >
      {/* Styled the header with a deep theme color and added emojis */}
      <AppShell.Header bg="orange.9" withBorder={false}>
        <Group h="100%" px="md" align="center" justify="center">
          <Title order={3} c="white">Star Chart 🌟</Title>
        </Group>
      </AppShell.Header>

      <AppShell.Main style={{ touchAction: "pan-y" }}>
        {renderMainContent()}
      </AppShell.Main>

      {/* Styled the footer with a clean border matching the theme */}
      <AppShell.Footer bg="white" style={{ borderTop: "2px solid var(--mantine-color-orange-2)" }}>
        <Group h="100%" px="sm" grow gap="0">
          {PAGES.map((page, index) => (
            <UnstyledButton
              key={page}
              onClick={() => setActiveIndex(index)}
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                color: activeIndex === index ? "var(--mantine-color-orange-7)" : "var(--mantine-color-gray-5)",
                transition: "color 0.2s ease",
              }}
            >
              {/* Inserted the emoji above the text */}
              <Text size="xl" mb={2}>{PAGE_EMOJIS[page]}</Text>
              <Text size="xs" fw={activeIndex === index ? 800 : 600} tt="capitalize">
                {page}
              </Text>
            </UnstyledButton>
          ))}
        </Group>
      </AppShell.Footer>
    </AppShell>
  );
}

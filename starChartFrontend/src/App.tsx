import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAtom, useSetAtom } from "jotai";
import { AppShell, Badge, Box, Button, Group, Menu, UnstyledButton, Text, Title } from "@mantine/core";

import HomePage from "./Pages/HomePage";
import LeaderboardPage from "./Pages/LeaderboardPage";
import StudentPage from "./Pages/StudentPage";
import LoginModal from "./Modal/LoginModal";
import CreateLoginModal from "./Modal/CreateLoginModal";
import type { ServerEvent } from "./api/protocol";
import {
  activitiesAtom,
  applyServerEventAtom,
  connectionErrorAtom,
  connectionStatusAtom,
  studentsAtom,
  teachersAtom,
  usernameAtom,
} from "./atoms";

const PAGES = ["home", "leaderboard", "student"];

const PAGE_EMOJIS: Record<string, string> = {
  home: "🏠",
  leaderboard: "🏆",
  student: "🎓",
};

type OpenModal = "login" | "create" | null;
type LoginNotice = { color: string; message: string } | null;

export default function App() {
  const [activeIndex, setActiveIndex] = useState(0);
  const activePage = PAGES[activeIndex];

  const [username, setUsername] = useAtom(usernameAtom);
  const setConnectionStatus = useSetAtom(connectionStatusAtom);
  const [connectionError, setConnectionError] = useAtom(connectionErrorAtom);
  const setStudents = useSetAtom(studentsAtom);
  const setTeachers = useSetAtom(teachersAtom);
  const setActivities = useSetAtom(activitiesAtom);
  const applyServerEvent = useSetAtom(applyServerEventAtom);

  const [openModal, setOpenModal] = useState<OpenModal>("login");
  const [loginUsername, setLoginUsername] = useState("");
  const [loginNotice, setLoginNotice] = useState<LoginNotice>(null);

  const clearSession = () => {
    setUsername(null);
    setConnectionStatus("disconnected");
    setStudents([]);
    setTeachers([]);
    setActivities([]);
  };

  useEffect(() => {
    const unlisten = listen("server-disconnected", () => {
      clearSession();
      setConnectionError("Lost connection to the server. Please log in again.");
      setOpenModal("login");
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<ServerEvent>("server-event", (event) => {
      applyServerEvent(event.payload).catch((err) => {
        console.error("Failed to apply server event", err);
      });
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  const openLogin = (prefill = username ?? "", notice: LoginNotice = null) => {
    setLoginUsername(prefill);
    setLoginNotice(notice);
    setOpenModal("login");
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
    } finally {
      clearSession();
      setConnectionError(null);
    }
  };

  const handleCreated = (newUsername: string) => {
    openLogin(newUsername, { color: "green", message: `Account "${newUsername}" created. You can log in now.` });
  };

  const closeModal = () => {
    setOpenModal(null);
    setConnectionError(null);
  };

  const renderMainContent = () => {
    switch (activePage) {
      case "leaderboard": return <LeaderboardPage />;
      case "student": return <StudentPage />;
      default: return <HomePage />;
    }
  };

  const notice: LoginNotice = connectionError ? { color: "orange", message: connectionError } : loginNotice;

  return (
    <AppShell
      header={{ height: 60 }}
      footer={{ height: 60 }}
      padding="md"
      bg="gray.0"
    >
      <AppShell.Header bg="orange.9" withBorder={false}>
        <Group h="100%" px="md" align="center" justify="center" pos="relative">
          <Title order={3} c="white">Star Chart 🌟</Title>

          <Box pos="absolute" right={16}>
            {username ? (
              <Menu position="bottom-end" shadow="md" width={160}>
                <Menu.Target>
                  <UnstyledButton>
                    <Badge color="white" c="orange.9" size="lg" variant="filled" style={{ cursor: "pointer" }}>
                      🧑‍🏫 {username}
                    </Badge>
                  </UnstyledButton>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Item color="red" onClick={handleLogout}>Log out 👋</Menu.Item>
                </Menu.Dropdown>
              </Menu>
            ) : (
              <Button size="xs" variant="white" color="orange.9" onClick={() => openLogin()}>
                Log in 🔐
              </Button>
            )}
          </Box>
        </Group>
      </AppShell.Header>

      <AppShell.Main style={{ touchAction: "pan-y" }}>
        {renderMainContent()}
      </AppShell.Main>

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
              <Text size="xl" mb={2}>{PAGE_EMOJIS[page]}</Text>
              <Text size="xs" fw={activeIndex === index ? 800 : 600} tt="capitalize">
                {page}
              </Text>
            </UnstyledButton>
          ))}
        </Group>
      </AppShell.Footer>

      <LoginModal
        opened={openModal === "login"}
        onClose={closeModal}
        onSwitchToCreate={() => setOpenModal("create")}
        initialUsername={loginUsername}
        notice={notice}
      />
      <CreateLoginModal
        opened={openModal === "create"}
        onClose={closeModal}
        onSwitchToLogin={() => openLogin("")}
        onCreated={handleCreated}
      />
    </AppShell>
  );
}

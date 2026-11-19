import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Paper, Text, Group, Stack, TextInput, PasswordInput, Select, Button, Title, Grid, Badge } from "@mantine/core";

// Matches the types in star_chart_protocol
type KeyStage = "ks3" | "ks4";
type Student = { id: number; name: string; ks: KeyStage; total_stars: number };
type ServerEvent =
  | { type: "student_added"; data: Student }
  | { type: "activity_added"; data: { student: Student } }
  | { type: "resync" };

type CommandError = { kind: string; code: string | null; message: string };

function errorMessage(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return (err as CommandError).message;
  }
  return String(err);
}

function TestPage() {
  const [loggedInAs, setLoggedInAs] = useState<string | null>(null);
  const [students, setStudents] = useState<Student[]>([]);
  const [log, setLog] = useState<string[]>([]);

  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [adminCode, setAdminCode] = useState("");

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const [studentName, setStudentName] = useState("");
  const [keyStage, setKeyStage] = useState<string | null>("ks3");

  const addLog = (line: string) => {
    const time = new Date().toLocaleTimeString();
    setLog((prev) => [`${time}  ${line}`, ...prev].slice(0, 20));
  };

  const refreshStudents = async () => {
    try {
      setStudents(await invoke<Student[]>("get_students"));
      addLog("✅ get_students");
    } catch (err) {
      addLog(`❌ get_students: ${errorMessage(err)}`);
    }
  };

  useEffect(() => {
    const unlisteners = [
      listen<ServerEvent>("server-event", ({ payload }) => {
        addLog(`📡 ${payload.type}`);
        if (payload.type === "student_added") {
          const added = payload.data;
          setStudents((prev) => (prev.some((s) => s.id === added.id) ? prev : [...prev, added]));
        } else if (payload.type === "activity_added") {
          const updated = payload.data.student;
          setStudents((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
        } else if (payload.type === "resync") {
          refreshStudents();
        }
      }),
      listen("server-disconnected", () => {
        addLog("🔌 disconnected from the server");
        setLoggedInAs(null);
      }),
    ];
    return () => {
      unlisteners.forEach((unlisten) => unlisten.then((stop) => stop()));
    };
  }, []);

  const handleCreateLogin = async () => {
    try {
      await invoke("create_login", { username: newUsername, password: newPassword, adminCode });
      addLog(`✅ create_login: ${newUsername.trim()}`);
      setUsername(newUsername.trim());
      setNewPassword("");
    } catch (err) {
      addLog(`❌ create_login: ${errorMessage(err)}`);
    }
  };

  const handleLogin = async () => {
    try {
      await invoke("login", { username, password });
      addLog(`✅ login: ${username}`);
      setLoggedInAs(username);
      setPassword("");
      await refreshStudents();
    } catch (err) {
      addLog(`❌ login: ${errorMessage(err)}`);
    }
  };

  const handleLogout = async () => {
    await invoke("logout");
    addLog("👋 logout");
    setLoggedInAs(null);
    setStudents([]);
  };

  const handleAddStudent = async () => {
    try {
      await invoke("add_student", { studentName, keyStage });
      addLog(`✅ add_student: ${studentName.trim()}`);
      setStudentName("");
      // No need to update the list here, the student_added broadcast does it
    } catch (err) {
      addLog(`❌ add_student: ${errorMessage(err)}`);
    }
  };

  return (
    <Stack h="100%">
      <Group justify="space-between">
        <Title order={2} c="violet.9">🧪 Server Test</Title>
        {loggedInAs ? (
          <Group gap="xs">
            <Badge color="green" size="lg" variant="light">🟢 {loggedInAs}</Badge>
            <Button size="xs" variant="light" color="red" onClick={handleLogout}>Log out</Button>
          </Group>
        ) : (
          <Badge color="gray" size="lg" variant="light">⚪ Not logged in</Badge>
        )}
      </Group>

      <Grid>
        {/* Create Login */}
        <Grid.Col span={{ base: 12, sm: 6 }}>
          <Paper shadow="sm" p="md" radius="md" withBorder bg="violet.0" h="100%">
            <Title order={4} mb="sm" c="violet.9">🆕 Create Login</Title>
            <Stack gap="xs">
              <TextInput label="Username" value={newUsername} onChange={(e) => setNewUsername(e.currentTarget.value)} />
              <PasswordInput label="Password" description="8 to 128 characters" value={newPassword} onChange={(e) => setNewPassword(e.currentTarget.value)} />
              <PasswordInput label="Admin code" description="debug_password in debug builds" value={adminCode} onChange={(e) => setAdminCode(e.currentTarget.value)} />
              <Button color="violet" onClick={handleCreateLogin}>Create ➕</Button>
            </Stack>
          </Paper>
        </Grid.Col>

        {/* Login */}
        <Grid.Col span={{ base: 12, sm: 6 }}>
          <Paper shadow="sm" p="md" radius="md" withBorder bg="violet.0" h="100%">
            <Title order={4} mb="sm" c="violet.9">🔑 Login</Title>
            <Stack gap="xs">
              <TextInput label="Username" value={username} onChange={(e) => setUsername(e.currentTarget.value)} />
              <PasswordInput label="Password" value={password} onChange={(e) => setPassword(e.currentTarget.value)} />
              <Button color="violet" onClick={handleLogin}>Log in 🔓</Button>
            </Stack>
          </Paper>
        </Grid.Col>
      </Grid>

      {/* Add Student */}
      <Paper shadow="sm" p="md" radius="md" withBorder bg="violet.0">
        <Title order={4} mb="sm" c="violet.9">✨ Add Student</Title>
        <Grid align="flex-end">
          <Grid.Col span={{ base: 12, sm: 6 }}>
            <TextInput label="Student Name" placeholder="e.g. John Doe" value={studentName} onChange={(e) => setStudentName(e.currentTarget.value)} />
          </Grid.Col>
          <Grid.Col span={{ base: 12, sm: 4 }}>
            <Select
              label="Key Stage"
              data={[{ value: "ks3", label: "KS3" }, { value: "ks4", label: "KS4" }]}
              value={keyStage}
              onChange={setKeyStage}
            />
          </Grid.Col>
          <Grid.Col span={{ base: 12, sm: 2 }}>
            <Button fullWidth color="violet" onClick={handleAddStudent} disabled={!loggedInAs}>Add ➕</Button>
          </Grid.Col>
        </Grid>
      </Paper>

      {/* Students from the server */}
      <Group justify="space-between" mt="md">
        <Title order={4} c="gray.7">📋 Students on the server</Title>
        <Button size="xs" variant="light" color="violet" onClick={refreshStudents} disabled={!loggedInAs}>Refresh 🔄</Button>
      </Group>
      <Grid>
        {students.map((student) => (
          <Grid.Col span={{ base: 12, md: 6 }} key={student.id}>
            <Paper shadow="xs" p="sm" withBorder radius="md">
              <Group justify="space-between">
                <div>
                  <Text fw={600}>👤 {student.name}</Text>
                  <Text size="sm" c="dimmed">📚 {student.ks.toUpperCase()} · #{student.id}</Text>
                </div>
                <Text fw={700} c="violet.7">⭐ {student.total_stars}</Text>
              </Group>
            </Paper>
          </Grid.Col>
        ))}
      </Grid>
      {loggedInAs && students.length === 0 && <Text size="sm" c="dimmed">No students yet.</Text>}

      {/* Log */}
      <Title order={4} mt="md" c="gray.7">📜 Log</Title>
      <Paper shadow="xs" p="sm" withBorder radius="md">
        {log.length === 0 ? (
          <Text size="sm" c="dimmed">Nothing yet.</Text>
        ) : (
          log.map((line, index) => (
            <Text key={index} size="sm" ff="monospace">{line}</Text>
          ))
        )}
      </Paper>
    </Stack>
  );
}

export default TestPage;

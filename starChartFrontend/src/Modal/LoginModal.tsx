import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSetAtom } from "jotai";
import { Alert, Anchor, Button, Modal, PasswordInput, Stack, Text, TextInput } from "@mantine/core";

import { connectionErrorAtom, connectionStatusAtom, loadServerDataAtom, usernameAtom } from "../atoms";
import { toCommandError } from "../api/errors";

type LoginModalProps = {
  opened: boolean;
  onClose: () => void;
  onSwitchToCreate: () => void;
  initialUsername?: string;
  notice?: { color: string; message: string } | null;
};

function LoginModal({ opened, onClose, onSwitchToCreate, initialUsername = "", notice }: LoginModalProps) {
  const setUsername = useSetAtom(usernameAtom);
  const setConnectionStatus = useSetAtom(connectionStatusAtom);
  const setConnectionError = useSetAtom(connectionErrorAtom);
  const loadServerData = useSetAtom(loadServerDataAtom);

  const [username, setUsernameInput] = useState(initialUsername);
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  // Reset the form each time the modal opens
  useEffect(() => {
    if (opened) {
      setUsernameInput(initialUsername);
      setPassword("");
      setPasswordError(null);
      setFormError(null);
    }
  }, [opened, initialUsername]);

  const trimmed = username.trim();
  const canSubmit = trimmed.length > 0 && password.length > 0 && !loading;

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;

    setLoading(true);
    setPasswordError(null);
    setFormError(null);
    setConnectionStatus("connecting");
    let loggedIn = false;
    try {
      await invoke("login", { username: trimmed, password });
      loggedIn = true;
      await loadServerData();
      setUsername(trimmed);
      setConnectionStatus("connected");
      setConnectionError(null);
      setPassword("");
      onClose();
    } catch (err) {
      const error = toCommandError(err);
      setConnectionStatus("disconnected");
      if (loggedIn) {
        // Logged in but couldn't load the data: close the connection rather than show an empty app
        await invoke("logout").catch(() => {});
        setFormError(`Logged in, but couldn't load the data: ${error.message}`);
      } else if (error.kind === "bad_login") {
        setPasswordError("Wrong username or password");
      } else {
        setFormError(error.message);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      centered
      radius="md"
      title={<Text fw={800} size="lg" c="violet.9">🔑 Log in</Text>}
    >
      <form onSubmit={handleSubmit}>
        <Stack gap="sm">
          {notice && !formError && <Alert color={notice.color} variant="light">{notice.message}</Alert>}
          {formError && <Alert color="red" variant="light">{formError}</Alert>}

          <TextInput
            label="Username"
            value={username}
            onChange={(e) => setUsernameInput(e.currentTarget.value)}
            autoComplete="username"
            data-autofocus={!initialUsername || undefined}
          />
          <PasswordInput
            label="Password"
            value={password}
            onChange={(e) => {
              setPassword(e.currentTarget.value);
              setPasswordError(null);
            }}
            error={passwordError}
            autoComplete="current-password"
            data-autofocus={initialUsername ? true : undefined}
          />

          <Button type="submit" color="violet" loading={loading} disabled={!canSubmit} mt="xs">
            Log in 🔓
          </Button>

          <Text size="sm" c="dimmed" ta="center">
            No account yet?{" "}
            <Anchor component="button" type="button" size="sm" onClick={onSwitchToCreate}>
              Create one
            </Anchor>
          </Text>
        </Stack>
      </form>
    </Modal>
  );
}

export default LoginModal;

import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert, Anchor, Button, Modal, PasswordInput, Stack, Text, TextInput } from "@mantine/core";

import { toCommandError } from "../api/errors";

// Must match the server's limits (see AppError::InvalidAccount)
const USERNAME_MAX = 50;
const PASSWORD_MIN = 8;
const PASSWORD_MAX = 128;

type CreateLoginModalProps = {
  opened: boolean;
  onClose: () => void;
  onSwitchToLogin: () => void;
  /** Called with the new (trimmed) username once the account exists. */
  onCreated: (username: string) => void;
};

type FieldErrors = {
  username?: string;
  password?: string;
  confirmPassword?: string;
  adminCode?: string;
};

function validate(username: string, password: string, confirmPassword: string, adminCode: string): FieldErrors {
  const errors: FieldErrors = {};
  if (username.length === 0) errors.username = "Enter a username";
  else if (username.length > USERNAME_MAX) errors.username = `At most ${USERNAME_MAX} characters`;

  if (password.length < PASSWORD_MIN) errors.password = `At least ${PASSWORD_MIN} characters`;
  else if (password.length > PASSWORD_MAX) errors.password = `At most ${PASSWORD_MAX} characters`;

  if (confirmPassword !== password) errors.confirmPassword = "Passwords don't match";
  if (adminCode.length === 0) errors.adminCode = "Enter the admin code";
  return errors;
}

function CreateLoginModal({ opened, onClose, onSwitchToLogin, onCreated }: CreateLoginModalProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [adminCode, setAdminCode] = useState("");
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [formError, setFormError] = useState<string | null>(null);

  // Start with a clean form each time the modal opens
  useEffect(() => {
    if (opened) {
      setUsername("");
      setPassword("");
      setConfirmPassword("");
      setAdminCode("");
      setErrors({});
      setFormError(null);
    }
  }, [opened]);

  const clearError = (field: keyof FieldErrors) =>
    setErrors((prev) => (prev[field] ? { ...prev, [field]: undefined } : prev));

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (loading) return;

    const trimmed = username.trim();
    const validation = validate(trimmed, password, confirmPassword, adminCode);
    setErrors(validation);
    setFormError(null);
    if (Object.keys(validation).length > 0) return;

    setLoading(true);
    try {
      await invoke("create_login", { username: trimmed, password, adminCode });
      onCreated(trimmed);
    } catch (err) {
      const error = toCommandError(err);
      switch (error.kind) {
        case "username_taken":
          setErrors({ username: "That username is already taken" });
          break;
        case "bad_admin_code":
          setErrors({ adminCode: "Wrong admin code" });
          break;
        default:
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
      title={<Text fw={800} size="lg" c="violet.9">🆕 Create login</Text>}
    >
      <form onSubmit={handleSubmit} noValidate>
        <Stack gap="sm">
          {formError && <Alert color="red" variant="light">{formError}</Alert>}

          <TextInput
            label="Username"
            description={`Up to ${USERNAME_MAX} characters`}
            value={username}
            onChange={(e) => {
              setUsername(e.currentTarget.value);
              clearError("username");
            }}
            error={errors.username}
            autoComplete="username"
            data-autofocus
          />
          <PasswordInput
            label="Password"
            description={`${PASSWORD_MIN} to ${PASSWORD_MAX} characters`}
            value={password}
            onChange={(e) => {
              setPassword(e.currentTarget.value);
              clearError("password");
            }}
            error={errors.password}
            autoComplete="new-password"
          />
          <PasswordInput
            label="Confirm password"
            value={confirmPassword}
            onChange={(e) => {
              setConfirmPassword(e.currentTarget.value);
              clearError("confirmPassword");
            }}
            error={errors.confirmPassword}
            autoComplete="new-password"
          />
          <PasswordInput
            label="Admin code"
            description="Ask an administrator for this code"
            value={adminCode}
            onChange={(e) => {
              setAdminCode(e.currentTarget.value);
              clearError("adminCode");
            }}
            error={errors.adminCode}
            autoComplete="off"
          />

          <Button type="submit" color="violet" loading={loading} mt="xs">
            Create account ➕
          </Button>

          <Text size="sm" c="dimmed" ta="center">
            Already have an account?{" "}
            <Anchor component="button" type="button" size="sm" onClick={onSwitchToLogin}>
              Log in
            </Anchor>
          </Text>
        </Stack>
      </form>
    </Modal>
  );
}

export default CreateLoginModal;

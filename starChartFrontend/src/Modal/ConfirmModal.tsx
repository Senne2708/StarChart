import { useEffect, useState, type ReactNode } from "react";
import { Alert, Button, Group, Modal, Stack, Text } from "@mantine/core";

import { toCommandError } from "../api/errors";
import type { CommandError } from "../api/protocol";

type ConfirmModalProps = {
  opened: boolean;
  title: string;
  children: ReactNode;
  confirmLabel: string;
  /** Runs when the user confirms. The modal closes if it resolves, and shows the error if it rejects. */
  onConfirm: () => Promise<void>;
  onClose: () => void;
  /** Turns a failed `onConfirm` into the message shown. Defaults to the error's own message. */
  describeError?: (error: CommandError) => string;
};

/** A "are you sure?" dialog for actions that can't be undone */
function ConfirmModal({ opened, title, children, confirmLabel, onConfirm, onClose, describeError }: ConfirmModalProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (opened) setError(null);
  }, [opened]);

  const handleConfirm = async () => {
    setLoading(true);
    setError(null);
    try {
      await onConfirm();
      onClose();
    } catch (err) {
      const commandError = toCommandError(err);
      setError(describeError ? describeError(commandError) : commandError.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={loading ? () => {} : onClose}
      centered
      radius="md"
      title={<Text fw={800} size="lg" c="red.8">{title}</Text>}
    >
      <Stack gap="sm">
        {error && <Alert color="red" variant="light">{error}</Alert>}
        {children}
        <Group justify="flex-end" mt="xs">
          <Button variant="default" onClick={onClose} disabled={loading}>Cancel</Button>
          <Button color="red" onClick={handleConfirm} loading={loading}>{confirmLabel}</Button>
        </Group>
      </Stack>
    </Modal>
  );
}

export default ConfirmModal;

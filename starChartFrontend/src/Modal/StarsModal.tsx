import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAtomValue } from "jotai";
import { Alert, Button, Modal, NumberInput, SegmentedControl, Select, Stack, Text, TextInput } from "@mantine/core";

import { studentsAtom, usernameAtom } from "../atoms";
import { toCommandError } from "../api/errors";
import { SUBJECTS, subjectLabel } from "../api/subjects";
import type { ActivityKind, KeyStage, Subject } from "../api/protocol";

type StarsModalProps = {
  opened: boolean;
  onClose: () => void;
};

type KeyStageFilter = "all" | KeyStage;
type NegativeKind = "take_away" | "cash_in";

const KEY_STAGE_FILTERS: { label: string; value: KeyStageFilter }[] = [
  { label: "All", value: "all" },
  { label: "KS3", value: "ks3" },
  { label: "KS4", value: "ks4" },
];

const NEGATIVE_KINDS: { label: string; value: NegativeKind }[] = [
  { label: "Take away", value: "take_away" },
  { label: "Cash in", value: "cash_in" },
];

// Match the server's limits
const MAX_REASON_LENGTH = 200;
const MAX_STARS_AT_ONCE = 100;

const KIND_UI: Record<ActivityKind, { label: string; placeholder: string; button: string; color: string }> = {
  reward: { label: "Reason", placeholder: "e.g. Great homework", button: "Award ⭐", color: "violet" },
  take_away: { label: "Reason", placeholder: "e.g. Late to class", button: "Take away ➖", color: "red" },
  cash_in: { label: "Reward", placeholder: "e.g. Homework pass", button: "Cash in 🎁", color: "orange" },
};

function starsText(count: number): string {
  return `${count} ${count === 1 ? "star" : "stars"}`;
}

function StarsModal({ opened, onClose }: StarsModalProps) {
  const username = useAtomValue(usernameAtom);
  const students = useAtomValue(studentsAtom);

  const [keyStage, setKeyStage] = useState<KeyStageFilter>("all");
  const [studentId, setStudentId] = useState<string | null>(null);
  const [subject, setSubject] = useState<Subject | null>(null);
  const [stars, setStars] = useState<number | string>("");
  const [negativeKind, setNegativeKind] = useState<NegativeKind>("take_away");
  const [reason, setReason] = useState("");
  const [loading, setLoading] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  // Reset the form each time the modal opens
  useEffect(() => {
    if (opened) {
      setKeyStage("all");
      setStudentId(null);
      setSubject(null);
      setStars("");
      setNegativeKind("take_away");
      setReason("");
      setFormError(null);
    }
  }, [opened]);

  const studentOptions = useMemo(
    () =>
      students
        .filter((s) => keyStage === "all" || s.ks === keyStage)
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((s) => ({ value: String(s.id), label: s.name })),
    [students, keyStage],
  );

  // Drop the selected student if the key stage filter hides them
  useEffect(() => {
    if (studentId && !studentOptions.some((o) => o.value === studentId)) {
      setStudentId(null);
    }
  }, [studentOptions, studentId]);

  const student = students.find((s) => String(s.id) === studentId) ?? null;
  const classBalance = student && subject ? student.balances[subject] : null;

  const starDiff = typeof stars === "number" ? stars : Number.NaN;
  const isNegative = starDiff < 0;
  const kind: ActivityKind = isNegative ? negativeKind : "reward";
  const ui = KIND_UI[kind];

  // Can't take away or cash in more stars than the student has in that class
  const minStars = -Math.min(MAX_STARS_AT_ONCE, classBalance ?? MAX_STARS_AT_ONCE);
  const starsValid = Number.isInteger(starDiff) && starDiff !== 0 && starDiff >= minStars && starDiff <= MAX_STARS_AT_ONCE;
  const starsError =
    student && subject && classBalance !== null && Number.isInteger(starDiff) && starDiff < minStars
      ? `${student.name} only has ${starsText(classBalance)} in ${subjectLabel(subject)}`
      : null;
  const starsDescription =
    student && subject && classBalance !== null
      ? `${student.name} has ${starsText(classBalance)} in ${subjectLabel(subject)}`
      : "Positive to award, negative to take away or cash in";

  const trimmedReason = reason.trim();
  const canSubmit =
    student !== null && subject !== null && starsValid && trimmedReason.length > 0 && username !== null && !loading;

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit || !student || !subject) return;

    setLoading(true);
    setFormError(null);
    try {
      // The server records it under the logged-in teacher. The updated student
      // and the new activity come back as a server event.
      await invoke("add_activity", {
        studentId: student.id,
        rewardDescription: trimmedReason,
        starDiff,
        kind,
        subject,
      });
      onClose();
    } catch (err) {
      setFormError(toCommandError(err).message);
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
      title={<Text fw={800} size="lg" c="violet.9">⭐ Give stars</Text>}
    >
      <form onSubmit={handleSubmit}>
        <Stack gap="sm">
          {formError && <Alert color="red" variant="light">{formError}</Alert>}
          {!username && (
            <Alert color="orange" variant="light">Log in to give stars.</Alert>
          )}

          <SegmentedControl
            value={keyStage}
            onChange={(value) => setKeyStage(value as KeyStageFilter)}
            data={KEY_STAGE_FILTERS}
            color="violet"
            fullWidth
          />

          <Select
            label="Student"
            placeholder="Type to search"
            data={studentOptions}
            value={studentId}
            onChange={setStudentId}
            searchable
            clearable
            nothingFoundMessage="No students found"
            data-autofocus
          />

          <Select
            label="Class"
            placeholder="Pick a class"
            data={SUBJECTS}
            value={subject}
            onChange={(value) => setSubject(value as Subject | null)}
            allowDeselect={false}
          />

          <NumberInput
            label="Stars"
            description={starsDescription}
            placeholder="e.g. 3 or -2"
            value={stars}
            onChange={setStars}
            min={minStars}
            max={MAX_STARS_AT_ONCE}
            allowDecimal={false}
            allowNegative
            error={starsError}
          />

          {isNegative && (
            <SegmentedControl
              value={negativeKind}
              onChange={(value) => setNegativeKind(value as NegativeKind)}
              data={NEGATIVE_KINDS}
              color={KIND_UI[negativeKind].color}
              fullWidth
            />
          )}

          <TextInput
            label={ui.label}
            placeholder={ui.placeholder}
            value={reason}
            onChange={(e) => setReason(e.currentTarget.value)}
            maxLength={MAX_REASON_LENGTH}
          />

          <Button type="submit" color={ui.color} loading={loading} disabled={!canSubmit} mt="xs">
            {ui.button}
          </Button>
        </Stack>
      </form>
    </Modal>
  );
}

export default StarsModal;

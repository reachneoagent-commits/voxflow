import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Circle, Square } from "lucide-react";
import { toast } from "sonner";
import { commands, type MeetingStatus } from "@/bindings";
import { Button } from "../../ui/Button";
import { HelpNote } from "../../ui/HelpNote";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ShortcutInput } from "../ShortcutInput";

const IDLE: MeetingStatus = {
  recording: false,
  seconds: 0,
  system_audio: false,
};

const clock = (total: number) => {
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${two(m)}:${two(s)}` : `${two(m)}:${two(s)}`;
};

export const MeetingsSettings: React.FC = () => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MeetingStatus>(IDLE);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setStatus(await commands.meetingStatus());
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!status.recording) return;
    const timer = setInterval(() => void refresh(), 1000);
    return () => clearInterval(timer);
  }, [status.recording, refresh]);

  const start = async () => {
    setBusy(true);
    const result = await commands.startMeeting();
    setBusy(false);
    if (result.status === "ok") setStatus(result.data);
    else toast.error(result.error);
  };

  const stop = async () => {
    setBusy(true);
    setStatus(IDLE);
    const result = await commands.stopMeeting();
    setBusy(false);
    if (result.status === "ok") toast.success(t("settings.meetings.saved"));
    else toast.error(result.error);
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <HelpNote
        title={t("settings.meetings.help.title")}
        example={t("settings.meetings.help.example")}
      >
        {t("settings.meetings.help.body")}
      </HelpNote>

      <div className="flex flex-col items-center gap-4 rounded-lg border border-line px-6 py-10">
        {status.recording ? (
          <>
            <div className="flex items-center gap-2 text-sm font-medium text-ink-2">
              <Circle
                width={10}
                height={10}
                className="animate-pulse fill-red-500 text-red-500"
                aria-hidden="true"
              />
              {t("settings.meetings.recording")}
            </div>
            <div className="text-4xl font-semibold tabular-nums">
              {clock(status.seconds)}
            </div>
            <Button variant="danger" size="md" onClick={stop} disabled={busy}>
              <Square width={14} height={14} className="me-2" />
              {t("settings.meetings.stop")}
            </Button>
            {!status.system_audio && (
              <p className="text-xs text-ink-2 text-center max-w-sm">
                {t("settings.meetings.micOnly")}
              </p>
            )}
          </>
        ) : (
          <>
            <p className="text-sm text-ink-2 text-center max-w-sm">
              {t("settings.meetings.ready")}
            </p>
            <Button variant="primary" size="md" onClick={start} disabled={busy}>
              {t("settings.meetings.start")}
            </Button>
          </>
        )}
      </div>
      <SettingsGroup>
        <ShortcutInput shortcutId="toggle_meeting" grouped={true} />
      </SettingsGroup>
    </div>
  );
};

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Circle, Keyboard, Square } from "lucide-react";
import { toast } from "sonner";
import { commands, type MeetingStatus } from "@/bindings";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
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

/** Start and stop a meeting, with the shortcut one click away. */
export const MeetingRecorder: React.FC = () => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MeetingStatus>(IDLE);
  const [busy, setBusy] = useState(false);
  const [shortcutOpen, setShortcutOpen] = useState(false);

  const refresh = useCallback(async () => {
    setStatus(await commands.meetingStatus());
  }, []);

  // Always watching: a meeting can also be started from the shortcut or the
  // menu bar while this page is open.
  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 1000);
    return () => clearInterval(timer);
  }, [refresh]);

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
    <div className="space-y-2 p-3">
      {status.recording ? (
        <>
          <div className="flex items-center justify-between rounded-lg bg-wash px-3 py-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Circle
                width={10}
                height={10}
                className="animate-pulse fill-red-500 text-red-500"
                aria-hidden="true"
              />
              {t("settings.meetings.recording")}
            </div>
            <div className="text-sm font-semibold tabular-nums">
              {clock(status.seconds)}
            </div>
          </div>
          <Button
            variant="danger"
            size="md"
            className="w-full"
            onClick={stop}
            disabled={busy}
          >
            <Square width={14} height={14} className="me-2" />
            {t("settings.meetings.stop")}
          </Button>
          {!status.system_audio && (
            <p className="text-xs text-ink-2">
              {t("settings.meetings.micOnly")}
            </p>
          )}
        </>
      ) : (
        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            size="md"
            className="flex-1"
            onClick={start}
            disabled={busy}
          >
            {t("settings.meetings.start")}
          </Button>
          <Button
            variant="secondary"
            size="md"
            aria-label={t("settings.meetings.shortcut.open")}
            title={t("settings.meetings.shortcut.open")}
            onClick={() => setShortcutOpen(true)}
          >
            <Keyboard width={16} height={16} />
          </Button>
        </div>
      )}

      <Dialog
        open={shortcutOpen}
        title={t("settings.meetings.shortcut.title")}
        description={t("settings.meetings.shortcut.description")}
        closeLabel={t("common.close")}
        onOpenChange={setShortcutOpen}
      >
        <SettingsGroup>
          <ShortcutInput shortcutId="toggle_meeting" grouped={true} />
        </SettingsGroup>
      </Dialog>
    </div>
  );
};

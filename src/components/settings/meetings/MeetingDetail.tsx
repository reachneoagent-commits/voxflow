import React, { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { convertFileSrc } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { readFile } from "@tauri-apps/plugin-fs";
import { Copy, Download, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { commands, type HistoryEntry } from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { useTranscriptionJobsStore } from "@/stores/transcriptionJobsStore";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { ConfirmDelete } from "../../ui/ConfirmDelete";
import { copyToClipboard } from "../history/clipboard";
import { meetingMarkdown, parseMeeting } from "./meetingText";

interface MeetingDetailProps {
  meeting: HistoryEntry;
  onDeleted: () => void;
}

/** One meeting: the full transcript, the recording, and ways to take it away. */
export const MeetingDetail: React.FC<MeetingDetailProps> = ({
  meeting,
  onDeleted,
}) => {
  const { t, i18n } = useTranslation();
  const osType = useOsType();
  const job = useTranscriptionJobsStore((state) => state.jobs[meeting.id]);
  const [confirming, setConfirming] = useState(false);

  const when = new Date(meeting.timestamp * 1000);
  const heading = new Intl.DateTimeFormat(i18n.language, {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).format(when);
  const time = new Intl.DateTimeFormat(i18n.language, {
    hour: "numeric",
    minute: "2-digit",
  }).format(when);

  const turns = parseMeeting(meeting.transcription_text);
  const labels = {
    you: t("settings.meetings.detail.you"),
    others: t("settings.meetings.detail.others"),
  };
  const document = () => meetingMarkdown(`${heading}, ${time}`, turns, labels);

  const getAudioUrl = useCallback(async () => {
    const result = await commands.getAudioFilePath(meeting.file_name);
    if (result.status !== "ok") return null;
    if (osType === "linux") {
      const data = await readFile(result.data);
      return URL.createObjectURL(new Blob([data], { type: "audio/wav" }));
    }
    return convertFileSrc(result.data, "asset");
  }, [meeting.file_name, osType]);

  const copyAll = async () => {
    if (await copyToClipboard(document())) {
      toast.success(t("settings.meetings.detail.copied"));
    } else {
      toast.error(t("settings.history.copyError"));
    }
  };

  const saveFile = async () => {
    const path = await save({
      defaultPath: `${meeting.source_name?.replace(/\.wav$/i, "") ?? "Meeting"}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!path) return;
    const result = await commands.saveMeetingFile(path, document());
    if (result.status === "ok") {
      toast.success(t("settings.meetings.detail.saved"));
    } else {
      toast.error(result.error);
    }
  };

  const remove = async () => {
    setConfirming(false);
    const result = await commands.deleteHistoryEntry(meeting.id);
    if (result.status === "ok") onDeleted();
    else toast.error(t("settings.history.deleteError"));
  };

  return (
    <article className="mx-auto max-w-2xl space-y-5 px-8 py-6 select-text">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold">{heading}</h2>
          <p className="text-sm text-ink-2">{time}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={copyAll}
            disabled={!turns.length}
          >
            <Copy width={14} height={14} className="me-2" />
            {t("settings.meetings.detail.copy")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={saveFile}
            disabled={!turns.length}
          >
            <Download width={14} height={14} className="me-2" />
            {t("settings.meetings.detail.save")}
          </Button>
          <Button
            variant="danger-ghost"
            size="sm"
            aria-label={t("common.delete")}
            onClick={() => setConfirming(true)}
          >
            <Trash2 width={16} height={16} />
          </Button>
        </div>
      </header>

      <AudioPlayerGroup>
        <AudioPlayer onLoadRequest={getAudioUrl} className="w-full" />
      </AudioPlayerGroup>

      <div className="space-y-5">
        {job ? (
          <p className="text-sm text-ink-2">
            {job.state === "running"
              ? t("settings.meetings.list.transcribing", {
                  percent: job.percent,
                })
              : t("settings.meetings.list.waiting")}
          </p>
        ) : turns.length === 0 ? (
          <p className="text-sm text-ink-2">
            {t("settings.meetings.list.noText")}
          </p>
        ) : (
          turns.map((turn, index) => (
            <div key={index} className="space-y-1">
              {turn.speaker && (
                <div
                  className={`text-xs font-semibold uppercase tracking-wide ${
                    turn.speaker === "You" ? "text-logo-primary" : "text-ink-3"
                  }`}
                >
                  {turn.speaker === "You" ? labels.you : labels.others}
                </div>
              )}
              <p className="whitespace-pre-wrap text-[15px] leading-relaxed">
                {turn.text}
              </p>
            </div>
          ))
        )}
      </div>

      <ConfirmDelete
        name={confirming ? `${heading}, ${time}` : null}
        onCancel={() => setConfirming(false)}
        onConfirm={remove}
      />
    </article>
  );
};

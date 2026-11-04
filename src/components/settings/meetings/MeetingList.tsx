import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import type { HistoryEntry } from "@/bindings";
import { useTranscriptionJobsStore } from "@/stores/transcriptionJobsStore";
import { Input } from "../../ui/Input";
import { meetingPreview, parseMeeting } from "./meetingText";

interface MeetingListProps {
  meetings: HistoryEntry[] | null;
  selectedId: number | null;
  onSelect: (id: number) => void;
}

const whenOf = (meeting: HistoryEntry) => new Date(meeting.timestamp * 1000);

/** Meetings grouped by month, newest first, with a search box. */
export const MeetingList: React.FC<MeetingListProps> = ({
  meetings,
  selectedId,
  onSelect,
}) => {
  const { t, i18n } = useTranslation();
  const jobs = useTranscriptionJobsStore((state) => state.jobs);
  const [query, setQuery] = useState("");

  const groups = useMemo(() => {
    const q = query.trim().toLowerCase();
    const shown = (meetings ?? []).filter(
      (m) =>
        !q ||
        m.transcription_text.toLowerCase().includes(q) ||
        m.title.toLowerCase().includes(q),
    );
    const month = new Intl.DateTimeFormat(i18n.language, {
      month: "long",
      year: "numeric",
    });
    const byMonth = new Map<string, HistoryEntry[]>();
    for (const meeting of shown) {
      const label = month.format(whenOf(meeting));
      byMonth.set(label, [...(byMonth.get(label) ?? []), meeting]);
    }
    return [...byMonth.entries()];
  }, [meetings, query, i18n.language]);

  const day = new Intl.DateTimeFormat(i18n.language, {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
  const clock = new Intl.DateTimeFormat(i18n.language, {
    hour: "numeric",
    minute: "2-digit",
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="relative px-3 pb-2">
        <Search
          width={14}
          height={14}
          className="pointer-events-none absolute start-5 top-1/2 -translate-y-1/2 text-ink-3"
          aria-hidden="true"
        />
        <Input
          className="w-full ps-8"
          value={query}
          placeholder={t("settings.meetings.list.search")}
          aria-label={t("settings.meetings.list.search")}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {meetings === null ? null : groups.length === 0 ? (
          <p className="px-4 py-6 text-center text-sm text-ink-2">
            {meetings.length === 0
              ? t("settings.meetings.list.empty")
              : t("settings.meetings.list.noMatches")}
          </p>
        ) : (
          groups.map(([label, items]) => (
            <section key={label}>
              <h3 className="sticky top-0 bg-background px-4 py-1.5 text-xs font-semibold uppercase tracking-wide text-ink-3">
                {label}
              </h3>
              {items.map((meeting) => {
                const job = jobs[meeting.id];
                const preview = meetingPreview(
                  parseMeeting(meeting.transcription_text),
                  140,
                );
                const selected = meeting.id === selectedId;
                return (
                  <button
                    key={meeting.id}
                    type="button"
                    aria-current={selected ? "true" : undefined}
                    onClick={() => onSelect(meeting.id)}
                    className={`block w-full cursor-pointer border-s-2 px-4 py-2.5 text-start transition-colors focus:outline-none focus-visible:bg-wash ${
                      selected
                        ? "border-logo-primary bg-logo-primary/10"
                        : "border-transparent hover:bg-wash"
                    }`}
                  >
                    <div className="text-sm font-medium">
                      {day.format(whenOf(meeting))} ·{" "}
                      {clock.format(whenOf(meeting))}
                    </div>
                    <div className="line-clamp-2 text-xs text-ink-2">
                      {job
                        ? job.state === "running"
                          ? t("settings.meetings.list.transcribing", {
                              percent: job.percent,
                            })
                          : t("settings.meetings.list.waiting")
                        : preview || t("settings.meetings.list.noText")}
                    </div>
                  </button>
                );
              })}
            </section>
          ))
        )}
      </div>
    </div>
  );
};

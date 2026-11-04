import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Users } from "lucide-react";
import { MeetingRecorder } from "./MeetingRecorder";
import { MeetingList } from "./MeetingList";
import { MeetingDetail } from "./MeetingDetail";
import { useMeetings } from "./useMeetings";

/** Meeting notes like an inbox: the meetings on the left, the one you pick
 *  on the right. Recording starts from the top of the list. */
export const MeetingsSettings: React.FC = () => {
  const { t } = useTranslation();
  const { meetings } = useMeetings();
  const [selectedId, setSelectedId] = useState<number | null>(null);

  // Open the newest meeting first, and move on when the open one goes away.
  const selected = meetings?.find((m) => m.id === selectedId) ?? null;
  useEffect(() => {
    if (meetings && !selected) setSelectedId(meetings[0]?.id ?? null);
  }, [meetings, selected]);

  return (
    <div className="flex min-h-0 w-full max-w-5xl flex-1 overflow-hidden rounded-lg border border-line">
      <aside className="flex w-72 shrink-0 flex-col border-e border-line">
        <MeetingRecorder />
        <MeetingList
          meetings={meetings}
          selectedId={selected?.id ?? null}
          onSelect={setSelectedId}
        />
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto">
        {selected ? (
          <MeetingDetail
            key={selected.id}
            meeting={selected}
            onDeleted={() => setSelectedId(null)}
          />
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-3 px-8 text-center text-ink-2">
            <Users width={28} height={28} aria-hidden="true" />
            <p className="max-w-sm text-sm">
              {t("settings.meetings.help.body")}
            </p>
          </div>
        )}
      </main>
    </div>
  );
};

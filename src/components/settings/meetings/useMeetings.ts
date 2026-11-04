import { useCallback, useEffect, useState } from "react";
import { commands, events, type HistoryEntry } from "@/bindings";

/** Recorded meetings, newest first, kept current as they are added and
 *  transcribed. */
export const useMeetings = () => {
  const [meetings, setMeetings] = useState<HistoryEntry[] | null>(null);

  const refresh = useCallback(async () => {
    const result = await commands.getMeetings();
    if (result.status === "ok") setMeetings(result.data);
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = events.historyUpdatePayload.listen(() => void refresh());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  return { meetings, refresh };
};

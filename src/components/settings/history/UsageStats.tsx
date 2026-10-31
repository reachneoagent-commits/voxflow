import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, events, type UsageStats as Stats } from "@/bindings";

const formatMinutes = (minutes: number) =>
  minutes >= 60
    ? `${Math.floor(minutes / 60)}h ${minutes % 60}m`
    : `${minutes}m`;

/** Lifetime dictation totals, refreshed whenever a dictation is added. */
export const UsageStats: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [stats, setStats] = useState<Stats | null>(null);

  const load = useCallback(async () => {
    try {
      const result = await commands.getUsageStats();
      if (result.status === "ok") setStats(result.data);
    } catch (error) {
      console.error("Failed to load usage stats:", error);
    }
  }, []);

  useEffect(() => {
    load();
    const unlisten = events.historyUpdatePayload.listen((event) => {
      if (event.payload.action === "added") load();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [load]);

  if (!stats) return null;

  const number = (n: number) => n.toLocaleString(i18n.language);
  const items = [
    {
      value: number(stats.words_this_week),
      label: t("settings.history.stats.wordsThisWeek"),
    },
    { value: number(stats.avg_wpm), label: t("settings.history.stats.avgWpm") },
    {
      value: formatMinutes(stats.minutes_saved),
      label: t("settings.history.stats.timeSaved"),
    },
  ];

  return (
    <div className="grid grid-cols-3 gap-3">
      {items.map((item) => (
        <div
          key={item.label}
          className="rounded-lg border border-line px-4 py-3"
        >
          <div className="text-xl font-semibold tracking-tight">
            {item.value}
          </div>
          <div className="text-xs text-ink-2">{item.label}</div>
        </div>
      ))}
    </div>
  );
};

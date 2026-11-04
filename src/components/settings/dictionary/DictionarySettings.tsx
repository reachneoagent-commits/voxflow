import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import { toast } from "sonner";
import type { Replacement } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { HelpNote } from "../../ui/HelpNote";

// A fixed fallback so a missing setting never changes identity between renders.
const NONE: never[] = [];

const MAX_PHRASE = 100;

type Tab = "all" | "words" | "replacements";

const clean = (value: string) =>
  value.replace(/[<>"]/g, "").replace(/\s+/g, " ").trim();

export const DictionarySettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const words = getSetting("custom_words") ?? NONE;
  const replacements = getSetting("replacements") ?? NONE;
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [tab, setTab] = useState<Tab>("all");

  const add = () => {
    const phrase = clean(from);
    const replacement = to.trim();
    if (!/[\p{L}\p{N}]/u.test(phrase)) {
      toast.error(t("settings.dictionary.errors.required"));
      return;
    }
    const lower = phrase.toLowerCase();
    if (replacement) {
      if (replacements.some((r) => r.from.toLowerCase() === lower)) {
        toast.error(
          t("settings.dictionary.errors.duplicate", { word: phrase }),
        );
        return;
      }
      const next: Replacement = {
        id: crypto.randomUUID(),
        from: phrase,
        to: replacement,
        enabled: true,
      };
      updateSetting("replacements", [...replacements, next]);
    } else {
      if (words.some((w) => w.toLowerCase() === lower)) {
        toast.error(
          t("settings.dictionary.errors.duplicate", { word: phrase }),
        );
        return;
      }
      updateSetting("custom_words", [...words, phrase]);
    }
    setFrom("");
    setTo("");
  };

  const toggle = (id: string, enabled: boolean) =>
    updateSetting(
      "replacements",
      replacements.map((r) => (r.id === id ? { ...r, enabled } : r)),
    );

  const tabs: { id: Tab; label: string }[] = [
    { id: "all", label: t("settings.dictionary.tabs.all") },
    { id: "words", label: t("settings.dictionary.tabs.words") },
    { id: "replacements", label: t("settings.dictionary.tabs.replacements") },
  ];
  const showWords = tab !== "replacements";
  const showReplacements = tab !== "words";
  const empty =
    (!showWords || words.length === 0) &&
    (!showReplacements || replacements.length === 0);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <HelpNote
        title={t("settings.dictionary.help.title")}
        example={t("settings.dictionary.help.example")}
      >
        {t("settings.dictionary.help.body")}
      </HelpNote>
      <div className="flex items-center gap-2">
        <Input
          className="flex-1"
          value={from}
          maxLength={MAX_PHRASE}
          placeholder={t("settings.dictionary.add.word")}
          aria-label={t("settings.dictionary.add.word")}
          onChange={(e) => setFrom(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Input
          className="flex-1"
          value={to}
          maxLength={MAX_PHRASE}
          placeholder={t("settings.dictionary.add.replacement")}
          aria-label={t("settings.dictionary.add.replacement")}
          onChange={(e) => setTo(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Button
          variant="primary"
          size="md"
          onClick={add}
          disabled={!from.trim()}
        >
          {t("settings.dictionary.add.button")}
        </Button>
      </div>

      <div className="flex gap-1" role="tablist">
        {tabs.map((item) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={tab === item.id}
            onClick={() => setTab(item.id)}
            className={`rounded-lg px-3 py-1 text-sm font-medium cursor-pointer transition-colors ${
              tab === item.id
                ? "bg-logo-primary/20"
                : "text-ink-2 hover:bg-wash"
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>

      <SettingsGroup
        title={t("settings.dictionary.title")}
        description={t("settings.dictionary.description")}
      >
        {empty && (
          <p className="px-4 py-6 text-sm text-ink-2 text-center">
            {t("settings.dictionary.empty")}
          </p>
        )}
        {showWords &&
          words.map((word) => (
            <div
              key={`w-${word}`}
              className="flex items-center justify-between gap-3 px-4 py-2 min-h-12"
            >
              <h3 className="text-sm font-medium truncate">{word}</h3>
              <div className="flex items-center gap-2 shrink-0">
                <span className="rounded-full bg-logo-primary/20 px-2 py-0.5 text-xs font-medium">
                  {t("settings.dictionary.kind.word")}
                </span>
                <Button
                  variant="danger-ghost"
                  size="sm"
                  aria-label={t("common.delete")}
                  onClick={() =>
                    updateSetting(
                      "custom_words",
                      words.filter((w) => w !== word),
                    )
                  }
                >
                  <Trash2 width={16} height={16} />
                </Button>
              </div>
            </div>
          ))}
        {showReplacements &&
          replacements.map((r) => (
            <div
              key={r.id}
              className="flex items-center justify-between gap-3 px-4 py-2 min-h-12"
            >
              <h3
                className={`text-sm font-medium truncate ${r.enabled === false ? "opacity-50" : ""}`}
              >
                {r.from}
                <span className="mx-2 text-ink-3" aria-hidden="true">
                  →
                </span>
                {r.to}
              </h3>
              <div className="flex items-center gap-2 shrink-0">
                <span className="rounded-full bg-logo-primary/20 px-2 py-0.5 text-xs font-medium">
                  {t("settings.dictionary.kind.replacement")}
                </span>
                <Button
                  variant="danger-ghost"
                  size="sm"
                  aria-label={t("common.delete")}
                  onClick={() =>
                    updateSetting(
                      "replacements",
                      replacements.filter((x) => x.id !== r.id),
                    )
                  }
                >
                  <Trash2 width={16} height={16} />
                </Button>
                <Switch
                  checked={r.enabled ?? true}
                  ariaLabel={r.from}
                  onChange={(enabled) => toggle(r.id, enabled)}
                />
              </div>
            </div>
          ))}
      </SettingsGroup>
    </div>
  );
};

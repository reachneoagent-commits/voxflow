import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { toast } from "sonner";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";

/** Mirrors the backend defaults (settings.rs). */
const DEFAULT_PHRASES = [
  "paste clipboard",
  "paste link",
  "paste url",
  "insert link",
  "insert clipboard",
  "past clipboard",
  "past link",
];

/** Say one of these phrases while dictating to type whatever you've copied. */
export const ClipboardPhrase: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("clipboard_phrase_enabled") ?? true;
  const phrases = getSetting("clipboard_phrases") ?? DEFAULT_PHRASES;
  const [draft, setDraft] = useState("");

  const add = () => {
    const next = draft.trim().replace(/\s+/g, " ");
    if (!next) return;
    if (!/[\p{L}\p{N}]/u.test(next)) {
      toast.error(t("settings.snippets.clipboard.phrases.error"));
      return;
    }
    if (phrases.some((p) => p.toLowerCase() === next.toLowerCase())) {
      toast.error(
        t("settings.snippets.clipboard.phrases.duplicate", { phrase: next }),
      );
      return;
    }
    updateSetting("clipboard_phrases", [...phrases, next]);
    setDraft("");
  };

  return (
    <SettingsGroup
      title={t("settings.snippets.clipboard.title")}
      description={t("settings.snippets.clipboard.description")}
    >
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("clipboard_phrase_enabled", value)}
        isUpdating={isUpdating("clipboard_phrase_enabled")}
        label={t("settings.snippets.clipboard.toggle.label")}
        description={t("settings.snippets.clipboard.toggle.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <div className={`px-4 py-3 space-y-3 ${enabled ? "" : "opacity-50"}`}>
        <div>
          <h3 className="text-sm font-medium">
            {t("settings.snippets.clipboard.phrases.label")}
          </h3>
          <p className="text-xs text-ink-2">
            {t("settings.snippets.clipboard.phrases.description")}
          </p>
        </div>
        <ul className="flex flex-wrap gap-2">
          {phrases.map((phrase) => (
            <li
              key={phrase}
              className="flex items-center gap-1 rounded-full bg-logo-primary/20 py-1 ps-3 pe-1 text-sm"
            >
              {phrase}
              <button
                type="button"
                disabled={!enabled}
                aria-label={t("common.delete")}
                onClick={() =>
                  updateSetting(
                    "clipboard_phrases",
                    phrases.filter((p) => p !== phrase),
                  )
                }
                className="grid size-5 place-items-center rounded-full text-ink-2 hover:bg-wash hover:text-text cursor-pointer disabled:cursor-default"
              >
                <X width={14} height={14} />
              </button>
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-2">
          <Input
            className="flex-1"
            value={draft}
            maxLength={50}
            disabled={!enabled}
            placeholder={t("settings.snippets.clipboard.phrases.placeholder")}
            aria-label={t("settings.snippets.clipboard.phrases.placeholder")}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && add()}
          />
          <Button
            variant="primary"
            size="md"
            onClick={add}
            disabled={!enabled || !draft.trim()}
          >
            {t("settings.snippets.clipboard.phrases.add")}
          </Button>
          <Button
            variant="outline"
            size="md"
            disabled={!enabled}
            onClick={() => updateSetting("clipboard_phrases", DEFAULT_PHRASES)}
          >
            {t("settings.snippets.clipboard.phrases.reset")}
          </Button>
        </div>
      </div>
    </SettingsGroup>
  );
};

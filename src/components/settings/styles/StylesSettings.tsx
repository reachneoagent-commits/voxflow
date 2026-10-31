import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import { toast } from "sonner";
import {
  commands,
  type ActiveApp,
  type AppStyle,
  type WritingStyle,
} from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { HelpNote } from "../../ui/HelpNote";

const STYLES: WritingStyle[] = ["formal", "casual", "very_casual", "technical"];

const styleKey = (style: WritingStyle) =>
  style === "very_casual" ? "veryCasual" : style;

const selectClass =
  "px-2 py-1 text-sm font-medium bg-wash border border-line-strong rounded-md hover:border-logo-primary focus:outline-none focus:border-logo-primary cursor-pointer";

const StyleSelect: React.FC<{
  value: WritingStyle;
  label: string;
  onChange: (style: WritingStyle) => void;
}> = ({ value, label, onChange }) => {
  const { t } = useTranslation();
  return (
    <select
      className={selectClass}
      aria-label={label}
      value={value}
      onChange={(e) => onChange(e.target.value as WritingStyle)}
    >
      {STYLES.map((style) => (
        <option key={style} value={style}>
          {t(`settings.styles.options.${styleKey(style)}.name`)}
        </option>
      ))}
    </select>
  );
};

export const StylesSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const defaultStyle = getSetting("default_style") ?? "casual";
  const appStyles = useMemo(() => getSetting("app_styles") ?? [], [getSetting]);
  const [adding, setAdding] = useState(false);
  const [running, setRunning] = useState<ActiveApp[]>([]);
  const [pick, setPick] = useState("");
  const [manualId, setManualId] = useState("");

  const openAdd = async () => {
    setAdding(true);
    try {
      setRunning(await commands.listRunningApps());
    } catch (error) {
      console.error("Failed to list running apps:", error);
    }
  };

  const available = running.filter(
    (app) =>
      !appStyles.some((s) => s.app_id.toLowerCase() === app.id.toLowerCase()),
  );

  const addApp = () => {
    const picked = available.find((app) => app.id === pick);
    const app: ActiveApp | undefined =
      picked ??
      (manualId.trim()
        ? { id: manualId.trim(), name: manualId.trim() }
        : undefined);
    if (!app) return;
    if (
      appStyles.some((s) => s.app_id.toLowerCase() === app.id.toLowerCase())
    ) {
      toast.error(t("settings.styles.errors.duplicate", { app: app.name }));
      return;
    }
    const next: AppStyle = {
      id: crypto.randomUUID(),
      app_id: app.id,
      app_name: app.name,
      style: defaultStyle,
    };
    updateSetting("app_styles", [...appStyles, next]);
    setAdding(false);
    setPick("");
    setManualId("");
  };

  const setAppStyle = (id: string, style: WritingStyle) =>
    updateSetting(
      "app_styles",
      appStyles.map((s) => (s.id === id ? { ...s, style } : s)),
    );

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <HelpNote
        title={t("settings.styles.help.title")}
        example={t("settings.styles.help.example")}
      >
        {t("settings.styles.help.body")}
      </HelpNote>
      <SettingsGroup
        title={t("settings.styles.title")}
        description={t("settings.styles.description")}
      >
        <div
          className="grid grid-cols-2 gap-3 p-4"
          role="radiogroup"
          aria-label={t("settings.styles.title")}
        >
          {STYLES.map((style) => {
            const picked = defaultStyle === style;
            const key = styleKey(style);
            return (
              <button
                key={style}
                type="button"
                role="radio"
                aria-checked={picked}
                onClick={() => updateSetting("default_style", style)}
                className={`rounded-xl border p-3 text-start cursor-pointer transition-colors ${
                  picked
                    ? "border-logo-primary bg-logo-primary/20"
                    : "border-line hover:border-line-strong"
                }`}
              >
                <b className="block text-sm font-semibold">
                  {t(`settings.styles.options.${key}.name`)}
                </b>
                <span className="block text-xs text-ink-2 mt-1">
                  {t(`settings.styles.options.${key}.description`)}
                </span>
                <span className="block text-xs mt-2 italic">
                  {t(`settings.styles.options.${key}.example`)}
                </span>
              </button>
            );
          })}
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.styles.apps.title")}
        description={t("settings.styles.apps.description")}
      >
        {appStyles.length === 0 && !adding && (
          <p className="px-4 py-6 text-sm text-ink-2 text-center">
            {t("settings.styles.apps.empty")}
          </p>
        )}
        {appStyles.map((s) => (
          <div
            key={s.id}
            className="flex items-center justify-between gap-3 px-4 py-2 min-h-12"
          >
            <h3 className="text-sm font-medium truncate">{s.app_name}</h3>
            <div className="flex items-center gap-2 shrink-0">
              <StyleSelect
                value={s.style}
                label={s.app_name}
                onChange={(style) => setAppStyle(s.id, style)}
              />
              <Button
                variant="danger-ghost"
                size="sm"
                aria-label={t("common.delete")}
                onClick={() =>
                  updateSetting(
                    "app_styles",
                    appStyles.filter((x) => x.id !== s.id),
                  )
                }
              >
                <Trash2 width={16} height={16} />
              </Button>
            </div>
          </div>
        ))}
        {adding ? (
          <div className="flex flex-wrap items-center gap-2 px-4 py-3">
            {available.length > 0 ? (
              <select
                className={`${selectClass} flex-1 min-w-48`}
                aria-label={t("settings.styles.apps.choose")}
                value={pick}
                onChange={(e) => setPick(e.target.value)}
              >
                <option value="">{t("settings.styles.apps.choose")}</option>
                {available.map((app) => (
                  <option key={app.id} value={app.id}>
                    {app.name}
                  </option>
                ))}
              </select>
            ) : (
              <Input
                className="flex-1 min-w-48"
                value={manualId}
                placeholder={t("settings.styles.apps.manual")}
                aria-label={t("settings.styles.apps.manual")}
                onChange={(e) => setManualId(e.target.value)}
              />
            )}
            <Button
              variant="primary"
              size="md"
              onClick={addApp}
              disabled={!pick && !manualId.trim()}
            >
              {t("settings.styles.apps.add")}
            </Button>
            <Button
              variant="outline"
              size="md"
              onClick={() => setAdding(false)}
            >
              {t("common.cancel")}
            </Button>
          </div>
        ) : (
          <div className="px-4 py-3">
            <Button variant="secondary" size="md" onClick={openAdd}>
              {t("settings.styles.apps.addApp")}
            </Button>
          </div>
        )}
      </SettingsGroup>
    </div>
  );
};

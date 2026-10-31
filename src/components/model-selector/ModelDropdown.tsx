import React from "react";
import { useTranslation } from "react-i18next";
import type { ModelInfo } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { useModelCatalog } from "@/hooks/useModelCatalog";
import { suggestedLanguages } from "@/lib/utils/languageSuggestions";
import { toTableModel } from "@/components/shared/modelTable/tableModel";
import { GradeChip } from "@/components/shared/GradeChip";
import { SpeedBars, speedLevel } from "@/components/shared/SpeedBars";
import {
  getTranslatedModelName,
  getTranslatedModelDescription,
} from "../../lib/utils/modelTranslation";

interface ModelDropdownProps {
  models: ModelInfo[];
  currentModelId: string;
  // A file is transcribing: show the models, but don't let them be picked.
  locked?: boolean;
  onModelSelect: (modelId: string) => void;
}

const ModelDropdown: React.FC<ModelDropdownProps> = ({
  models,
  currentModelId,
  locked = false,
  onModelSelect,
}) => {
  const { t } = useTranslation();
  const downloadedModels = models.filter((m) => m.is_downloaded);
  const { getSetting } = useSettings();
  const spoken = getSetting("spoken_languages");
  const { grading } = useModelCatalog(
    spoken?.length ? spoken : suggestedLanguages(),
  );

  const handleModelClick = (modelId: string) => {
    if (locked) return;
    onModelSelect(modelId);
  };

  return (
    <div className="absolute bottom-full start-0 mb-2 w-80 max-h-[60vh] overflow-y-auto bg-background border border-line rounded-lg shadow-lg py-2 z-50">
      {locked && (
        <div className="px-3 pb-2 mb-1 text-xs text-ink-2 border-b border-line">
          {t("modelSelector.lockedWhileTranscribing")}
        </div>
      )}
      {downloadedModels.length > 0 ? (
        <div>
          <div
            className="flex items-center justify-end px-3 pb-1 text-[10px] font-semibold uppercase tracking-wide text-ink-3"
            aria-hidden="true"
          >
            <span className="w-14 text-center">
              {t("modelSelector.accuracy")}
            </span>
            <span className="w-12 text-center">{t("modelSelector.speed")}</span>
          </div>
          {downloadedModels.map((model) => {
            const catalogModel = toTableModel(model, t).model;
            const grade = grading.overall(catalogModel);
            const speed = speedLevel(grading.wait(catalogModel));
            return (
              <div
                key={model.id}
                onClick={() => handleModelClick(model.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleModelClick(model.id);
                  }
                }}
                tabIndex={locked ? -1 : 0}
                role="button"
                aria-disabled={locked}
                className={`w-full px-3 py-2 text-start transition-colors focus:outline-none ${
                  locked
                    ? "opacity-50 cursor-not-allowed"
                    : "hover:bg-wash cursor-pointer"
                } ${
                  currentModelId === model.id
                    ? "bg-logo-primary/10 text-logo-primary"
                    : ""
                }`}
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-sm text-text">
                      {getTranslatedModelName(model, t)}
                      {model.is_custom && (
                        <span className="ms-1.5 text-[10px] font-medium text-ink-3 uppercase">
                          {t("modelSelector.custom")}
                        </span>
                      )}
                      {currentModelId === model.id && (
                        <span className="ms-1.5 text-[10px] font-medium text-logo-primary uppercase">
                          {t("modelSelector.active")}
                        </span>
                      )}
                      {model.supports_streaming && (
                        <span className="ms-1.5 text-[10px] font-medium text-logo-primary uppercase">
                          {t("modelSelector.streaming")}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-ink-3 italic pe-4">
                      {getTranslatedModelDescription(model, t)}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center">
                    <span
                      className="flex w-14 justify-center"
                      title={`${t("modelSelector.accuracy")}: ${grade.label}${
                        grade.letter !== "none"
                          ? ` — ${t(`modelTable.rubric.meaning.${grade.letter}`)}`
                          : ""
                      }`}
                    >
                      <GradeChip
                        letter={grade.letter}
                        label={grade.label}
                        dense
                      />
                    </span>
                    <span
                      className="flex w-12 justify-center"
                      title={t("modelSelector.speed")}
                      aria-label={t("modelSelector.speed")}
                    >
                      <SpeedBars level={speed} />
                    </span>
                  </div>
                </div>
              </div>
            );
          })}
          <p className="mt-1 border-t border-line px-3 pt-2 text-xs text-ink-2">
            {t("modelSelector.legend")}
          </p>
        </div>
      ) : (
        <div className="px-3 py-2 text-sm text-ink-2">
          {t("modelSelector.noModelsAvailable")}
        </div>
      )}
    </div>
  );
};

export default ModelDropdown;

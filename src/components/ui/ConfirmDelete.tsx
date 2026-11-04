import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./Button";
import { Dialog } from "./Dialog";

interface ConfirmDeleteProps {
  // What is about to be deleted; null keeps the dialog closed.
  name: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}

/** Asks before anything the user typed or saved is thrown away. */
export const ConfirmDelete: React.FC<ConfirmDeleteProps> = ({
  name,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();
  return (
    <Dialog
      open={name !== null}
      title={t("common.confirmDelete.title")}
      description={t("common.confirmDelete.body", { name: name ?? "" })}
      closeLabel={t("common.close")}
      onOpenChange={(open) => !open && onCancel()}
      className="max-w-sm"
      contentClassName="hidden"
      footer={
        <>
          <Button variant="secondary" size="md" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button variant="danger" size="md" onClick={onConfirm}>
            {t("common.delete")}
          </Button>
        </>
      }
    >
      {null}
    </Dialog>
  );
};

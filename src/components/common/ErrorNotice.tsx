import { ReactNode, useState } from "react";
import { useTranslation } from "../../i18n";
import { copyText } from "../../utils/clipboard";
import { Button } from "./Controls";

export interface ErrorNoticeProps {
  title?: ReactNode;
  error?: any;
  onRetry?: () => void;
}

export default function ErrorNotice({ title, error, onRetry }: ErrorNoticeProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  if (!error) return null;
  const text =
    typeof error === "string" ? error : error.message || String(error);
  return (
    <div className="ui-error" role="alert">
      {title && <strong>{title}</strong>}
      <details>
        <summary>{t("ui.details")}</summary>
        <pre>{text}</pre>
        <Button
          icon="copy"
          onClick={async () => setCopied(await copyText(text))}
        >
          {t(copied ? "ui.copied" : "ui.copy")}
        </Button>
      </details>
      {onRetry && <Button onClick={onRetry}>{t("common.retry")}</Button>}
    </div>
  );
}

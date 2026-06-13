import { useState } from "react";
import { CheckCircle, XCircle, ChevronDown, ChevronUp } from "lucide-react";
import type { InstallTaskResult } from "@skills-pp/shared";

interface Props {
  task: InstallTaskResult;
}

export function InstallLogPanel({ task }: Props) {
  const [expanded, setExpanded] = useState(false);

  const isSuccess = task.status === "success";
  const haslogs = task.logLines && task.logLines.length > 0;

  return (
    <div
      className={`rounded-[var(--radius-lg)] border p-4 ${
        isSuccess
          ? "border-[var(--color-success)]/20 bg-[var(--color-success-subtle)]"
          : "border-[var(--color-danger)]/20 bg-[var(--color-danger-subtle)]"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2.5">
          {isSuccess ? (
            <CheckCircle className="h-4 w-4 shrink-0 text-[var(--color-success)]" />
          ) : (
            <XCircle className="h-4 w-4 shrink-0 text-[var(--color-danger)]" />
          )}
          <div>
            <p
              className={`text-[13px] font-medium ${
                isSuccess ? "text-[var(--color-success)]" : "text-[var(--color-danger)]"
              }`}
            >
              {isSuccess
                ? `「${task.skillName}」安装成功`
                : `「${task.skillName}」安装失败`}
            </p>
            {!isSuccess && task.errorMessage && (
              <p className="mt-0.5 text-[11px] text-[var(--color-danger)] opacity-80">
                {task.errorMessage}
              </p>
            )}
          </div>
        </div>

        {haslogs && (
          <button
            onClick={() => setExpanded((v) => !v)}
            className={`flex shrink-0 items-center gap-1 text-[11px] ${
              isSuccess
                ? "text-[var(--color-success)]/70 hover:text-[var(--color-success)]"
                : "text-[var(--color-danger)]/70 hover:text-[var(--color-danger)]"
            }`}
          >
            {expanded ? (
              <>
                收起 <ChevronUp className="h-3 w-3" />
              </>
            ) : (
              <>
                日志 <ChevronDown className="h-3 w-3" />
              </>
            )}
          </button>
        )}
      </div>

      {expanded && haslogs && (
        <pre className="mt-3 max-h-48 overflow-auto rounded-[var(--radius-md)] bg-black/30 p-3 font-mono text-[11px] leading-relaxed text-[var(--color-text-secondary)]">
          {task.logLines.join("\n")}
        </pre>
      )}
    </div>
  );
}
